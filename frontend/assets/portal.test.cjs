const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "portal.js"), "utf8");

function loginPage(fetch) {
  const handlers = {};
  const destinations = [];
  const status = { textContent: "" };
  const form = {
    elements: {
      email: { value: "officer.judge@itanongmo.local" },
      password: { value: "OfficerJudge2026!" },
    },
    addEventListener(name, handler) {
      handlers[name] = handler;
    },
  };
  const document = {
    body: { dataset: { portalPage: "login" } },
    getElementById(id) {
      return id === "portal-sign-in" ? form : status;
    },
  };
  const window = {
    ITANONGMO_CONFIG: { apiBaseUrl: "https://api.example.test" },
    location: { assign(destination) { destinations.push(destination); } },
  };
  vm.runInNewContext(source, { document, window, fetch });
  return { form, handlers, status, destinations };
}

test("login routes by the server actor and sends credentials without a client role", async () => {
  let request;
  const page = loginPage(async (url, options) => {
    request = { url, options };
    return {
      ok: true,
      json: async () => ({ data: { actor: { actor_type: "staff" }, csrf_token: "token" } }),
    };
  });
  await page.handlers.submit({ preventDefault() {}, currentTarget: page.form });
  assert.equal(request.url, "https://api.example.test/api/v1/auth/login");
  assert.equal(request.options.credentials, "include");
  assert.deepEqual(JSON.parse(request.options.body), {
    email: "officer.judge@itanongmo.local",
    password: "OfficerJudge2026!",
  });
  assert.deepEqual(page.destinations, ["government-demo.html"]);
  assert.equal(page.form.elements.password.value, "");
});

test("failed login stays on the page and shows the backend error", async () => {
  const page = loginPage(async () => ({
    ok: false,
    status: 401,
    json: async () => ({ error: { message: "Invalid email or password." } }),
  }));
  await page.handlers.submit({ preventDefault() {}, currentTarget: page.form });
  assert.deepEqual(page.destinations, []);
  assert.equal(page.status.textContent, "Invalid email or password.");
});
