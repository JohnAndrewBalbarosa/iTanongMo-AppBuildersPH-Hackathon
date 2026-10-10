"use strict";
const assert = require("node:assert/strict");
const path = require("node:path");
const { execFileSync } = require("node:child_process");
const { performance } = require("node:perf_hooks");
const { setTimeout: pause } = require("node:timers/promises");
const { login, request, startApi } = require("./local-api.cjs");
const { summarizeRequests, requireLoopback } = require("./metrics.cjs");

function databaseFixture(api, action, destination) {
  return JSON.parse(execFileSync("python", [path.join(__dirname, "db-fixture.py"), action,
    api.database, ...(destination ? [destination] : [])], { windowsHide: true, encoding: "utf8" }));
}

function processResources(pid) {
  const command = `$p=Get-Process -Id ${Number(pid)}; ` +
    "@{working_set_bytes=$p.WorkingSet64;peak_working_set_bytes=$p.PeakWorkingSet64;" +
    "cpu_seconds=$p.TotalProcessorTime.TotalSeconds}|ConvertTo-Json -Compress";
  return JSON.parse(execFileSync("powershell", ["-NoProfile", "-Command", command], {
    windowsHide: true, encoding: "utf8", timeout: 10000,
  }));
}

// Mental model: drive a bounded closed-loop workload; stop on error/latency limits, retain every denominator.
async function probeWorkload(url, options) {
  requireLoopback(url);
  const samples = [];
  const started = performance.now();
  const deadline = started + options.durationMs;
  let aborted = false;
  let nextSample = 0;
  async function worker() {
    while (!aborted && performance.now() < deadline && nextSample < options.maxRequests) {
      nextSample += 1;
      const began = performance.now();
      let ok = false;
      let status = 0;
      try {
        const response = await fetch(url, { headers: options.headers,
          method: options.body === undefined ? "GET" : "POST",
          body: options.body === undefined ? undefined : JSON.stringify(options.body),
          signal: AbortSignal.timeout(5000) });
        status = response.status;
        const body = await response.json();
        ok = status === 200 && (options.validate ? options.validate(body) : true);
      } catch { /* Count transport/timeout failures; never omit them from the denominator. */ }
      samples.push({ durationMs: performance.now() - began, ok, status });
      if (samples.length >= 20 && (summarizeRequests(samples, 1).error_rate > 0.1 ||
          samples.at(-1).durationMs >= 5000)) aborted = true;
      if (options.intervalMs) await pause(options.intervalMs);
    }
  }
  await Promise.all(Array.from({ length: options.concurrency }, worker));
  return { ...summarizeRequests(samples, performance.now() - started),
    workload: { concurrency: options.concurrency, duration_limit_ms: options.durationMs,
      request_limit: options.maxRequests, interval_per_worker_ms: options.intervalMs || 0 },
    stop_reason: aborted ? "safety_threshold" : nextSample >= options.maxRequests ? "request_cap" : "time_limit",
    status_counts: samples.reduce((counts, item) => {
      counts[item.status] = (counts[item.status] || 0) + 1; return counts;
    }, {}),
  };
}

async function securityAndWorkflow(api) {
  const results = [];
  async function check(name, operation, expected) {
    const result = await operation();
    results.push({ name, expected_status: expected, actual_status: result.status,
      passed: result.status === expected });
    assert.equal(result.status, expected, name);
    return result;
  }
  await check("unauthenticated_session", () => request(api, "/auth/session"), 401);
  const citizen = await login(api, "citizen");
  const officer = await login(api, "government");
  await check("citizen_cannot_read_government_board", () => request(api, "/government/board", citizen), 403);
  await check("officer_cannot_read_citizen_portal", () => request(api, "/citizen/requests", officer), 403);
  await check("wrong_agency_case", () => request(api, "/government/requests/FOI-2026-001", officer), 403);
  await check("invalid_logout_csrf", () => request(api, "/auth/logout", officer, {}, "invalid"), 403);
  await check("missing_shared_provider_fallback", () => request(api, "/requester-ai/draft", citizen,
    { manual_title: "Scholarship statistics", what_to_ask: "Please provide published scholarship statistics for 2025." }), 503);
  await check("citizen_cannot_queue_government_draft", () => request(api,
    "/government/requests/FOI-2026-002/ai-draft", citizen, { verified_facts: "Verified scholarship report records for 2025." }), 403);
  await check("fro_cannot_publish", () => request(api,
    "/government/requests/FOI-2026-002/ai-drafts/evidence-job/publish", officer,
    { edited_body: "Reviewed response", source_output_hash: "evidence-hash", factual_review_confirmed: "yes" }), 403);
  const began = performance.now();
  const reads = Array.from({ length: 10 }, () => request(api, "/government/board", officer));
  await check("logout_during_pending_reads", () => request(api, "/auth/logout", officer, {}), 200);
  const readResults = await Promise.all(reads);
  assert.ok(readResults.every((result) => [200, 401].includes(result.status)));
  await check("revoked_session", () => request(api, "/auth/session", officer), 401);
  const raceMs = performance.now() - began;
  databaseFixture(api, "prepare");
  const decision = await login(api, "government");
  const publication = { edited_body: "Human-reviewed synthetic scholarship response.",
    source_output_hash: "evidence-hash", factual_review_confirmed: "yes" };
  await check("review_confirmation_required", () => request(api,
    "/government/requests/FOI-2026-002/ai-drafts/evidence-job/publish", decision,
    { ...publication, factual_review_confirmed: "" }), 422);
  await check("changed_source_hash_rejected", () => request(api,
    "/government/requests/FOI-2026-002/ai-drafts/evidence-job/publish", decision,
    { ...publication, source_output_hash: "stale-hash" }), 422);
  await check("reviewed_publication", () => request(api,
    "/government/requests/FOI-2026-002/ai-drafts/evidence-job/publish", decision, publication), 200);
  await check("repeat_publication", () => request(api,
    "/government/requests/FOI-2026-002/ai-drafts/evidence-job/publish", decision, publication), 422);
  const visible = await request(api, "/citizen/requests/FOI-2026-002", citizen);
  assert.ok(visible.data.timeline.messages.some((message) => message.body === publication.edited_body));
  results.push({ name: "citizen_receives_reviewed_response", passed: true });
  return { cases: results, passed: results.filter((item) => item.passed).length,
    attempted: results.length, concurrent_read_logout_duration_ms: raceMs, decision };
}

async function recoveryProbe(api) {
  const snapshot = path.join(api.directory, "snapshot.db");
  const restored = path.join(api.directory, "restored.db");
  const snapshotStarted = performance.now();
  const expected = databaseFixture(api, "snapshot", snapshot);
  const backupMs = performance.now() - snapshotStarted;
  const restartStarted = performance.now();
  await api.stop();
  const restarted = await startApi({ directory: api.directory, database: api.database, port: api.port });
  const restartMs = performance.now() - restartStarted;
  await restarted.stop();
  const restoreStarted = performance.now();
  const actual = JSON.parse(execFileSync("python", [path.join(__dirname, "db-fixture.py"),
    "recover", snapshot, restored], { encoding: "utf8", windowsHide: true }));
  assert.deepEqual(actual, expected);
  const recovered = await startApi({ directory: api.directory, database: restored, port: api.port });
  try {
    const citizen = await login(recovered, "citizen");
    const detail = await request(recovered, "/citizen/requests/FOI-2026-002", citizen);
    assert.ok(detail.data.timeline.messages.some((message) =>
      message.body === "Human-reviewed synthetic scholarship response."));
  } finally { await recovered.stop(); }
  return { status: "passed", online_backup_ms: backupMs, restart_ready_ms: restartMs,
    restore_and_api_verification_ms: performance.now() - restoreStarted,
    preserved_records: expected.counts, records_sha256_equal: true,
    scope: "isolated SQLite snapshot; tracked request/message/AI tables; no production RPO claim" };
}

module.exports = { probeWorkload, processResources, securityAndWorkflow, recoveryProbe, databaseFixture };
