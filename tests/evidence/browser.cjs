"use strict";
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const http = require("node:http");
const { spawnOwned, stopOwned } = require("./processes.cjs");
const { performance } = require("node:perf_hooks");
const { setTimeout: pause } = require("node:timers/promises");
const { startApi, freePort } = require("./local-api.cjs");
const { databaseFixture } = require("./api-probes.cjs");

async function prepareBrowser(output, modelPort) {
  let api;
  let submissionCount = 0;
  let mutationCount = 0;
  const root = process.cwd();
  const server = http.createServer(async (request, response) => {
    try {
      if (request.url.startsWith("/api/")) {
        const chunks = [];
        for await (const chunk of request) chunks.push(chunk);
        const headers = { ...request.headers };
        delete headers.host; delete headers["content-length"];
        const upstream = await fetch(api.origin + request.url, { method: request.method, headers,
          body: request.method === "GET" ? undefined : Buffer.concat(chunks),
          signal: AbortSignal.timeout(5000) });
        let body = Buffer.from(await upstream.arrayBuffer());
        let status = upstream.status;
        if (request.url === "/api/v1/requester-ai/draft" && status === 503) {
          status = 200;
          body = Buffer.from(JSON.stringify({ data: {
            email_body: "Please provide the published annual scholarship records.",
            description: "Published annual scholarship application and approval statistics for 2025.",
            purpose: "Public-interest research", urgency_details: "",
            agency_queries: ["Department of Science and Technology"],
          } }));
        }
        if (request.method === "POST") mutationCount += 1;
        if (request.url === "/api/v1/submissions" && status === 201) submissionCount += 1;
        response.statusCode = status;
        response.setHeader("content-type", "application/json");
        const cookies = upstream.headers.getSetCookie();
        if (cookies.length) response.setHeader("set-cookie", cookies);
        response.end(body);
        return;
      }
      if (request.url === "/config.js") {
        response.setHeader("content-type", "text/javascript");
        response.end(`window.ITANONGMO_CONFIG={apiBaseUrl:'${origin}'};`);
        return;
      }
      const file = path.resolve(root, "frontend", "." + new URL(request.url, origin).pathname);
      assert.ok(file.startsWith(path.join(root, "frontend") + path.sep));
      let content = fs.readFileSync(file);
      if (file.endsWith("local-triage.js")) {
        content = content.toString().replace("http://127.0.0.1:8765", `http://127.0.0.1:${modelPort}`);
      }
      response.setHeader("content-type", { ".html": "text/html", ".js": "text/javascript",
        ".css": "text/css" }[path.extname(file)] || "application/octet-stream");
      response.end(content);
    } catch {
      response.statusCode = 500; response.end("Isolated browser fixture failed");
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const origin = `http://127.0.0.1:${server.address().port}`;
  try { api = await startApi({ frontendOrigin: origin }); }
  catch (error) { server.close(); throw error; }

  async function close() {
    await api.stop(); server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }

  async function run() {
    const debugPort = await freePort();
    const chrome = spawnOwned("C:/Program Files/Google/Chrome/Application/chrome.exe", [
      "--headless=new", "--no-first-run", "--no-default-browser-check",
      `--remote-debugging-port=${debugPort}`, `--user-data-dir=${api.directory}/chrome`, "about:blank",
    ], { windowsHide: true, stdio: "ignore" });
    let socket;
    try {
      await pause(2500);
      const targets = await (await fetch(`http://127.0.0.1:${debugPort}/json/list`)).json();
      socket = new WebSocket(targets.find((target) => target.type === "page").webSocketDebuggerUrl);
      await new Promise((resolve) => socket.addEventListener("open", resolve, { once: true }));
      const pending = new Map();
      let nextId = 0;
      socket.addEventListener("message", (event) => {
        const message = JSON.parse(event.data);
        if (!pending.has(message.id)) return;
        const waiter = pending.get(message.id); pending.delete(message.id);
        clearTimeout(waiter.timer);
        if (message.error) waiter.reject(new Error("CDP command failed")); else waiter.resolve(message.result);
      });
      const command = (method, params = {}) => new Promise((resolve, reject) => {
        const id = ++nextId;
        const timer = setTimeout(() => { pending.delete(id); reject(new Error("CDP deadline exceeded")); }, 20000);
        pending.set(id, { resolve, reject, timer }); socket.send(JSON.stringify({ id, method, params }));
      });
      async function evaluate(expression) {
        const result = await command("Runtime.evaluate", { expression, awaitPromise: true, returnByValue: true });
        if (result.exceptionDetails) throw new Error("Browser evaluation failed");
        return result.result.value;
      }
      async function ready(expression) {
        const deadline = Date.now() + 20000;
        while (Date.now() < deadline) {
          if (await evaluate(expression)) return;
          await pause(100);
        }
        throw new Error("Browser readiness deadline exceeded");
      }
      await command("Page.navigate", { url: origin + "/login.html" });
      await ready("!!document.getElementById('portal-sign-in')");
      assert.equal(await evaluate(`fetch('/api/v1/auth/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:'citizen.judge@itanongmo.local',password:'CitizenJudge2026!'})}).then(r=>r.status)`), 200);
      await command("Page.navigate", { url: origin + "/citizen.html" });
      await ready("!!document.getElementById('citizen-request-title') && document.getElementById('citizen-request-list').children.length>0");
      await evaluate(`document.querySelector('.citizen-ai-options').open=true; document.getElementById('citizen-request-title').value='Scholarship records for review'; document.getElementById('citizen-ai-question').value='Please provide scholarship application and approval statistics for 2025.'; document.getElementById('citizen-ai-generate').click()`);
      await ready("!document.getElementById('citizen-ai-apply').disabled");
      assert.equal(submissionCount, 0);
      await evaluate("document.getElementById('citizen-ai-apply').click(); document.getElementById('citizen-requester-email').value='citizen.judge@itanongmo.local'; document.getElementById('citizen-agency').value='agency-dost'; document.querySelector('#citizen-request-form [name=consent]').checked=true; document.getElementById('citizen-request-form').requestSubmit()");
      await ready("document.getElementById('citizen-submit-status').textContent.includes('Submitted')");
      assert.equal(submissionCount, 1);
      await evaluate("document.getElementById('portal-logout').click()");
      await ready("location.pathname.endsWith('/login.html') && !!document.getElementById('portal-sign-in')");
      databaseFixture(api, "prepare");
      assert.equal(await evaluate(`fetch('/api/v1/auth/login',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({email:'officer.judge@itanongmo.local',password:'OfficerJudge2026!'})}).then(r=>r.status)`), 200);
      await command("Page.navigate", { url: origin + "/government-demo.html" });
      await ready("!!document.querySelector('[data-ai=body]') && document.querySelectorAll('.case-card').length>0");
      await evaluate("[...document.querySelectorAll('.case-card')].find(e=>e.textContent.includes('FOI-2026-002')).click()");
      await ready("document.querySelector('[data-ai=body]').value.includes('Verified scholarship')");
      assert.equal(await evaluate("document.querySelector('[data-ai=publish]').disabled"), true);
      const beforeTriage = mutationCount;
      const triageStarted = performance.now();
      await evaluate("document.getElementById('classify-local-case').click()");
      await ready("document.querySelectorAll('#local-model-scores li').length===8 && !document.getElementById('classify-local-case').disabled");
      const localTriageMs = performance.now() - triageStarted;
      assert.equal(mutationCount, beforeTriage, "local classification must never save or route automatically");
      await evaluate("window.iTanongMoLanguage.setLanguage('fil')");
      assert.equal(await evaluate("document.documentElement.lang"), "fil");
      assert.equal(await evaluate("document.querySelector('h1').textContent"), "Workspace para sa local AI triage");
      const layouts = [];
      for (const [name, width, height, mobile] of [["desktop", 1280, 1000, false], ["mobile", 390, 844, true]]) {
        await command("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile });
        await pause(100);
        const measurements = await evaluate(`(()=>{const panel=document.getElementById('local-triage');const fields=[...document.querySelectorAll('.government-ai textarea,.government-ai select')];return {overflow:document.documentElement.scrollWidth>innerWidth,fieldWidths:fields.map(e=>Math.round(e.getBoundingClientRect().width)),triageBeforeBoard:panel.compareDocumentPosition(document.querySelector('.board-panel'))===Node.DOCUMENT_POSITION_FOLLOWING,stepCount:panel.querySelectorAll('.local-triage-steps li').length};})()`);
        assert.equal(measurements.overflow, false);
        assert.equal(new Set(measurements.fieldWidths).size, 1);
        assert.equal(measurements.triageBeforeBoard, true);
        assert.equal(measurements.stepCount, 3);
        const metrics = await command("Page.getLayoutMetrics");
        const screenshot = await command("Page.captureScreenshot", { format: "png", captureBeyondViewport: true,
          clip: { x: 0, y: 0, width, height: Math.min(metrics.cssContentSize.height, 5000), scale: 1 } });
        fs.writeFileSync(path.join(output, `government-triage-${name}.png`), Buffer.from(screenshot.data, "base64"));
        layouts.push({ name, width, height, ...measurements });
      }
      await evaluate("document.getElementById('classify-local-case').focus()");
      assert.equal(await evaluate("document.activeElement.id"), "classify-local-case");
      const accessibility = await command("Accessibility.getFullAXTree");
      assert.ok(accessibility.nodes.some((node) => node.role?.value === "button" && /classify/i.test(node.name?.value || "")));
      await evaluate("let body=document.querySelector('[data-ai=body]');body.value='Human-reviewed scholarship response from browser fixture.';body.dispatchEvent(new Event('input'));let review=document.querySelector('[data-ai=review]');review.checked=true;review.dispatchEvent(new Event('change'));document.querySelector('[data-ai=publish]').click()");
      await ready("document.querySelector('[data-ai=body]').value==='' && document.querySelector('[data-ai=publish]').disabled");
      const published = await evaluate("fetch('/api/v1/government/requests/FOI-2026-002').then(r=>r.json()).then(r=>r.data.ai_drafts.find(j=>j.id==='evidence-job').status)");
      assert.equal(published, "published");
      return { status: "passed", primary_feature: "local_ai_triage", local_model: "real_pinned_cpu_model",
        triage_browser_duration_ms: localTriageMs, automatic_backend_mutations_after_classification: 0,
        citizen_submissions: submissionCount, reviewed_publications: 1, layouts,
        accessibility: { keyboard_focus: true, classify_button_in_accessibility_tree: true,
          full_wcag_audit: false, screen_reader_tested: false },
        limitations: ["Chrome headless only; no Firefox/Safari or physical-phone proof",
          "Local companion origin and port adapted in disposable test process; deployed Pages permission prompt not evaluated",
          "Citizen draft mocked; government response seeded; local triage model inference real",
          "Public release EXE packaging/download is separate from Python companion behavior"] };
    } finally { socket?.close(); stopOwned(chrome); }
  }
  return { origin, run, close };
}

module.exports = { prepareBrowser };
