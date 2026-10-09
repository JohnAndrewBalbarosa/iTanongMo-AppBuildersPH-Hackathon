const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const test = require("node:test");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "language.js"), "utf8");

function fixture(savedLanguage = "en") {
  const values = new Map([["itanongmo.language", savedLanguage]]);
  const events = [];
  const text = (value) => ({ nodeType: 3, textContent: value });
  const element = (tag, children = []) => ({
    nodeType: 1,
    tagName: tag.toUpperCase(),
    childNodes: children,
    attributes: new Map(),
    matches(selector) { return selector.split(", ").includes(tag); },
    hasAttribute(name) { return this.attributes.has(name); },
    getAttribute(name) { return this.attributes.get(name); },
    setAttribute(name, value) { this.attributes.set(name, value); },
    querySelectorAll() { return children.flatMap((child) =>
      child.nodeType === 1 ? [child, ...child.querySelectorAll()] : []); },
  });
  const headingText = text("Make a request");
  const statusText = text("3 of 10 agencies selected.");
  const heading = element("h1", [headingText]);
  heading.setAttribute("aria-label", "Main navigation");
  const status = element("p", [statusText]);
  const optionText = text("Public health");
  const option = element("option", [optionText]);
  option.value = optionText.textContent;
  optionText.parentElement = option;
  const root = element("html", [heading, status, option]);
  let onMutation;
  const document = {
    documentElement: root,
    body: root,
    dispatchEvent(event) { events.push(event); },
    addEventListener() {},
  };
  const context = {
    window: {}, document,
    localStorage: {
      getItem(key) { return values.get(key); },
      setItem(key, value) { values.set(key, value); },
    },
    Node: { TEXT_NODE: 3, ELEMENT_NODE: 1 },
    CustomEvent: class { constructor(type, options) { this.type = type; this.detail = options.detail; } },
    MutationObserver: class {
      constructor(callback) { onMutation = callback; }
      observe() {}
    },
  };
  vm.runInNewContext(source, context);
  return { ...context, values, events, heading, headingText, statusText, option, text, element,
    mutation(records) { onMutation(records); } };
}

test("switches the interface and document language without changing option values", () => {
  const page = fixture();
  const language = page.window.iTanongMoLanguage;
  language.setLanguage("fil");
  assert.equal(page.document.documentElement.lang, "fil");
  assert.equal(page.headingText.textContent, "Maghain ng kahilingan");
  assert.equal(page.heading.getAttribute("aria-label"), "Pangunahing nabigasyon");
  assert.equal(page.statusText.textContent, "3 sa 10 ahensiya ang napili.");
  assert.equal(page.option.value, "Public health");
  assert.equal(page.values.get("itanongmo.language"), "fil");
  assert.equal(page.events[0].detail.language, "fil");

  language.setLanguage("en");
  assert.equal(page.headingText.textContent, "Make a request");
  assert.equal(page.heading.getAttribute("aria-label"), "Main navigation");
  assert.equal(page.statusText.textContent, "3 of 10 agencies selected.");
  assert.equal(page.document.documentElement.lang, "en");
});

test("restores the saved preference and ignores unsupported languages", () => {
  const page = fixture("fil");
  const language = page.window.iTanongMoLanguage;
  language.translate();
  assert.equal(page.headingText.textContent, "Maghain ng kahilingan");
  language.setLanguage("es");
  assert.equal(language.current, "fil");
  assert.equal(page.values.get("itanongmo.language"), "fil");
  const newStatus = page.text("Checking your account…");
  page.mutation([{ type: "childList", addedNodes: [newStatus] }]);
  assert.equal(newStatus.textContent, "Sinusuri ang iyong account…");
});
