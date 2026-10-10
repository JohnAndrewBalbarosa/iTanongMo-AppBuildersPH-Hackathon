const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "request.js"), "utf8");

function element(tagName = "div") {
  return {
    tagName, children: [], value: "", textContent: "", hidden: false, disabled: false,
    addEventListener(name, callback) { this[name] = callback; },
    focus() {},
    append(...children) { this.children.push(...children); },
    replaceChildren(...children) {
      this.children = children;
      if (tagName === "select") this.value = children[0]?.value || "";
    },
  };
}

async function page(fetch) {
  const ids = [
    "foi-request-form", "agency-search", "agency-results", "add-agency", "agency-status",
    "submit-request", "selected-agencies", "agency-count", "submission-status",
    "batch-reference", "submission-results", "submission-output", "guest-request-section",
    "guest-ai-question", "guest-request-title-input", "guest-ai-generate", "guest-ai-apply",
    "guest-ai-status", "guest-ai-preview", "guest-request-description", "guest-request-purpose",
    "guest-request-urgency",
  ];
  const elements = new Map(ids.map((id) => [id, element(id === "agency-results" ? "select" : "div")]));
  const destinations = [];
  const document = {
    getElementById(id) { return elements.get(id); },
    createElement: element,
  };
  const window = {
    iTanongMo: { apiBaseUrl: "https://api.example.test" },
    iTanongMoCitizenAi: { validateDraft(value) { return value; } },
    setTimeout, clearTimeout,
    location: { replace(value) { destinations.push(value); } },
  };
  const FormData = class {
    entries() {
      return [
        ["requester_name", "Guest User"], ["requester_email", "guest@example.test"],
        ["title", "Public records"], ["description", "Please provide the public records."],
        ["purpose", "Research"], ["consent", "yes"],
      ];
    }
  };
  vm.runInNewContext(source, { document, window, fetch, FormData, URL, AbortController });
  await new Promise((resolve) => setImmediate(resolve));
  return { elements, destinations };
}

test("guest can submit without a session and receives references", async () => {
  const requests = [];
  const { elements, destinations } = await page(async (url, options) => {
    requests.push({ url, options });
    if (url.endsWith("/auth/session")) return { ok: false, status: 401 };
    if (url.endsWith("/config")) return { ok: true, json: async () => ({ data: { guest_ai_enabled: false } }) };
    if (url.endsWith("/agencies")) {
      return { ok: true, json: async () => ({ data: [{ id: "dost", name: "Science Department", short_name: "DOST" }] }) };
    }
    return { ok: true, json: async () => ({ data: {
      batch_reference: "FOI-BATCH-1",
      requests: [{ reference: "FOI-1", agency: "DOST" }],
    } }) };
  });
  await elements.get("add-agency").click();
  await elements.get("foi-request-form").submit({ preventDefault() {} });
  assert.deepEqual(destinations, []);
  assert.equal(requests[3].options.credentials, "omit");
  assert.equal(JSON.parse(requests[3].options.body).agency_ids, "dost");
  assert.equal(elements.get("submission-results").children[0].children[0].href, "track.html?reference=FOI-1");
  assert.equal(elements.get("guest-request-section").hidden, true);
});

test("signed-in requester uses the portal instead of creating a guest request", async () => {
  const requests = [];
  const { destinations } = await page(async (url, options) => {
    requests.push({ url, options });
    return { ok: true, json: async () => ({ data: { actor: { actor_type: "requester" } } }) };
  });
  assert.deepEqual(destinations, ["citizen.html#citizen-request-form"]);
  assert.equal(requests.length, 1);
  assert.equal(requests[0].options.credentials, "include");
});

test("guest submission without an agency never calls the write endpoint", async () => {
  const requests = [];
  const { elements } = await page(async (url) => {
    requests.push(url);
    if (url.endsWith("/auth/session")) return { ok: false, status: 401 };
    if (url.endsWith("/config")) return { ok: true, json: async () => ({ data: { guest_ai_enabled: false } }) };
    return { ok: true, json: async () => ({ data: [{ id: "dost", name: "Science Department" }] }) };
  });
  await elements.get("foi-request-form").submit({ preventDefault() {} });
  assert.equal(requests.length, 3);
  assert.equal(elements.get("submission-status").textContent, "Select at least one agency.");
});

test("session lookup failure does not block the guest form", async () => {
  const requests = [];
  const { elements, destinations } = await page(async (url) => {
    requests.push(url);
    if (url.endsWith("/auth/session")) throw new Error("Session unavailable");
    if (url.endsWith("/config")) return { ok: true, json: async () => ({ data: { guest_ai_enabled: false } }) };
    return { ok: true, json: async () => ({ data: [{ id: "dost", name: "Science Department" }] }) };
  });
  assert.deepEqual(destinations, []);
  assert.equal(requests.length, 3);
  assert.equal(elements.get("submit-request").disabled, false);
});

test("guest AI preview applies only request text after explicit review", async () => {
  const draft = {
    email_body: "Please send the public program records.",
    description: "Please provide public program records.",
    purpose: "Public interest research",
    urgency_details: "No urgent concern",
    agency_queries: ["Science Department"],
  };
  const requests = [];
  const { elements } = await page(async (url, options) => {
    requests.push({ url, options });
    if (url.endsWith("/auth/session")) return { ok: false, status: 401 };
    if (url.endsWith("/config")) return { ok: true, json: async () => ({ data: { guest_ai_enabled: true } }) };
    if (url.endsWith("/agencies")) return { ok: true, json: async () => ({ data: [{ id: "dost", name: "Science Department" }] }) };
    return { ok: true, json: async () => ({ data: draft }) };
  });
  elements.get("guest-request-title-input").value = "Program records";
  elements.get("guest-ai-question").value = "Please provide the public program records from this year.";
  await elements.get("guest-ai-generate").click();
  assert.equal(requests[3].options.credentials, "omit");
  assert.equal(requests[3].options.headers.Authorization, undefined);
  assert.equal(elements.get("guest-request-description").value, "");
  assert.match(elements.get("guest-ai-preview").textContent, /Science Department/);
  await elements.get("guest-ai-apply").click();
  assert.equal(elements.get("guest-request-description").value, draft.description);
  assert.equal(elements.get("guest-request-purpose").value, draft.purpose);
  assert.equal(elements.get("guest-request-urgency").value, draft.urgency_details);
});
