const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "site.js"), "utf8");

function node(tagName) {
  return {
    tagName,
    children: [],
    attributes: {},
    dataset: {},
    classList: { add() {} },
    append(...children) { this.children.push(...children); this.firstChild ||= this.children[0]; },
    prepend(child) { this.children.unshift(child); },
    replaceChildren(...children) { this.children = children; },
    setAttribute(name, value) { this.attributes[name] = value; },
    removeAttribute(name) { delete this.attributes[name]; },
    addEventListener(name, handler) { this[name] = handler; },
    querySelector(selector) {
      if (selector === "a") return this.children.find((child) => child.tagName === "a") || null;
      return null;
    },
    focus() {},
  };
}

async function page(fetch) {
  const header = node("header");
  const footer = node("footer");
  header.isConnected = true;
  footer.isConnected = true;
  const destinations = [];
  const document = {
    body: node("body"),
    head: node("head"),
    createElement: node,
    createElementNS(_namespace, name) { return node(name); },
    querySelector(selector) {
      if (selector === "header") return header;
      if (selector === "footer") return footer;
      return null;
    },
    getElementById() { return null; },
  };
  const window = {
    location: {
      pathname: "/index.html",
      assign(value) { destinations.push(value); },
      replace(value) { destinations.push(value); },
    },
    ITANONGMO_CONFIG: { apiBaseUrl: "https://api.example.test" },
  };
  vm.runInNewContext(source, { document, window, fetch });
  await new Promise((resolve) => setImmediate(resolve));
  const navigation = header.children[1].children[0];
  const actions = navigation.children.map((item) => item.children[0]);
  return { actions, footer, destinations, status: header.children[1].children[1] };
}

test("header keeps guest request and tracking links without a backend session", async () => {
  const { actions, footer } = await page(async () => ({ ok: false, status: 401 }));
  assert.deepEqual(actions.map((action) => action.textContent), ["Make a request", "Track a request", "About", "Sign in"]);
  assert.deepEqual(actions.slice(0, 2).map((action) => action.href), ["request.html", "track.html"]);
  assert.equal(footer.children[1].tagName, "label");
});

test("valid backend session shows Sign out and logout uses a fresh CSRF token", async () => {
  const requests = [];
  let reads = 0;
  const { actions, destinations } = await page(async (url, options) => {
    requests.push({ url, options });
    if (url.endsWith("/auth/session")) {
      reads += 1;
      return {
        ok: true,
        json: async () => ({ data: { actor: { actor_type: "requester" }, csrf_token: `token-${reads}` } }),
      };
    }
    return { ok: true };
  });
  assert.equal(actions[3].textContent, "Sign out");
  await actions[3].click({ preventDefault() {} });
  assert.equal(requests[0].options.credentials, "include");
  assert.equal(requests[2].options.credentials, "include");
  assert.equal(requests[2].options.headers["X-CSRF-Token"], "token-2");
  assert.deepEqual(destinations, ["login.html"]);
});

test("failed logout leaves the session action available and reports the error", async () => {
  const { actions, destinations, status } = await page(async (url) => url.endsWith("/auth/session")
    ? { ok: true, json: async () => ({ data: { actor: {}, csrf_token: "token" } }) }
    : { ok: false, status: 403 });
  await actions[3].click({ preventDefault() {} });
  assert.equal(actions[3].textContent, "Sign out");
  assert.equal(status.textContent, "Sign out failed. Please try again.");
  assert.deepEqual(destinations, []);
});
