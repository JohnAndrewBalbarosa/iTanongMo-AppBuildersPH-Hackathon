"use strict";
const { spawn, execFileSync } = require("node:child_process");

const owned = new Set();

function stopOwned(child) {
  if (!owned.has(child) || child.exitCode !== null || child.signalCode !== null) return;
  if (process.platform === "win32") {
    try {
      execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
        windowsHide: true, stdio: "ignore", timeout: 10000,
      });
    } catch { child.kill(); }
  } else child.kill();
}

function spawnOwned(command, args, options) {
  const child = spawn(command, args, options);
  owned.add(child);
  child.once("exit", () => owned.delete(child));
  child.once("error", () => owned.delete(child));
  return child;
}

process.once("exit", () => { for (const child of owned) stopOwned(child); });

module.exports = { spawnOwned, stopOwned };
