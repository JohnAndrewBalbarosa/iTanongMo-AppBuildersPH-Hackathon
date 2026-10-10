const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "local-triage.js"), "utf8");
const categories = [
  "time_sensitive", "safety_review", "sensitive_data", "voluminous",
  "multi_unit", "appeal", "records_preservation", "clarification",
];

function page(fetch, title = "FOI-1 — Request", description = "Request details") {
  const elements = new Map();
  for (const id of [
    "local-model-status", "local-model-scores", "check-local-model", "classify-local-case",
    "government-case-title", "government-case-description",
  ]) {
    elements.set(id, {
      textContent: id === "government-case-title" ? title :
        id === "government-case-description" ? description : "",
      dataset: {},
      children: [],
      addEventListener(name, handler) { this[name] = handler; },
      replaceChildren() { this.children = []; },
      append(...children) { this.children.push(...children); },
    });
  }
  const document = {
    getElementById(id) { return elements.get(id); },
    addEventListener() {},
    querySelectorAll() { return []; },
    createElement() {
      return { children: [], append(...children) { this.children.push(...children); } };
    },
  };
  vm.runInNewContext(source, { document, window: {}, fetch, console: { error() {} } });
  return elements;
}

test("government local classifier sends only the selected case to loopback", async () => {
  const calls = [];
  const elements = page(async (url, options) => {
    calls.push({ url, options });
    return { ok: true, json: async () => ({ scores: Object.fromEntries(categories.map((key) => [key, 0.5])) }) };
  });
  await elements.get("classify-local-case").click();
  assert.equal(calls[0].url, "http://127.0.0.1:8765/classify");
  assert.equal(calls[0].options.credentials, "omit");
  assert.equal(JSON.parse(calls[0].options.body).text, "FOI-1 — Request\nRequest details");
  assert.equal(elements.get("local-model-scores").children.length, 8);
});

test("government local classifier requires a selected case", async () => {
  let called = false;
  const elements = page(async () => { called = true; }, "Choose a request", "");
  await elements.get("classify-local-case").click();
  assert.equal(called, false);
  assert.equal(elements.get("local-model-status").textContent, "Select a case on the request board first.");
});

test("government local classifier rejects incomplete scores", async () => {
  const elements = page(async () => ({ ok: true, json: async () => ({ scores: { appeal: 0.8 } }) }));
  await elements.get("classify-local-case").click();
  assert.equal(elements.get("local-model-scores").children.length, 0);
  assert.equal(elements.get("local-model-status").textContent, "The local model returned invalid scores.");
});
