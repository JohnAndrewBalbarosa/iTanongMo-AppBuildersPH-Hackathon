(function (root) {
  "use strict";

  const fields = ["email_body", "description", "purpose", "urgency_details", "agency_queries"];

  function initialize(api, document = root.document) {
    const byId = (id) => document.getElementById(id);
    const form = byId("citizen-request-form");
    const question = byId("citizen-ai-question");
    const title = byId("citizen-request-title");
    const generate = byId("citizen-ai-generate");
    const apply = byId("citizen-ai-apply");
    const status = byId("citizen-ai-status");
    const preview = byId("citizen-ai-preview");
    let draft = null;
    let revision = 0;
    let generating = false;

    function invalidateDraft() {
      revision += 1;
      draft = null;
      apply.disabled = true;
      preview.textContent = "";
    }

    // Mental model: ask once, preview validated text, and apply only after the citizen chooses.
    async function generateDraft() {
      if (generating) return;
      invalidateDraft();
      const currentRevision = revision;
      generating = true;
      generate.disabled = true;
      status.textContent = "Preparing your AI draft…";
      try {
        const input = { what_to_ask: question.value.trim(), manual_title: title.value.trim() };
        if (Array.from(input.manual_title).length < 5 || Array.from(input.manual_title).length > 180) {
          throw new Error("Enter the manual request title first.");
        }
        if (Array.from(input.what_to_ask).length < 20 || Array.from(input.what_to_ask).length > 8000) {
          throw new Error("Describe the records you need in 20–8,000 characters.");
        }
        const result = validateDraft(await api("/requester-ai/draft", { method: "POST", body: input }));
        if (revision !== currentRevision) {
          status.textContent = "Your inputs changed. Generate a new draft.";
          return;
        }
        draft = result;
        preview.textContent = `${draft.email_body}\n\n${draft.description}\n\n${draft.purpose}\n\n${draft.urgency_details}\n\n${draft.agency_queries.join(", ")}`;
        apply.disabled = false;
        status.textContent = "Review the draft and suggested agencies before applying it.";
      } catch (error) {
        status.textContent = error.status === 503
          ? "AI service is temporarily unavailable. You can continue manually."
          : error.status === 429
            ? "AI usage limit reached. Try again later or continue manually."
            : error.message;
      } finally {
        generating = false;
        generate.disabled = false;
      }
    }

    function applyDraft() {
      if (!draft) return;
      byId("citizen-request-details").value = draft.description;
      byId("citizen-request-purpose").value = draft.purpose;
      byId("citizen-request-urgency").value = draft.urgency_details;
      status.textContent = "Draft applied. Check your agency and details, then submit when ready.";
      byId("citizen-request-details").focus();
    }

    generate.addEventListener("click", generateDraft);
    apply.addEventListener("click", applyDraft);
    question.addEventListener("input", invalidateDraft);
    title.addEventListener("input", invalidateDraft);
    form.addEventListener("reset", () => { invalidateDraft(); status.textContent = ""; });
  }

  function validateDraft(value) {
    if (!value || typeof value !== "object" || Array.isArray(value) ||
        Object.keys(value).length !== fields.length ||
        Object.keys(value).some((key) => !fields.includes(key))) {
      throw new Error("AI returned an invalid JSON draft.");
    }
    for (const [name, minimum, maximum] of [
      ["email_body", 20, 12000], ["description", 20, 8000],
      ["purpose", 3, 1000], ["urgency_details", 0, 1000],
    ]) {
      if (typeof value[name] !== "string" || Array.from(value[name].trim()).length < minimum ||
          Array.from(value[name]).length > maximum) throw new Error("AI returned invalid form values.");
    }
    if (!Array.isArray(value.agency_queries) || value.agency_queries.length < 1 ||
        value.agency_queries.length > 10 || value.agency_queries.some((query) =>
          typeof query !== "string" || Array.from(query.trim()).length < 2 || Array.from(query).length > 120)) {
      throw new Error("AI returned invalid agency suggestions.");
    }
    return value;
  }

  const api = Object.freeze({ initialize, validateDraft });
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else root.iTanongMoCitizenAi = api;
})(typeof window !== "undefined" ? window : globalThis);
