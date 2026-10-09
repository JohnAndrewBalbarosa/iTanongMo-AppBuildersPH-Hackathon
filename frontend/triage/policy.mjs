import { TRIAGE_LABELS, TRIAGE_POLICY_VERSION } from "./label-map.v1.mjs";

export function reviewableSuggestions(result, ruleHits = []) {
  const scores = validatedScores(result);
  const categories = new Set(ruleOnlySuggestions(ruleHits));
  const rows = TRIAGE_LABELS.map((label) => {
    const score = scores.get(label.hypothesis);
    const suggested = score >= label.threshold;
    if (suggested) categories.add(label.category);
    return { ...label, score, suggested };
  });
  return { version: TRIAGE_POLICY_VERSION, categories: [...categories], rows };
}

export function ruleOnlySuggestions(ruleHits = []) {
  const knownCategories = new Set(TRIAGE_LABELS.map((label) => label.category));
  return [...new Set(ruleHits.map((hit) => hit.category).filter((category) => knownCategories.has(category)))];
}

function validatedScores(result) {
  if (!result || !Array.isArray(result.labels) || !Array.isArray(result.scores)) {
    throw new Error("Invalid classifier output.");
  }
  if (result.labels.length !== TRIAGE_LABELS.length || result.scores.length !== TRIAGE_LABELS.length) {
    throw new Error("Classifier output does not match the label map.");
  }
  const scores = new Map();
  for (let index = 0; index < result.labels.length; index += 1) {
    const label = result.labels[index];
    const score = result.scores[index];
    if (scores.has(label) || typeof score !== "number" || !Number.isFinite(score) || score < 0 || score > 1) {
      throw new Error("Classifier returned invalid scores.");
    }
    scores.set(label, score);
  }
  if (TRIAGE_LABELS.some((label) => !scores.has(label.hypothesis))) {
    throw new Error("Classifier returned unknown labels.");
  }
  return scores;
}
