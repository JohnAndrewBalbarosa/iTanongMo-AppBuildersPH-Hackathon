(function (root) {
  "use strict";

  function parseDraft(job) {
    if (job?.status !== "completed" || !job.output_hash) return null;
    try {
      const output = JSON.parse(job.output);
      return typeof output.body === "string" && output.body.trim() &&
        output.requires_human_review === true ? { job, output } : null;
    } catch { return null; }
  }

  // Mental model: select a completed draft, review edits, recheck its source, then publish explicitly.
  function createController(api, roles, view, onPublished) {
    let reference = "";
    let drafts = [];
    let selected = null;
    let revision = 0;
    let contextRevision = 0;
    let busy = false;
    const canPublish = roles.some((role) =>
      ["agency_admin", "decision_maker", "platform_admin"].includes(role));

    function validResponse() {
      const length = Array.from(view.body.value.trim()).length;
      return length >= 2 && length <= 5000;
    }

    function updateControls() {
      view.generate.disabled = busy || !reference;
      view.refresh.disabled = busy || !reference;
      view.select.disabled = busy || !drafts.length;
      view.body.disabled = busy || !selected;
      view.review.disabled = busy || !selected || !canPublish;
      view.publish.disabled = busy || !selected || !canPublish || !view.review.checked ||
        !validResponse();
    }

    function clear() {
      revision += 1;
      contextRevision += 1;
      reference = "";
      drafts = [];
      selected = null;
      view.facts.value = "";
      view.body.value = "";
      view.details.textContent = "";
      view.review.checked = false;
      view.options([]);
      view.status.textContent = "";
      updateControls();
    }

    function choose(id) {
      revision += 1;
      selected = drafts.find((draft) => draft.job.id === id) || null;
      view.body.value = selected?.output.body || "";
      view.review.checked = false;
      view.details.textContent = selected ? JSON.stringify({
        model: selected.job.model_id,
        source_record_ids: selected.output.source_record_ids || [],
        limitations: selected.output.limitations || [],
      }, null, 2) : "";
      updateControls();
    }

    function setCase(nextReference, detail) {
      if (reference !== nextReference) { clear(); reference = nextReference; }
      const previous = selected;
      drafts = (detail.ai_drafts || []).map(parseDraft).filter(Boolean);
      view.options(drafts.map((draft) => ({ id: draft.job.id,
        label: `${draft.output.subject || "Response draft"} · ${draft.job.created_at || ""}` })));
      const current = drafts.find((draft) => draft.job.id === previous?.job.id);
      if (current && current.job.output_hash === previous.job.output_hash &&
          current.job.output === previous.job.output) {
        selected = current;
        view.select.value = current.job.id;
      } else {
        choose(drafts[0]?.job.id);
        if (previous) view.status.textContent = "Nagbago ang source draft. Suriin muli bago mag-publish.";
      }
      if (!drafts.length) {
        const jobs = detail.ai_drafts || [];
        view.status.textContent = jobs.some((job) => ["pending", "running", "processing"].includes(job.status))
          ? "Nakapila ang AI draft. Pindutin ang Refresh drafts para tingnan ang resulta."
          : jobs.some((job) => job.status === "failed")
            ? "Hindi nagawa ang AI draft. Maaari kang gumawa ulit o mag-reply nang manu-mano."
            : "Wala pang completed AI draft para sa request na ito.";
      }
      updateControls();
    }

    async function refresh() {
      if (busy || !reference) return;
      const currentReference = reference;
      const currentContext = contextRevision;
      busy = true; updateControls();
      try {
        const detail = await api(`/government/requests/${encodeURIComponent(currentReference)}`);
        if (reference === currentReference && contextRevision === currentContext) setCase(currentReference, detail);
      } catch (error) {
        if (reference === currentReference && contextRevision === currentContext) view.status.textContent = error.message;
      } finally { busy = false; updateControls(); }
    }

    async function generate() {
      if (busy || !reference) return;
      const facts = view.facts.value.trim();
      if (facts.length < 20 || facts.length > 8000) {
        view.status.textContent = "Maglagay ng verified facts na 20–8,000 character.";
        return;
      }
      const currentReference = reference;
      const currentContext = contextRevision;
      busy = true; updateControls();
      view.status.textContent = "Ipinapila ang draft gamit ang shared backend AI…";
      try {
        await api(`/government/requests/${encodeURIComponent(currentReference)}/ai-draft`, {
          method: "POST", body: { verified_facts: facts },
        });
        if (reference === currentReference && contextRevision === currentContext) {
          view.status.textContent = "Nakapila ang AI draft. Pindutin ang Refresh drafts para tingnan ang resulta.";
        }
      } catch (error) {
        if (reference === currentReference && contextRevision === currentContext) view.status.textContent = error.message;
      } finally { busy = false; updateControls(); }
    }

    async function publish() {
      if (busy || !reference || !selected || !canPublish || !view.review.checked ||
          !validResponse()) return;
      const source = selected;
      const currentReference = reference;
      const currentRevision = revision;
      const currentContext = contextRevision;
      const editedBody = view.body.value;
      busy = true; updateControls();
      try {
        const detail = await api(`/government/requests/${encodeURIComponent(currentReference)}`);
        if (reference !== currentReference || revision !== currentRevision) return;
        const latest = (detail.ai_drafts || []).find((job) => job.id === source.job.id);
        if (!parseDraft(latest) || latest.output_hash !== source.job.output_hash ||
            latest.output !== source.job.output) {
          setCase(currentReference, detail);
          view.review.checked = false;
          view.status.textContent = "Nagbago ang source draft. Suriin muli bago mag-publish.";
          return;
        }
        await api(`/government/requests/${encodeURIComponent(currentReference)}/ai-drafts/${encodeURIComponent(source.job.id)}/publish`, {
          method: "POST", body: { source_output_hash: source.job.output_hash,
            edited_body: editedBody, factual_review_confirmed: "yes" },
        });
        if (reference === currentReference && contextRevision === currentContext) {
          choose(null);
          view.status.textContent = "Na-publish ang reviewed response sa citizen.";
          await onPublished(currentReference);
        }
      } catch (error) {
        if (reference === currentReference && contextRevision === currentContext) view.status.textContent = error.message;
      } finally { busy = false; updateControls(); }
    }

    view.generate.addEventListener("click", generate);
    view.refresh.addEventListener("click", refresh);
    view.select.addEventListener("change", () => choose(view.select.value));
    view.body.addEventListener("input", () => {
      revision += 1; view.review.checked = false; updateControls();
    });
    view.review.addEventListener("change", updateControls);
    view.publish.addEventListener("click", publish);
    clear();
    return { clear, setCase, generate, refresh, publish };
  }

  function initialize(api, roles, onPublished, document = root.document) {
    const stylesheet = document.createElement("link");
    stylesheet.rel = "stylesheet";
    stylesheet.href = "assets/government-ai.css";
    document.head.append(stylesheet);
    const panel = document.createElement("section");
    panel.className = "government-ai";
    panel.setAttribute("aria-label", "AI response draft");
    panel.innerHTML = `<h3>AI response draft</h3>
      <p>Shared backend AI ang gagawa ng draft. Suriin ang facts at sagot bago mag-publish.</p>
      <label>Verified facts<textarea data-ai="facts" rows="4" maxlength="8000" placeholder="Ilagay ang na-verify na impormasyon at record references."></textarea></label>
      <div class="government-ai-buttons"><button type="button" data-ai="generate">Generate AI draft</button><button type="button" data-ai="refresh">Refresh drafts</button></div>
      <label>Completed drafts<select data-ai="select"></select></label>
      <label>Review and edit response<textarea data-ai="body" rows="8" maxlength="5000" data-language-ignore></textarea></label>
      <details><summary>Sources and limitations</summary><pre data-ai="details" data-language-ignore></pre></details>
      <label class="government-ai-review"><input type="checkbox" data-ai="review"><span>Na-check ko ang facts, sources, at final response.</span></label>
      <p>Agency admin o decision maker ang maaaring mag-publish. Manual at explicit ang pagpapadala.</p>
      <button type="button" data-ai="publish">Publish reviewed response</button>
      <p data-ai="status" role="status" aria-live="polite"></p>`;
    const reply = document.getElementById("government-reply-form");
    reply.before(panel);
    const view = Object.fromEntries(["facts", "generate", "refresh", "select", "body", "details",
      "review", "publish", "status"].map((key) => [key, panel.querySelector(`[data-ai="${key}"]`)]));
    view.options = (items) => {
      view.select.replaceChildren(...items.map((item) => {
        const option = document.createElement("option");
        option.value = item.id; option.textContent = item.label;
        option.setAttribute("data-language-ignore", "");
        return option;
      }));
    };
    return createController(api, roles || [], view, onPublished);
  }

  const exports = { initialize, createController, parseDraft };
  if (typeof module !== "undefined" && module.exports) module.exports = exports;
  else root.iTanongMoGovernmentAi = exports;
})(typeof window !== "undefined" ? window : globalThis);
