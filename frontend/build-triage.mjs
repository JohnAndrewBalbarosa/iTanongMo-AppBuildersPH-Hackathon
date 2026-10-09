import { cp, mkdir, readdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const frontend = fileURLToPath(new URL(".", import.meta.url));
const assets = join(frontend, "assets");
const wasmSource = join(frontend, "node_modules", "onnxruntime-web", "dist");
const wasmTarget = join(assets, "wasm");
const site = join(frontend, "site");

await build({
  entryPoints: [join(frontend, "triage", "worker.mjs")],
  outfile: join(assets, "triage-worker.bundle.js"),
  bundle: true,
  format: "esm",
  platform: "browser",
  target: "es2022",
  minify: true,
  define: { "process.env.NODE_ENV": '"production"' },
});

await mkdir(wasmTarget, { recursive: true });
const runtimeFiles = (await readdir(wasmSource)).filter(
  (name) => name.startsWith("ort-wasm") && (name.endsWith(".wasm") || name.endsWith(".mjs")),
);
if (runtimeFiles.length === 0) throw new Error("ONNX Runtime Web WASM assets were not found.");
await Promise.all(runtimeFiles.map((name) => cp(join(wasmSource, name), join(wasmTarget, name))));
await mkdir(site, { recursive: true });
const buildOnly = new Set(["node_modules", "var", "dist", "site", "build-triage.mjs", "package.json", "package-lock.json"]);
for (const name of await readdir(frontend)) {
  if (buildOnly.has(name)) continue;
  await cp(join(frontend, name), join(site, name), { recursive: true, force: true });
}
process.stdout.write(`Built static site and copied ${runtimeFiles.length} WASM runtime files.\n`);
