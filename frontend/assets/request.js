(() => {
  "use strict";

  const apiBaseUrl = window.iTanongMo?.apiBaseUrl || "";
  const form = document.getElementById("foi-request-form");
  if (!form) return;
  const byId = (id) => document.getElementById(id);
  const selectedAgencies = new Map();
  let agencies = [];
  let aiDraft = null;
  let aiRevision = 0;

  initialize();
  byId("agency-search").addEventListener("input", renderAgencyOptions);
  byId("add-agency").addEventListener("click", addAgency);
  form.addEventListener("submit", submitRequest);
  byId("guest-ai-generate").addEventListener("click", generateAiDraft);
  byId("guest-ai-apply").addEventListener("click", applyAiDraft);
  for (const id of ["guest-ai-question", "guest-request-title-input"]) {
    byId(id).addEventListener("input", invalidateAiDraft);
  }

  // Mental model: account holders use the portal; guests choose agencies and receive a one-time receipt.
  async function initialize() {
    if (!apiBaseUrl) {
      byId("agency-status").textContent = "Hindi pa configured ang HTTPS Rust API.";
      return;
    }
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), 4000);
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/auth/session`, {
        credentials: "include", headers: { Accept: "application/json" }, signal: controller.signal,
      });
      if (response.ok && (await response.json()).data?.actor?.actor_type === "requester") {
        window.location.replace("citizen.html#citizen-request-form");
        return;
      }
    } catch {
      // A guest does not need an account session to submit.
    } finally {
      window.clearTimeout(timeout);
    }
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/config`, {
        credentials: "omit", headers: { Accept: "application/json" },
      });
      if (!response.ok) throw new Error("AI service is unavailable. You can continue manually.");
      const config = (await response.json()).data;
      if (config?.guest_ai_enabled === true) byId("guest-ai-generate").disabled = false;
      else byId("guest-ai-status").textContent = "AI drafting is unavailable. You can continue manually.";
    } catch {
      byId("guest-ai-status").textContent = "AI service is unavailable. You can continue manually.";
    }
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/agencies`, {
        credentials: "omit", headers: { Accept: "application/json" },
      });
      if (!response.ok) throw new Error("Unable to load departments. Try again later.");
      const data = (await response.json()).data;
      if (!Array.isArray(data)) throw new Error("Invalid department directory.");
      agencies = data.filter((agency) => agency.id && agency.name);
      if (!agencies.length) throw new Error("No departments are available.");
      byId("add-agency").disabled = false;
      byId("submit-request").disabled = false;
      renderAgencyOptions();
    } catch (error) {
      byId("agency-status").textContent = error.message;
    }
  }

  function invalidateAiDraft() {
    aiRevision += 1;
    aiDraft = null;
    byId("guest-ai-apply").disabled = true;
    byId("guest-ai-preview").textContent = "";
  }

  async function generateAiDraft() {
    invalidateAiDraft();
    const revision = aiRevision;
    const manualTitle = byId("guest-request-title-input").value.trim();
    const question = byId("guest-ai-question").value.trim();
    const status = byId("guest-ai-status");
    if (Array.from(manualTitle).length < 5 || Array.from(manualTitle).length > 180) {
      status.textContent = "Enter a title of 5–180 characters in Records requested first.";
      return;
    }
    if (Array.from(question).length < 20 || Array.from(question).length > 8000) {
      status.textContent = "Describe the records you need in 20–8,000 characters.";
      return;
    }
    const button = byId("guest-ai-generate");
    button.disabled = true;
    status.textContent = "Preparing your AI draft…";
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/requester-ai/guest-draft`, {
        method: "POST", credentials: "omit", cache: "no-store",
        headers: { Accept: "application/json", "Content-Type": "application/json" },
        body: JSON.stringify({ manual_title: manualTitle, what_to_ask: question }),
      });
      if (!response.ok) {
        const error = new Error(response.status === 429 ? "AI usage limit reached. Try again later." :
          "AI service is unavailable. You can continue manually.");
        error.status = response.status;
        throw error;
      }
      const draft = window.iTanongMoCitizenAi.validateDraft((await response.json()).data);
      if (revision !== aiRevision) {
        status.textContent = "Your inputs changed. Generate a new draft.";
        return;
      }
      aiDraft = draft;
      byId("guest-ai-preview").textContent = [draft.email_body, draft.description, draft.purpose,
        draft.urgency_details, `Suggested agencies: ${draft.agency_queries.join(", ")}`].join("\n\n");
      byId("guest-ai-apply").disabled = false;
      status.textContent = "Review the draft and suggested agencies before applying it.";
    } catch (error) {
      status.textContent = error.message;
    } finally {
      button.disabled = false;
    }
  }

  function applyAiDraft() {
    if (!aiDraft) return;
    byId("guest-request-description").value = aiDraft.description;
    byId("guest-request-purpose").value = aiDraft.purpose;
    byId("guest-request-urgency").value = aiDraft.urgency_details;
    byId("guest-ai-status").textContent = "Draft applied. Check your agency and details, then submit when ready.";
    byId("guest-request-description").focus();
  }

  function renderAgencyOptions() {
    const query = byId("agency-search").value.trim().toLocaleLowerCase();
    const matches = agencies.filter((agency) =>
      `${agency.short_name || ""} ${agency.name}`.toLocaleLowerCase().includes(query) &&
      !selectedAgencies.has(agency.id));
    const options = matches.map((agency) => {
      const option = document.createElement("option");
      option.value = agency.id;
      option.textContent = `${agency.short_name || agency.name} — ${agency.name}`;
      return option;
    });
    byId("agency-results").replaceChildren(...options);
    byId("add-agency").disabled = !options.length || selectedAgencies.size >= 10;
    byId("agency-status").textContent = `${matches.length} matching departments.`;
  }

  function addAgency() {
    const id = byId("agency-results").value;
    const agency = agencies.find((item) => item.id === id);
    if (!agency || selectedAgencies.size >= 10) return;
    selectedAgencies.set(id, agency);
    renderSelectedAgencies();
  }

  function renderSelectedAgencies() {
    const rows = [...selectedAgencies.values()].map((agency) => {
      const row = document.createElement("li");
      const remove = document.createElement("button");
      remove.type = "button";
      remove.textContent = "Remove";
      remove.addEventListener("click", () => {
        selectedAgencies.delete(agency.id);
        renderSelectedAgencies();
      });
      row.textContent = `${agency.short_name || agency.name} — ${agency.name} `;
      row.append(remove);
      return row;
    });
    byId("selected-agencies").replaceChildren(...rows);
    byId("agency-count").textContent = `${selectedAgencies.size} of 10 agencies selected.`;
    renderAgencyOptions();
  }

  async function submitRequest(event) {
    event.preventDefault();
    if (!selectedAgencies.size) {
      byId("submission-status").textContent = "Select at least one agency.";
      return;
    }
    const fields = new FormData(form);
    const payload = Object.fromEntries(fields.entries());
    payload.agency_ids = [...selectedAgencies.keys()].join(",");
    const submit = byId("submit-request");
    submit.disabled = true;
    byId("submission-status").textContent = "Submitting…";
    let accepted = false;
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/submissions`, {
        method: "POST", credentials: "omit", cache: "no-store",
        headers: { Accept: "application/json", "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (!response.ok) throw new Error(`Submission failed (${response.status}). Please retry.`);
      accepted = true;
      renderReceipt((await response.json()).data);
      byId("guest-request-section").hidden = true;
      byId("submission-output").focus();
    } catch (error) {
      byId("submission-status").textContent = accepted
        ? "The API accepted your request, but the receipt could not be shown. Contact support; do not submit it again."
        : error.message;
      if (accepted) form.hidden = true;
      else submit.disabled = false;
    }
  }

  function renderReceipt(data) {
    if (!data?.batch_reference || !data.requests?.length ||
        data.requests.some((request) => !request.reference || !request.agency)) {
      throw new Error("Submission response is incomplete. Contact support before retrying.");
    }
    byId("batch-reference").textContent = data.batch_reference;
    const links = data.requests.map((request) => {
      const row = document.createElement("li");
      const link = document.createElement("a");
      link.href = `track.html?reference=${encodeURIComponent(request.reference)}`;
      link.textContent = `${request.agency}: ${request.reference}`;
      row.append(link);
      return row;
    });
    byId("submission-results").replaceChildren(...links);
    byId("submission-output").hidden = false;
  }
})();
