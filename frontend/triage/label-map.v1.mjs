export const TRIAGE_POLICY_VERSION = "foi-triage-v1-pilot";
export const MODEL_ID = "onnx-community/multilingual-MiniLMv2-L6-mnli-xnli-ONNX";
export const MODEL_REVISION = "ca5daf3d11b6c4b3143b1f4602a2edfb64c3ad7e";

// Pilot thresholds favor recall; calibrate against labeled Filipino,
// English, and Taglish FOI requests before operational use.
export const TRIAGE_LABELS = Object.freeze([
  { category: "time_sensitive", hypothesis: "an urgent or time-sensitive request", threshold: 0.35 },
  { category: "safety_review", hypothesis: "a risk to safety or public health", threshold: 0.35 },
  { category: "sensitive_data", hypothesis: "personal or sensitive information", threshold: 0.35 },
  { category: "voluminous", hypothesis: "a large volume of records", threshold: 0.40 },
  { category: "multi_unit", hypothesis: "records from multiple government units", threshold: 0.40 },
  { category: "appeal", hypothesis: "an appeal or reconsideration of a decision", threshold: 0.40 },
  { category: "records_preservation", hypothesis: "preservation of records at risk of deletion", threshold: 0.35 },
  { category: "clarification", hypothesis: "a request that needs clarification", threshold: 0.40 },
]);
