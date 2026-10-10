"use strict";
const fs = require("node:fs");
const path = require("node:path");
const os = require("node:os");
const crypto = require("node:crypto");
const { execFileSync } = require("node:child_process");
const { spawnOwned, stopOwned } = require("./processes.cjs");
const { performance } = require("node:perf_hooks");
const { prepareBrowser } = require("./browser.cjs");
const { freePort, startApi } = require("./local-api.cjs");
const { probeWorkload, processResources, securityAndWorkflow, recoveryProbe } = require("./api-probes.cjs");

const root = path.resolve(__dirname, "../..");
const python = path.join(root, "ai-local/.venv/Scripts/python.exe");
const cargo = "cargo";
const toolchain = "+1.98.0-x86_64-pc-windows-gnu";
const target = "x86_64-pc-windows-gnu";
const iso = (code) => `iso25010.${code}`;

function jsonFile(file, value) { fs.writeFileSync(file, JSON.stringify(value, null, 2) + "\n"); }
function sha256(file) { return crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex"); }

function boundedCommand(command, args, label, output, allowTestFailure = false) {
  return new Promise((resolve, reject) => {
    const started = performance.now();
    const child = spawnOwned(command, args, { cwd: root, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
    const tail = [];
    const counts = { tests: 0, passed: 0, failed: 0, ignored: 0 };
    const timeout = setTimeout(() => stopOwned(child), 300000);
    function consume(stream) {
      let partial = "";
      function line(text) {
        try {
          const structured = JSON.parse(text);
          if (structured.event === "evidence.node.summary") Object.assign(counts, structured.counts);
        } catch { /* Non-JSON diagnostics belong only in the bounded failure tail. */ }
        const pythonFailure = text.match(/^FAILED \((.*)\)/);
        if (pythonFailure) {
          counts.failed = [...pythonFailure[1].matchAll(/(?:failures|errors)=(\d+)/g)]
            .reduce((sum, match) => sum + Number(match[1]), 0);
        }
        const rust = text.match(/test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/);
        const tap = text.match(/^# (tests|pass|fail|skipped) (\d+)/);
        const unittest = text.match(/^Ran (\d+) tests?/);
        if (rust) {
          counts.passed += Number(rust[1]); counts.failed += Number(rust[2]); counts.ignored += Number(rust[3]);
          counts.tests += Number(rust[1]) + Number(rust[2]) + Number(rust[3]);
        } else if (tap) {
          counts[{ tests: "tests", pass: "passed", fail: "failed", skipped: "ignored" }[tap[1]]] = Number(tap[2]);
        } else if (unittest) { counts.tests = Number(unittest[1]); }
        tail.push(/password|token|cookie|credential/i.test(text) ? "[sensitive diagnostic line omitted]" : text.slice(0, 300));
        if (tail.length > 12) tail.shift();
      }
      stream.on("data", (chunk) => {
        partial += chunk.toString();
        const lines = partial.split(/\r?\n/); partial = lines.pop(); lines.forEach(line);
        if (partial.length > 4096) partial = partial.slice(-4096);
      });
      stream.on("end", () => { if (partial) line(partial); });
    }
    consume(child.stdout); consume(child.stderr);
    child.once("error", () => { clearTimeout(timeout); reject(new Error(`${label}: launch failed`)); });
    child.once("close", (exitCode) => {
      clearTimeout(timeout);
      if (label.includes("python")) counts.passed = Math.max(0, counts.tests - counts.failed - counts.ignored);
      const record = { label, exit_code: exitCode, duration_ms: performance.now() - started, counts };
      if (exitCode !== 0) record.error_tail = tail;
      jsonFile(path.join(output, `${label}.json`), record);
      console.log(JSON.stringify({ event: "evidence.command.completed", ...record }));
      if (exitCode === 0 || allowTestFailure) resolve(record); else reject(new Error(`${label}: check failed`));
    });
  });
}

function evidence(testType, classifications, method, workload, measurements, limitations = []) {
  return { test_type: testType, classification_codes: classifications.map(iso), method, workload, measurements,
    verification: "unverified", maturity: "controlled_measured", resume_eligible: false,
    limitations: [...limitations, "Requires explicit human verification before resume promotion"] };
}

function startClassifier(output, port, origin) {
  const cache = path.join(root, "var/evidence-input/triage-cache.json");
  const child = spawnOwned(python, [path.join(__dirname, "triage-benchmark.py"), "--output",
    path.join(output, "triage-benchmark.json"), "--serve-port", String(port), "--test-origin", origin,
    ...(fs.existsSync(cache) ? ["--reuse-benchmark", cache] : [])], {
    cwd: root, windowsHide: true, stdio: ["ignore", "pipe", "pipe"], env: { ...process.env,
      HF_HUB_OFFLINE: "1", TRANSFORMERS_OFFLINE: "1", OMP_NUM_THREADS: "4", MKL_NUM_THREADS: "4" },
  });
  const ready = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => { stopOwned(child); reject(new Error("Classifier readiness deadline exceeded")); }, 300000);
    let partial = "";
    child.stdout.on("data", (chunk) => {
      partial += chunk.toString();
      const lines = partial.split(/\r?\n/); partial = lines.pop();
      for (const line of lines) {
        try {
          const event = JSON.parse(line);
          if (event.event === "triage.test_server.ready") { clearTimeout(timeout); resolve(event.pid); }
          if (event.event === "triage.benchmark.completed") console.log(line);
        } catch { /* Ignore non-JSON library notices; never collect model payloads. */ }
      }
    });
    child.stderr.resume();
    child.once("error", () => { clearTimeout(timeout); reject(new Error("Classifier launch failed")); });
    child.once("exit", () => { clearTimeout(timeout); reject(new Error("Classifier exited before readiness")); });
  });
  return { child, ready };
}

async function staticChecks(output) {
  const javascript = ["tests/evidence/campaign.cjs", "tests/evidence/browser.cjs", "tests/evidence/api-probes.cjs",
    "tests/evidence/local-api.cjs", "tests/evidence/metrics.cjs", "tests/evidence/node-reporter.cjs", "tests/evidence/processes.cjs", "frontend/assets/local-triage.js"];
  for (const file of javascript) await boundedCommand(process.execPath, ["--check", file],
    `syntax-${path.basename(file)}`, output);
  await boundedCommand(python, ["-m", "ruff", "check", "tests/evidence"], "python-lint", output);
  await boundedCommand(python, ["-m", "ruff", "format", "--check", "tests/evidence"], "python-format", output);
  await boundedCommand(cargo, [toolchain, "fmt", "--", "--check"], "rust-format", output);
  await boundedCommand(cargo, [toolchain, "clippy", "--locked", "--target", target, "--all-targets", "--", "-D", "warnings"], "rust-clippy", output);
}

async function suites(output) {
  const nodeTests = fs.readdirSync(path.join(root, "frontend/assets"))
    .filter((file) => file.endsWith(".test.cjs")).map((file) => `frontend/assets/${file}`);
  return [
    await boundedCommand(cargo, [toolchain, "test", "--locked", "--target", target], "rust-suite", output, true),
    await boundedCommand(process.execPath, ["--test", "--test-reporter=./tests/evidence/node-reporter.cjs", ...nodeTests, "frontend/triage/policy.test.mjs",
      "tests/evidence/metrics.test.cjs"], "node-suite", output, true),
    await boundedCommand(python, ["-m", "unittest", "discover", "-s", "ai-local", "-p", "test_*.py"], "python-companion-suite", output, true),
    await boundedCommand(python, ["-m", "unittest", "discover", "-s", "tests/evidence", "-p", "test_*.py"], "python-evidence-suite", output),
  ];
}

function writeReport(output, report) {
  jsonFile(path.join(output, "report.json"), report);
  const assets = fs.readdirSync(output).filter((name) => name !== "manifest.json" && name !== "report.md")
    .map((name) => ({ path: name, sha256: sha256(path.join(output, name)) }));
  jsonFile(path.join(output, "manifest.json"), { schema_version: "evidence-artifacts/v1", artifacts: assets });
  const triage = report.evidence.find((item) => item.test_type === "benchmark");
  const rows = report.evidence.map((item) => `| ${item.test_type} | ${item.method} | ${JSON.stringify(item.measurements)} |`);
  fs.writeFileSync(path.join(output, "report.md"), [
    "# iTanongMo local triage evidence", "", `Outcome: **${report.status}**. Commit: \`${report.commit}\`.`, "",
    "All resume evidence remains **unverified** and **not resume-eligible** pending explicit human review.", "",
    "## Primary feature: local AI triage", "",
    triage ? `Synthetic benchmark macro-F1: **${triage.measurements.macro_f1}**; real pinned CPU model, fixed threshold 0.5. This is not independently verified FOI accuracy.` : "Benchmark unavailable.",
    "", "## Measurements", "", "| Type | Method | Measurements |", "|---|---|---|", ...rows,
    "", "## Findings", "", ...report.findings.map((item) => `- ${item}`),
    "", "## Pending evidence", "", ...report.pending.map((item) => `- ${item}`), "",
    "## Limitations", "", ...report.limitations.map((item) => `- ${item}`), "",
    "Methods, workloads, per-label results and environment: report.json and triage-benchmark.json. File hashes: manifest.json.", "",
  ].join("\n"));
}

// Mental model: verify the whole change, measure real local triage, exercise review boundaries, then soak and recover.
async function main() {
  if (process.argv.length > 2) throw new Error("Campaign has a fixed local-only workload; no target URL is accepted");
  process.chdir(root);
  const output = path.join(root, "var/test-evidence", new Date().toISOString().replaceAll(/[:.]/g, "-"));
  fs.mkdirSync(output, { recursive: true });
  const report = { schema_version: "local-test-evidence/v1", primary_feature: "local_ai_triage",
    started_at: new Date().toISOString(), commit: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
    source_state: "worktree; see source-manifest.json hashes", environment: { platform: os.platform(), release: os.release(),
      node: process.version, cpu_model: os.cpus()[0].model, logical_cpus: os.cpus().length, total_ram_bytes: os.totalmem() },
    status: "running", evidence: [], findings: [], pending: [
      "Real shared-AI drafting quality: provider intentionally unconfigured in isolated test runtime",
      "Independent FOI labels, factual-grounding review and explicit user/officer evidence verification",
      "Firefox/Safari, physical devices, full screen-reader/WCAG audit",
      "Release EXE download/first-launch packaging and live Pages local-network permission",
    ], limitations: ["Local synthetic workload; no production-capacity, SLA, ISO-certification or population-accuracy claim",
      "Client concurrency and request caps are workload parameters, not measured server capacity",
      "Node closed-loop probes reuse installed runtime; k6 considered but unavailable; no new dependencies",
      "Peak working set is Windows lifetime counter; start/end snapshots do not prove absence of leaks",
      "Classifier benchmark was completed before browser/load probes; soak includes API and local inference concurrently",
      "Shared technology registry commands unavailable in installed observability CLI; repository-local reuse recorded",
    ] };
  let browser;
  let model;
  let api;
  try {
    const sourceFiles = [...fs.readdirSync(__dirname).filter((name) => /\.(cjs|py)$/.test(name)).map((name) => `tests/evidence/${name}`),
      "tests/evidence/fixtures/foi-cases.json", "frontend/assets/local-triage.js", "frontend/assets/portal-layout.css",
      "frontend/government-demo.html", "ai-local/triage.py", "ai-local/companion.py"];
    jsonFile(path.join(output, "source-manifest.json"), sourceFiles.map((file) => ({ path: file, sha256: sha256(path.join(root, file)) })));
    await staticChecks(output);
    const results = await suites(output);
    for (const result of results) report.evidence.push(evidence("integration", ["functional_suitability", "security"],
      result.label, {}, { ...result.counts, duration_ms: result.duration_ms, exit_code: result.exit_code }));
    const failedSuites = results.filter((result) => result.exit_code !== 0);
    for (const result of failedSuites) report.findings.push(`Existing suite failure: ${result.label}; retained assertions and bounded failure evidence`);
    const frozen = path.join(root, "var/evidence-runtime/itanongmo.exe");
    fs.mkdirSync(path.dirname(frozen), { recursive: true });
    fs.copyFileSync(path.join(root, "target/x86_64-pc-windows-gnu/debug/itanongmo.exe"), frozen);
    report.environment.backend_binary_sha256 = sha256(frozen);
    if (fs.existsSync(path.join(root, "var/evidence-preflight.json"))) {
      const preflight = JSON.parse(fs.readFileSync(path.join(root, "var/evidence-preflight.json"), "utf8"));
      report.preflight = preflight;
      report.findings.push(...preflight.unresolved_findings);
    }
    const modelPort = await freePort();
    browser = await prepareBrowser(output, modelPort);
    model = startClassifier(output, modelPort, browser.origin);
    model.pid = await model.ready;
    const benchmark = JSON.parse(fs.readFileSync(path.join(output, "triage-benchmark.json"), "utf8"));
    if (benchmark.measurement_provenance) report.benchmark_provenance = benchmark.measurement_provenance;
    report.evidence.push(evidence("benchmark", ["functional_suitability", "performance_efficiency"],
      "Pinned real local CPU classifier; 32 synthetic multilingual cases; fixed score threshold",
      benchmark.workload, benchmark.measurements, benchmark.limitations));
    const browserResult = await browser.run();
    jsonFile(path.join(output, "browser.json"), browserResult);
    report.evidence.push(evidence("end_to_end", ["functional_suitability", "interaction_capability", "safety"],
      "Real local triage in Chrome; no automatic save; explicit reviewed response publish", {}, {
        local_triage_ms: browserResult.triage_browser_duration_ms, automatic_backend_mutations: 0,
        citizen_submissions: browserResult.citizen_submissions, reviewed_publications: browserResult.reviewed_publications,
        viewport_checks_passed: browserResult.layouts.length }, browserResult.limitations));
    report.evidence.push(evidence("accessibility", ["interaction_capability"],
      "Focused classify button and checked accessible button name", {}, { checks_passed: 2 },
      ["Limited keyboard/accessibility-tree checks; no WCAG compliance claim"]));
    report.evidence.push(evidence("compatibility", ["compatibility"],
      "Chrome desktop 1280x1000 and emulated mobile 390x844", {}, { viewport_checks_passed: 2, overflow_count: 0 }));
    const modelOrigin = browser.origin;
    await browser.close(); browser = null;
    api = await startApi();
    const security = await securityAndWorkflow(api);
    jsonFile(path.join(output, "security.json"), { ...security, decision: undefined });
    report.evidence.push(evidence("security", ["security", "safety"],
      "Role, agency, CSRF, revoked-session and reviewed-source negative checks", { pending_reads: 10 }, {
        passed: security.passed, attempted: security.attempted, concurrent_logout_ms: security.concurrent_read_logout_duration_ms }));
    const validateScores = (result) => result.human_review_required === true &&
      Object.values(result.scores || {}).length === 8 && Object.values(result.scores).every((score) =>
        Number.isFinite(score) && score >= 0 && score <= 1);
    const triageOptions = { headers: { Origin: modelOrigin, "Content-Type": "application/json" },
      body: { text: "Inaapela ko ang pagtanggi sa aking kahilingan at humihingi ng muling pagsusuri." }, validate: validateScores };
    for (const concurrency of [1, 2, 4]) {
      const result = await probeWorkload(`http://127.0.0.1:${modelPort}/classify`, {
        ...triageOptions, concurrency, durationMs: 60000, maxRequests: 8 });
      report.evidence.push(evidence("load", ["performance_efficiency"],
        "Local companion classification; bounded closed-loop requests", result.workload, result));
      if (result.failed) report.findings.push(`Local triage load: ${result.failed}/${result.attempts} errors at ${concurrency} workers`);
    }
    for (const concurrency of [1, 5, 10, 20]) {
      const resourcesBefore = processResources(api.child.pid);
      const result = await probeWorkload(`${api.origin}/api/v1/government/board`, {
        headers: { Cookie: security.decision.cookie }, concurrency, durationMs: 15000,
        maxRequests: 200, validate: (body) => Array.isArray(body.data?.cards) });
      report.evidence.push(evidence(concurrency === 20 ? "stress" : "load", ["performance_efficiency"],
        "Authenticated API board reads; stop at request/time cap", result.workload,
        { ...result, resources_start: resourcesBefore, resources_end: processResources(api.child.pid) }));
      if (result.failed) report.findings.push(`API load: ${result.failed}/${result.attempts} errors at ${concurrency} workers`);
    }
    console.log(JSON.stringify({ event: "evidence.soak.started", duration_seconds: 600,
      api_workers: 2, local_triage_interval_seconds: 20 }));
    const apiBefore = processResources(api.child.pid);
    const modelBefore = processResources(model.pid);
    const [apiSoak, triageSoak] = await Promise.all([
      probeWorkload(`${api.origin}/api/v1/government/board`, { headers: { Cookie: security.decision.cookie },
        concurrency: 2, durationMs: 600000, maxRequests: 2000, intervalMs: 1000,
        validate: (body) => Array.isArray(body.data?.cards) }),
      probeWorkload(`http://127.0.0.1:${modelPort}/classify`, { ...triageOptions,
        concurrency: 1, durationMs: 600000, maxRequests: 100, intervalMs: 20000 }),
    ]);
    report.evidence.push(evidence("soak", ["reliability", "performance_efficiency"],
      "Ten-minute simultaneous local API reads and real companion inference", { duration_seconds: 600 }, {
        api: apiSoak, local_triage: triageSoak, api_resources_start: apiBefore,
        api_resources_end: processResources(api.child.pid), model_resources_start: modelBefore,
        model_resources_end: processResources(model.pid) }));
    if (apiSoak.failed || triageSoak.failed) report.findings.push("Soak recorded failures; see measured durations and denominators");
    report.evidence.push(evidence("recovery", ["reliability"],
      "SQLite online backup; restart; restore into different temporary database; verify citizen-visible response", {}, await recoveryProbe(api)));
    report.status = failedSuites.length ? "failed_with_completed_measurements"
      : report.findings.length ? "completed_with_findings_and_pending_evidence" : "passed_with_pending_evidence";
    if (failedSuites.length) process.exitCode = 1;
  } catch (error) {
    report.status = "failed";
    report.failure = { type: error.name, reason: error.message.slice(0, 160) };
    process.exitCode = 1;
  } finally {
    try { if (browser) await browser.close(); if (api) await api.stop(); }
    finally { if (model) stopOwned(model.child); }
    report.evidence.forEach((record, index) => {
      record.id = `evidence-${index + 1}`;
      record.environment_id = "campaign-environment";
      record.artifact_manifest = "manifest.json";
      record.measurement_source = `report.json#/evidence/${index}/measurements`;
    });
    report.finished_at = new Date().toISOString();
    writeReport(output, report);
    console.log(JSON.stringify({ event: "evidence.campaign.completed", status: report.status,
      report: path.relative(root, path.join(output, "report.md")), evidence_records: report.evidence.length,
      pending: report.pending.length, resume_eligible: false }));
  }
}

if (require.main === module) main().catch(() => { console.error("Evidence campaign failed before report initialization"); process.exitCode = 1; });
