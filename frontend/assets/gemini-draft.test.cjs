const assert = require("node:assert/strict");
const test = require("node:test");
const { generateDraft, validateDraft } = require("./gemini-draft.js");

const draft = {
  email_body: "Please provide the public budget allocation records for Bulacan.",
  description: "Budget allocation and spending records for Bulacan public projects.",
  purpose: "Public research",
  urgency_details: "",
  agency_queries: ["Department of Budget and Management"],
};

test("Gemini BYOK sends the key directly to Google and validates the draft", async () => {
  let calls = 0;
  const fetchMock = async (url, options) => {
    calls++;
    assert.equal(url, "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent");
    assert.equal(options.headers["x-goog-api-key"], "user-owned-key-123");
    assert.equal(options.credentials, "omit");
    assert.equal(options.referrerPolicy, "no-referrer");
    assert.ok(!url.includes("user-owned-key-123"));
    assert.ok(!options.body.includes("user-owned-key-123"));
    const request = JSON.parse(options.body);
    assert.equal(request.generationConfig.responseMimeType, "application/json");
    assert.ok(!request.generationConfig.responseSchema.properties.title);
    return { ok: true, json: async () => ({ candidates: [{ content: { parts: [{ text: JSON.stringify(draft) }] } }] }) };
  };
  const result = await generateDraft("user-owned-key-123", {
    what_to_ask: "How much was allotted for Bulacan public projects?",
    manual_title: "Bulacan public budget",
  }, fetchMock);
  assert.deepEqual(result, draft);
  assert.equal(calls, 1);
});

test("Gemini output cannot set a requester identity or title", () => {
  assert.throws(() => validateDraft({ ...draft, requester_name: "Other person" }), /invalid JSON draft/);
  assert.throws(() => validateDraft({ ...draft, title: "Other title" }), /invalid JSON draft/);
  assert.throws(() => validateDraft({ ...draft, agency_queries: [] }), /invalid agency suggestions/);
});

test("provider failures do not display provider response bodies", async () => {
  const fetchMock = async () => ({ ok: false, status: 403, text: async () => "sensitive provider detail" });
  await assert.rejects(
    generateDraft("user-owned-key-123", { what_to_ask: "How much was allotted for Bulacan public projects?", manual_title: "" }, fetchMock),
    /Gemini rejected the key or request/,
  );
});
