const { test } = require("node:test");
const assert = require("node:assert/strict");
const { createController, parseDraft } = require("./government-ai.js");

function fixture(roles = ["decision_maker"], handler = async () => detail()) {
  const listeners = {};
  const view = Object.fromEntries(["facts", "generate", "refresh", "select", "body", "details",
    "review", "publish", "status"].map((key) => [key, { value: "", checked: false,
    addEventListener(event, callback) { listeners[`${key}:${event}`] = callback; } }]));
  view.options = (items) => { view.items = items; view.select.value = items[0]?.id || ""; };
  const calls = [];
  const controller = createController(async (path, options) => {
    calls.push({ path, options }); return handler(path, options);
  }, roles, view, async () => {});
  controller.setCase("R-1", detail());
  return { view, calls, controller, listeners };
}

function detail(hash = "hash-1", status = "completed") {
  return { ai_drafts: [{ id: "job-1", status, output_hash: hash,
    output: JSON.stringify({ body: "Verified response", subject: "Response",
      requires_human_review: true, limitations: ["Confirm records"] }) }] };
}

test("draft loading never generates or publishes automatically", () => {
  const f = fixture();
  assert.equal(f.calls.length, 0);
  assert.equal(f.view.body.value, "Verified response");
  assert.equal(f.view.publish.disabled, true);
});

test("review confirmation and publishing role are mandatory", async () => {
  for (const roles of [["fro"], ["decision_maker"]]) {
    const f = fixture(roles);
    await f.controller.publish();
    assert.equal(f.calls.length, 0);
    if (roles[0] === "fro") {
      f.view.review.checked = true;
      await f.controller.publish();
      assert.equal(f.calls.length, 0);
    }
  }
});

test("explicit publish sends edited body with reviewed source hash", async () => {
  const f = fixture();
  f.view.body.value = "Human reviewed response";
  f.view.review.checked = true;
  await f.controller.publish();
  assert.equal(f.calls.length, 2);
  assert.deepEqual(f.calls[1].options.body, {
    source_output_hash: "hash-1", edited_body: "Human reviewed response", factual_review_confirmed: "yes",
  });
  assert.match(f.calls[1].path, /job-1\/publish$/);
  assert.equal(f.view.publish.disabled, true);
});

test("editing invalidates review; unchanged refresh preserves user edits", () => {
  const f = fixture();
  f.view.body.value = "Human edits";
  f.view.review.checked = true;
  f.listeners["body:input"]();
  assert.equal(f.view.review.checked, false);
  f.controller.setCase("R-1", detail());
  assert.equal(f.view.body.value, "Human edits");
});

test("changed or already published source prevents publishing", async () => {
  for (const response of [detail("new-hash"), detail("hash-1", "published")]) {
    const f = fixture(undefined, async () => response);
    f.view.review.checked = true;
    await f.controller.publish();
    assert.equal(f.calls.length, 1);
    assert.equal(f.view.review.checked, false);
    assert.match(f.view.status.textContent, /Nagbago/);
  }
});

test("case switch during source check cancels publication and duplicate clicks", async () => {
  let resolve;
  const f = fixture(undefined, () => new Promise((done) => { resolve = done; }));
  f.view.review.checked = true;
  const publication = f.controller.publish();
  await f.controller.publish();
  f.controller.setCase("R-2", detail());
  resolve(detail());
  await publication;
  assert.equal(f.calls.length, 1);
  assert.equal(f.view.review.checked, false);
});

test("queue uses verified facts only and never sends the response", async () => {
  const f = fixture();
  f.view.facts.value = "Verified official records confirm the requested information.";
  await f.controller.generate();
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.calls[0].options.body, { verified_facts: f.view.facts.value });
  assert.match(f.calls[0].path, /ai-draft$/);
  assert.match(f.view.status.textContent, /Nakapila/);
});

test("pending, failed and malformed drafts never become publishable", () => {
  const f = fixture();
  for (const status of ["pending", "failed"]) {
    f.controller.setCase("R-1", detail("hash", status));
    assert.equal(f.view.publish.disabled, true);
  }
  assert.equal(parseDraft({ status: "completed", output_hash: "hash", output: "invalid" }), null);
  const malformed = detail();
  malformed.ai_drafts[0].output = JSON.stringify({ body: "unsafe", requires_human_review: false });
  f.controller.setCase("R-1", malformed);
  assert.equal(f.view.publish.disabled, true);
});

test("API errors are visible and leave manual reply independent", async () => {
  const f = fixture(undefined, async () => { throw new Error("AI unavailable"); });
  f.view.facts.value = "Verified facts for the requested response.";
  await f.controller.generate();
  assert.equal(f.view.status.textContent, "AI unavailable");
  assert.equal(f.view.generate.disabled, false);
});
