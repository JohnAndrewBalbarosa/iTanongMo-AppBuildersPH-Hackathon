const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "track.js"), "utf8");

test("guest tracking reads public status using only the reference", async () => {
  const calls = [];
  const output = {
    children: [], textContent: "",
    replaceChildren() { this.children = []; },
    append(node) { this.children.push(node); },
  };
  const form = {
    elements: { reference: { value: "" } },
    addEventListener(_, handler) { this.submit = handler; },
  };
  const document = {
    querySelector(selector) { return selector === "#tracking-form" ? form : output; },
    createElement(tagName) { return { tagName, textContent: "", setAttribute() {} }; },
  };
  const window = {
    iTanongMo: { apiBaseUrl: "https://api.example.test" },
    location: { search: "?reference=FOI-123" },
  };
  const fetch = async (url, options) => {
    calls.push({ url, options });
    return { ok: true, json: async () => ({ data: {
      reference: "FOI-123", title: "Records", agency: "DOST", status: "submitted",
      due_at: "2026-10-25", description: "Public records", events: [], messages: [], documents: [],
    } }) };
  };
  vm.runInNewContext(source, { document, window, fetch, URLSearchParams });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(form.elements.reference.value, "FOI-123");
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, "https://api.example.test/api/v1/requests/FOI-123");
  assert.equal(calls[0].options.credentials, "omit");
  assert.match(output.children[0].textContent, /FOI-123/);
});
