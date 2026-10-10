"use strict";
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const net = require("node:net");
const { spawnOwned, stopOwned } = require("./processes.cjs");
const { setTimeout: pause } = require("node:timers/promises");
const { requireLoopback } = require("./metrics.cjs");

async function freePort() {
  const listener = net.createServer();
  await new Promise((resolve, reject) => {
    listener.once("error", reject); listener.listen(0, "127.0.0.1", resolve);
  });
  const port = listener.address().port;
  await new Promise((resolve) => listener.close(resolve));
  return port;
}

// Mental model: allocate a disposable database, run only loopback, then await bounded readiness.
async function startApi(options = {}) {
  const root = options.root || process.cwd();
  const directory = options.directory || fs.mkdtempSync(path.join(os.tmpdir(), "itanongmo-evidence-"));
  const database = options.database || path.join(directory, "test.db");
  const port = options.port || await freePort();
  const origin = `http://127.0.0.1:${port}`;
  requireLoopback(origin);
  const frozen = path.join(root, "var/evidence-runtime/itanongmo.exe");
  const binary = fs.existsSync(frozen) ? frozen : path.join(root, "target/x86_64-pc-windows-gnu/debug/itanongmo.exe");
  const child = spawnOwned(binary, [], {
    windowsHide: true, stdio: ["ignore", "ignore", "ignore"], env: { ...process.env,
      ITANONGMO_BIND: `127.0.0.1:${port}`, ITANONGMO_PUBLIC_ORIGIN: origin,
      ITANONGMO_FRONTEND_ORIGIN: options.frontendOrigin || origin,
      ITANONGMO_DATABASE_URL: `sqlite://${database.replaceAll("\\", "/")}?mode=rwc`,
      ITANONGMO_DATA_DIR: directory, ITANONGMO_LOCAL_DEVELOPMENT: "true",
      ITANONGMO_REQUESTER_AI_ENABLED: "true", ITANONGMO_AI_BASE_URL: "",
      ITANONGMO_AI_PROXY_TOKEN: "", ITANONGMO_FEATURE_PERSONA_LOGIN: "false",
    },
  });
  let launchError;
  child.once("error", (error) => { launchError = error; });
  async function stop() {
    if (child.exitCode !== null || child.signalCode !== null || launchError) return;
    const ended = new Promise((resolve) => child.once("exit", resolve));
    stopOwned(child);
    await Promise.race([ended, pause(5000)]);
    if (child.exitCode === null && child.signalCode === null) throw new Error("API cleanup timeout");
  }
  try {
    const deadline = Date.now() + 20000;
    let ready = false;
    while (Date.now() < deadline && !ready) {
      if (launchError || child.exitCode !== null) throw new Error("Local API failed to launch");
      try { ready = (await fetch(`${origin}/health`, { signal: AbortSignal.timeout(1000) })).ok; }
      catch { /* Bounded readiness retry while Argon2 demo seeding initializes. */ }
      if (!ready) await pause(250);
    }
    if (!ready) throw new Error("Local API readiness deadline exceeded");
    return { child, directory, database, port, origin, stop };
  } catch (error) { await stop(); throw error; }
}

async function request(api, endpoint, actor, body, csrfOverride) {
  requireLoopback(api.origin);
  const response = await fetch(`${api.origin}/api/v1${endpoint}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { "Content-Type": "application/json", ...(actor ? {
      Cookie: actor.cookie, "X-CSRF-Token": csrfOverride ?? actor.csrf,
    } : {}) },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(5000),
  });
  const text = await response.text();
  let payload;
  let format = "json";
  try { payload = JSON.parse(text); }
  catch {
    if (response.ok) throw new Error("Successful API response must contain JSON");
    payload = {};
    format = "text";
  }
  return { status: response.status, data: payload.data, headers: response.headers, format };
}

async function login(api, role) {
  const government = role === "government";
  const result = await request(api, "/auth/login", null, {
    email: government ? "officer.judge@itanongmo.local" : "citizen.judge@itanongmo.local",
    password: government ? "OfficerJudge2026!" : "CitizenJudge2026!",
  });
  if (result.status !== 200) throw new Error("Demo login failed");
  return { cookie: result.headers.getSetCookie()[0].split(";")[0],
    csrf: result.data.csrf_token, roles: result.data.actor.roles };
}

module.exports = { freePort, startApi, request, login };
