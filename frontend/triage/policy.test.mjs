import assert from "node:assert/strict";
import test from "node:test";
import { TRIAGE_LABELS } from "./label-map.v1.mjs";
import { reviewableSuggestions, ruleOnlySuggestions } from "./policy.mjs";

const output = (overrides = {}) => ({
  labels: TRIAGE_LABELS.map((label) => label.hypothesis),
  scores: TRIAGE_LABELS.map((label) => overrides[label.category] ?? 0.1),
});

test("multi-label scores and rule hits produce reviewable suggestions", () => {
  const result = reviewableSuggestions(output({ time_sensitive: 0.9, sensitive_data: 0.8 }), [
    { category: "records_preservation" },
  ]);
  assert.deepEqual(result.categories, ["records_preservation", "time_sensitive", "sensitive_data"]);
  assert.equal(result.rows.find((row) => row.category === "time_sensitive").suggested, true);
});

test("threshold boundary is inclusive and lower scores stay advisory", () => {
  const result = reviewableSuggestions(output({ clarification: 0.4, appeal: 0.399 }));
  assert.deepEqual(result.categories, ["clarification"]);
});

test("invalid model scores are rejected without hiding rule hits", () => {
  assert.throws(() => reviewableSuggestions({ labels: ["unknown"], scores: [0.9] }));
  assert.throws(() => reviewableSuggestions(output({ appeal: Number.NaN })));
  assert.deepEqual(ruleOnlySuggestions([{ category: "appeal" }, { category: "appeal" }]), ["appeal"]);
});
