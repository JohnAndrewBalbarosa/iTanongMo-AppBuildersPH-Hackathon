"use strict";

function percentile(values, fraction) {
  if (!values.length) return null;
  if (fraction < 0 || fraction > 1) throw new RangeError("Invalid percentile");
  const sorted = [...values].sort((left, right) => left - right);
  return sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)];
}

function summarizeRequests(samples, durationMs) {
  const latencies = samples.map((sample) => sample.durationMs);
  const failed = samples.filter((sample) => !sample.ok).length;
  return {
    attempts: samples.length,
    passed: samples.length - failed,
    failed,
    error_rate: samples.length ? failed / samples.length : null,
    duration_ms: durationMs,
    requests_per_second: durationMs > 0 ? samples.length * 1000 / durationMs : null,
    p50_ms: percentile(latencies, 0.5),
    p95_ms: percentile(latencies, 0.95),
    p99_ms: percentile(latencies, 0.99),
    latency_aggregation: "nearest-rank; includes failed requests; client-observed",
  };
}

function requireLoopback(url) {
  const parsed = new URL(url);
  if (parsed.protocol !== "http:" || parsed.hostname !== "127.0.0.1" ||
      parsed.username || parsed.password) throw new Error("Only isolated HTTP loopback is allowed");
  return parsed;
}

module.exports = { percentile, summarizeRequests, requireLoopback };
