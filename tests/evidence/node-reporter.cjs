"use strict";

// Consume the documented TestsStream summary rather than version-dependent console formatting.
module.exports = async function* reportTestSummary(events) {
  for await (const event of events) {
    if (event.type === "test:summary" && event.data.file === undefined) {
      const counts = event.data.counts;
      yield JSON.stringify({ event: "evidence.node.summary", counts: {
        tests: counts.tests, passed: counts.passed, failed: counts.failed,
        ignored: counts.skipped + counts.todo, cancelled: counts.cancelled,
      } }) + "\n";
    } else if (event.type === "test:fail") {
      yield JSON.stringify({ event: "evidence.node.failure",
        error_type: event.data.details?.error?.name || "TestFailure" }) + "\n";
    }
  }
};
