const { test } = require("node:test");
const assert = require("node:assert/strict");
const { percentile, summarizeRequests, requireLoopback } = require("./metrics.cjs");

test("nearest-rank preserves numeric sorting, tail failures and denominator", () => {
  const input = [{ durationMs: 100, ok: false }, { durationMs: 2, ok: true },
    { durationMs: 10, ok: true }, { durationMs: 1, ok: true }];
  const result = summarizeRequests(input, 2000);
  assert.equal(result.error_rate, 0.25);
  assert.equal(result.requests_per_second, 2);
  assert.equal(result.p50_ms, 2);
  assert.equal(result.p95_ms, 100);
  assert.equal(result.p99_ms, 100);
  assert.equal(input[0].durationMs, 100);
});

test("empty measurements remain unknown rather than fabricated success", () => {
  assert.equal(percentile([], 0.95), null);
  const result = summarizeRequests([], 0);
  assert.equal(result.error_rate, null);
  assert.equal(result.requests_per_second, null);
});

test("the campaign rejects public endpoints and URL credentials", () => {
  for (const url of ["https://example.com", "http://localhost", "http://127.0.0.2",
    "https://127.0.0.1", "http://secret@127.0.0.1"]) {
    assert.throws(() => requireLoopback(url));
  }
  assert.equal(requireLoopback("http://127.0.0.1:1234/health").port, "1234");
});

test("structured reporter uses the global summary and retains skips and cancellations", async () => {
  const reporter = require("./node-reporter.cjs");
  const events = [
    { type: "test:summary", data: { file: "one.test.cjs", counts: { tests: 2 } } },
    { type: "test:summary", data: { counts: {
      tests: 8, passed: 5, failed: 1, skipped: 1, todo: 0, cancelled: 1,
    } } },
  ];
  const records = [];
  for await (const record of reporter(events)) records.push(JSON.parse(record));
  assert.equal(records.length, 1);
  assert.deepEqual(records[0].counts, { tests: 8, passed: 5, failed: 1, ignored: 1, cancelled: 1 });
});

test("the HTTP adapter preserves plain-text error status and rejects malformed successful JSON", async () => {
  const http = require("node:http");
  const { request } = require("./local-api.cjs");
  const server = http.createServer((incoming, response) => {
    response.statusCode = incoming.url.endsWith("/error") ? 503 : 200;
    response.end("AI service unavailable");
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const api = { origin: `http://127.0.0.1:${server.address().port}` };
  try {
    const result = await request(api, "/error");
    assert.equal(result.status, 503);
    assert.equal(result.format, "text");
    assert.equal(result.data, undefined);
    await assert.rejects(request(api, "/malformed-success"), /must contain JSON/);
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
});
