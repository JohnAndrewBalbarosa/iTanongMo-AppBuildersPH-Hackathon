import { env, pipeline } from "@huggingface/transformers";
import { MODEL_ID, MODEL_REVISION, TRIAGE_LABELS } from "./label-map.v1.mjs";

env.backends.onnx.wasm.wasmPaths = new URL("./wasm/", import.meta.url).href;
env.allowLocalModels = false;

let classifierPromise;

self.onmessage = async ({ data }) => {
  if (data?.type !== "classify" || !Number.isSafeInteger(data.id) || typeof data.text !== "string") return;
  try {
    const classifier = await loadClassifier();
    const result = await classifier(data.text, TRIAGE_LABELS.map((label) => label.hypothesis), {
      multi_label: true,
      hypothesis_template: "This FOI request concerns {}.",
    });
    self.postMessage({ type: "result", id: data.id, labels: result.labels, scores: result.scores });
  } catch (error) {
    // Diagnostic data excludes request text and model payloads.
    console.error(JSON.stringify({ event: "triage.classifier_failed", component: "triage-worker", error: String(error?.name || "Error") }));
    self.postMessage({ type: "error", id: data.id, message: "Local classifier unavailable; review the existing rule hits manually." });
  }
};

function loadClassifier() {
  classifierPromise ??= pipeline("zero-shot-classification", MODEL_ID, {
    revision: MODEL_REVISION,
    dtype: "q8",
    device: "wasm",
  }).catch((error) => {
    classifierPromise = undefined;
    throw error;
  });
  return classifierPromise;
}
