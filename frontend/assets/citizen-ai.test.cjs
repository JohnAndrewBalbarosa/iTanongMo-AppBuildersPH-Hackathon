const assert = require("node:assert/strict");
const test = require("node:test");
const { initialize, validateDraft } = require("./citizen-ai.js");

const draft = {
  email_body: "Please provide the public budget allocation records for Bulacan.",
  description: "Budget allocation and spending records for Bulacan public projects.",
  purpose: "Public research", urgency_details: "",
  agency_queries: ["Department of Budget and Management"],
};

function fixture(api) {
  const ids = ["citizen-request-form", "citizen-ai-question", "citizen-request-title",
    "citizen-ai-generate", "citizen-ai-apply", "citizen-ai-status", "citizen-ai-preview",
    "citizen-request-details", "citizen-request-purpose", "citizen-request-urgency"];
  const elements = Object.fromEntries(ids.map(id => [id, {
    value: "", textContent: "", disabled: id === "citizen-ai-apply", handlers: {},
    addEventListener(name, handler) { this.handlers[name] = handler; }, focus() {},
  }]));
  elements["citizen-request-title"].value = "Bulacan public budget";
  elements["citizen-ai-question"].value = "How much was allotted for Bulacan public projects?";
  elements["citizen-request-details"].value = "Existing manual details";
  initialize(api, { getElementById: id => elements[id] });
  return elements;
}

test("shared API drafting sends facts without a provider, model or user key and requires Apply", async () => {
  let calls = 0;
  const elements = fixture(async (path, options) => {
    calls++;
    assert.equal(path, "/requester-ai/draft");
    assert.equal(options.method, "POST");
    assert.deepEqual(Object.keys(options.body).sort(), ["manual_title", "what_to_ask"]);
    return draft;
  });
  await elements["citizen-ai-generate"].handlers.click();
  assert.equal(calls, 1);
  assert.equal(elements["citizen-request-details"].value, "Existing manual details");
  assert.equal(elements["citizen-ai-apply"].disabled, false);
  elements["citizen-ai-apply"].handlers.click();
  assert.equal(elements["citizen-request-details"].value, draft.description);
  assert.equal(elements["citizen-request-purpose"].value, draft.purpose);
  assert.equal(elements["citizen-request-title"].value, "Bulacan public budget");
  assert.equal(elements["citizen-request-form"].handlers.submit, undefined);
});

test("AI output cannot set a requester identity or title", () => {
  assert.throws(() => validateDraft({ ...draft, requester_name: "Other person" }), /invalid JSON draft/);
  assert.throws(() => validateDraft({ ...draft, title: "Other title" }), /invalid JSON draft/);
  assert.throws(() => validateDraft({ ...draft, agency_queries: [] }), /invalid agency suggestions/);
  assert.throws(() => validateDraft({ ...draft, purpose: "" }), /invalid form values/);
});

test("failures preserve manual values and keep Apply disabled", async () => {
  const elements = fixture(async () => { throw new Error("AI service is temporarily unavailable"); });
  await elements["citizen-ai-generate"].handlers.click();
  assert.match(elements["citizen-ai-status"].textContent, /unavailable/);
  assert.equal(elements["citizen-ai-apply"].disabled, true);
  assert.equal(elements["citizen-ai-generate"].disabled, false);
  assert.equal(elements["citizen-request-details"].value, "Existing manual details");
});

test("editing inputs invalidates a pending draft and repeated clicks make only one call", async () => {
  let release;
  let calls = 0;
  const elements = fixture(() => { calls++; return new Promise(resolve => { release = resolve; }); });
  const pending = elements["citizen-ai-generate"].handlers.click();
  await elements["citizen-ai-generate"].handlers.click();
  elements["citizen-request-title"].handlers.input();
  release(draft);
  await pending;
  assert.equal(calls, 1);
  assert.equal(elements["citizen-ai-apply"].disabled, true);
  assert.match(elements["citizen-ai-status"].textContent, /inputs changed/);
});

test("resetting after submission clears the preview and prevents reapplying an old draft", async () => {
  const elements = fixture(async () => draft);
  await elements["citizen-ai-generate"].handlers.click();
  elements["citizen-request-form"].handlers.reset();
  assert.equal(elements["citizen-ai-apply"].disabled, true);
  assert.equal(elements["citizen-ai-preview"].textContent, "");
});
