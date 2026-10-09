(() => {
  "use strict";

  const apiBaseUrl = window.iTanongMo?.apiBaseUrl || "";
  const form = document.querySelector("#foi-request-form");
  if (!form) return;

  const byId = (id) => document.getElementById(id);
  const selectedAgencies = new Map();
  const credentialTtlMilliseconds = 15 * 60 * 1000;
  let allAgencies = [];
  let temporaryCredential = null;
  let generatedFormValues = null;
  let visibleAgencies = [];
  let searchController = null;
  let searchTimer = null;

  function apiUrl(path) {
    return `${apiBaseUrl}${path}`;
  }

  async function apiFetch(path, options = {}) {
    if (!apiBaseUrl) throw new Error("Hindi pa configured ang HTTPS Rust API.");
    const response = await fetch(apiUrl(path), { credentials: "omit", ...options });
    if (!response.ok) {
      const message = await response.text();
      throw new Error(message || `API request failed (${response.status}).`);
    }
    return response.json();
  }

  function openDialog(id) {
    const dialog = byId(id);
    if (dialog && !dialog.open) dialog.showModal();
  }

  function renderSelectedAgencies() {
    const container = byId("selected-agencies");
    container.replaceChildren();
    for (const agency of selectedAgencies.values()) {
      const row = document.createElement("div");
      const label = document.createElement("span");
      label.textContent = `${agency.short_name} — ${agency.name}${agency.office_email ? ` — ${agency.office_email}` : ""} `;
      const remove = document.createElement("button");
      remove.type = "button";
      remove.textContent = "Remove";
      remove.addEventListener("click", () => {
        selectedAgencies.delete(agency.id);
        renderSelectedAgencies();
        renderAgencyMatches(visibleAgencies);
      });
      row.append(label, remove);
      container.append(row);
    }
    byId("agency-count").textContent = `${selectedAgencies.size} of 10 agencies selected.`;
  }

  function renderAgencyMatches(agencies) {
    visibleAgencies = agencies;
    const options = agencies.map((agency) => {
      const item = document.createElement("option");
      item.value = agency.id;
      item.textContent = `${agency.short_name} — ${agency.name}`;
      item.disabled = selectedAgencies.has(agency.id);
      return item;
    });
    if (options.length === 0) {
      const empty = document.createElement("option");
      empty.textContent = "No matching department";
      empty.disabled = true;
      options.push(empty);
    }
    byId("agency-results").replaceChildren(...options);
    byId("agency-status").textContent = `${agencies.length} matching departments.`;
  }

  function addSelectedAgency() {
    if (selectedAgencies.size >= 10) {
      byId("agency-status").textContent = "Maximum of 10 departments per submission.";
      return;
    }
    const id = byId("agency-results").value;
    const agency = visibleAgencies.find((item) => item.id === id);
    if (!agency || selectedAgencies.has(id)) return;
    selectedAgencies.set(id, agency);
    renderSelectedAgencies();
    renderAgencyMatches(visibleAgencies);
  }

  function searchStaticAgencies(query) {
    const terms = query.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
    if (terms.length === 0) return allAgencies;
    return allAgencies.filter((agency) => {
      const searchableName = `${agency.short_name} ${agency.name}`.toLocaleLowerCase();
      return terms.every((term) => searchableName.includes(term));
    });
  }

  async function loadAgencyDirectory() {
    const response = await fetch("agencies.json", { cache: "force-cache", credentials: "same-origin" });
    if (!response.ok) throw new Error("Hindi ma-load ang agency directory.");
    const agencies = await response.json();
    if (!Array.isArray(agencies)) throw new Error("Invalid ang agency directory.");
    return agencies;
  }

  async function searchAgencies(query) {
    if (!apiBaseUrl) return searchStaticAgencies(query);
    if (searchController) searchController.abort();
    searchController = new AbortController();
    try {
      const result = await apiFetch(`/api/v1/agencies/search?q=${encodeURIComponent(query)}`, {
        signal: searchController.signal,
      });
      return result.data;
    } catch (error) {
      if (error.name === "AbortError") throw error;
      return searchStaticAgencies(query);
    }
  }

  async function loadConfiguration() {
    try {
      allAgencies = await loadAgencyDirectory();
      renderAgencyMatches(allAgencies);
    } catch (error) {
      byId("agency-status").textContent = error.message;
    }
    if (!apiBaseUrl) {
      form.querySelector("button[type=submit]").disabled = true;
      return;
    }
    try {
      const result = await apiFetch("/api/v1/config");
      renderAgencyMatches(await searchAgencies(""));
    } catch (error) {
      form.querySelector("button[type=submit]").disabled = true;
    }
  }

  function forgetCredential(message = "API key removed from this tab's memory.") {
    temporaryCredential = null;
    byId("ai-token").value = "";
    byId("api-key-status").textContent = message;
    byId("ai-status").textContent = message;
  }

  function credential() {
    if (temporaryCredential && Date.now() < temporaryCredential.expiresAt) {
      return temporaryCredential.value;
    }
    forgetCredential("Insert an API key before generating a draft.");
    throw new Error("Insert an API key before generating a draft.");
  }

  function rememberCredential() {
    const value = byId("ai-token").value.trim();
    if (value.length < 10 || value.length > 512 || /\s/.test(value)) throw new Error("Enter a valid Gemini API key.");
    temporaryCredential = {
      value,
      expiresAt: Date.now() + credentialTtlMilliseconds,
    };
    byId("ai-token").value = "";
    byId("api-key-status").textContent = "API key is ready in this tab for up to 15 minutes.";
    window.setTimeout(() => {
      if (temporaryCredential && Date.now() >= temporaryCredential.expiresAt) forgetCredential();
    }, credentialTtlMilliseconds + 250);
  }

  async function pasteCredential() {
    if (!window.isSecureContext || !navigator.clipboard?.readText) {
      throw new Error("Clipboard paste is unavailable. Paste the key manually into the password field.");
    }
    const value = (await navigator.clipboard.readText()).trim();
    if (!value) throw new Error("The clipboard does not contain an API key.");
    byId("ai-token").value = value;
    byId("api-key-status").textContent = "API key pasted. Select “Use for 15 minutes” to continue.";
  }

  function draftInput() {
    return {
      what_to_ask: byId("ai-details").value.trim(),
      manual_title: form.elements.title.value.trim(),
    };
  }

  async function generateDraft(event) {
    event.preventDefault();
    generatedFormValues = null;
    byId("ai-results").hidden = true;
    if (!window.isSecureContext) {
      byId("ai-status").textContent = "Open this page over HTTPS to use Gemini drafting.";
      return;
    }
    try {
      byId("ai-status").textContent = "Generating one strict JSON draft…";
      const token = credential();
      generatedFormValues = await window.iTanongMoGeminiDraft.generateDraft(token, draftInput());
      byId("ai-email-output").textContent = generatedFormValues.email_body;
      byId("ai-json-output").textContent = JSON.stringify(generatedFormValues, null, 2);
      byId("ai-results").hidden = false;
      byId("ai-status").textContent = "Review the JSON, departments, and email preview before applying.";
    } catch (error) {
      byId("ai-status").textContent = error.message;
    }
  }

  async function applyDraft() {
    if (!generatedFormValues) return;
    const manualName = form.elements.requester_name.value;
    const manualTitle = form.elements.title.value;
    for (const field of ["description", "purpose", "urgency_details"]) {
      form.elements[field].value = generatedFormValues[field] || "";
    }
    for (const query of generatedFormValues.agency_queries || []) {
      const matches = await searchAgencies(query);
      if (matches[0] && selectedAgencies.size < 10) selectedAgencies.set(matches[0].id, matches[0]);
    }
    form.elements.requester_name.value = manualName;
    form.elements.title.value = manualTitle;
    renderSelectedAgencies();
    renderAgencyMatches(visibleAgencies);
    byId("ai-status").textContent = "Applied for review. Name and title were preserved; verify every field and department.";
  }

  function downloadDraftJson() {
    if (!generatedFormValues) return;
    const content = new Blob([`${JSON.stringify(generatedFormValues, null, 2)}\n`], { type: "application/json" });
    const link = document.createElement("a");
    link.href = URL.createObjectURL(content);
    link.download = "itanongmo-ai-draft.json";
    link.click();
    URL.revokeObjectURL(link.href);
  }

  async function submitRequest(event) {
    event.preventDefault();
    if (selectedAgencies.size === 0) {
      byId("submission-status").textContent = "Select at least one agency.";
      return;
    }
    const values = new FormData(form);
    const payload = Object.fromEntries(values.entries());
    payload.agency_ids = [...selectedAgencies.keys()].join(",");
    try {
      byId("submission-status").textContent = "Submitting…";
      const result = await apiFetch("/api/v1/submissions", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify(payload),
      });
      const data = result.data;
      byId("submission-status").textContent = `Submitted batch ${data.batch_reference}.`;
      byId("requester-access-token").textContent = data.requester_access_token;
      const list = byId("submission-results");
      list.replaceChildren();
      for (const request of data.requests) {
        sessionStorage.setItem(`itanongmo.requester-access.${request.reference}`, data.requester_access_token);
        const item = document.createElement("li");
        const link = document.createElement("a");
        link.href = `track.html?reference=${encodeURIComponent(request.reference)}`;
        link.textContent = `${request.agency}: ${request.reference}`;
        item.append(link);
        list.append(item);
      }
      byId("submission-output").hidden = false;
    } catch (error) {
      byId("submission-status").textContent = error.message;
    }
  }

  byId("open-ai-information").addEventListener("click", () => openDialog("ai-information-dialog"));
  byId("open-ai-draft").addEventListener("click", () => openDialog("ai-draft-dialog"));
  byId("open-api-key").addEventListener("click", () => openDialog("api-key-dialog"));
  byId("change-ai-token").addEventListener("click", () => openDialog("api-key-dialog"));
  byId("continue-to-ai").addEventListener("click", () => {
    byId("ai-information-dialog").close();
    openDialog("ai-draft-dialog");
  });
  document.querySelectorAll("[data-close-dialog]").forEach((button) => {
    button.addEventListener("click", () => byId(button.dataset.closeDialog).close());
  });
  byId("forget-ai-token").addEventListener("click", () => forgetCredential());
  byId("remember-ai-token").addEventListener("click", () => {
    try {
      rememberCredential();
    } catch (error) {
      byId("api-key-status").textContent = error.message;
    }
  });
  byId("paste-ai-token").addEventListener("click", async () => {
    try {
      await pasteCredential();
    } catch (error) {
      byId("api-key-status").textContent = error.message;
    }
  });
  byId("ai-draft-form").addEventListener("submit", generateDraft);
  byId("apply-ai-values").addEventListener("click", applyDraft);
  byId("download-ai-json").addEventListener("click", downloadDraftJson);
  byId("add-agency").addEventListener("click", addSelectedAgency);
  byId("agency-search").addEventListener("input", (event) => {
    window.clearTimeout(searchTimer);
    searchTimer = window.setTimeout(async () => {
      try {
        renderAgencyMatches(await searchAgencies(event.target.value));
      } catch (error) {
        if (error.name !== "AbortError") byId("agency-status").textContent = error.message;
      }
    }, 120);
  });
  form.addEventListener("submit", submitRequest);
  window.addEventListener("pagehide", () => { temporaryCredential = null; });
  loadConfiguration();
})();
