(() => {
  "use strict";

  const apiBaseUrl = window.iTanongMo?.apiBaseUrl || "";
  const byId = (id) => document.getElementById(id);
  let csrfToken = "";
  let currentReference = "";
  let currentAiDrafts = [];

  async function apiFetch(path, options = {}) {
    if (!apiBaseUrl) throw new Error("Kailangan ang public HTTPS Rust API URL para buksan ang government workspace.");
    const response = await fetch(`${apiBaseUrl}${path}`, { credentials: "include", ...options });
    if (!response.ok) {
      const body = await response.text();
      throw new Error(body || `API request failed (${response.status}).`);
    }
    return response.headers.get("content-type")?.includes("json") ? response.json() : response;
  }

  function jsonRequest(method, body) {
    return { method, headers: { "content-type": "application/json" }, body: JSON.stringify(body) };
  }

  function option(value, label) {
    const item = document.createElement("option");
    item.value = value;
    item.textContent = label;
    return item;
  }

  function rememberSession(session) {
    csrfToken = session.csrf_token;
    byId("actor-name").textContent = `${session.actor.display_name} — ${session.actor.roles.join(", ")}`;
    byId("persona-section").hidden = true;
    byId("workspace").hidden = false;
  }

  async function loadPersonas() {
    const result = await apiFetch("/api/v1/government/personas");
    const list = byId("persona-list");
    list.replaceChildren(...result.data.map((persona) => option(persona.id, `${persona.display_name} — ${persona.role}`)));
  }

  async function selectPersona() {
    const result = await apiFetch(
      "/api/v1/government/session",
      jsonRequest("POST", { persona_id: byId("persona-list").value }),
    );
    rememberSession(result.data);
    await loadBoard();
  }

  async function loadBoard() {
    const result = await apiFetch("/api/v1/government/board");
    rememberSession(result.data.session);
    const board = byId("government-board");
    board.replaceChildren();
    const columns = new Map();
    for (const card of result.data.cards) {
      if (!columns.has(card.column)) columns.set(card.column, []);
      columns.get(card.column).push(card);
    }
    for (const [name, cards] of columns) {
      const section = document.createElement("section");
      const heading = document.createElement("h3");
      heading.textContent = name;
      section.append(heading);
      for (const card of cards) {
        const button = document.createElement("button");
        button.type = "button";
        button.textContent = `${card.reference} — ${card.title}`;
        button.addEventListener("click", () => openRequest(card.reference));
        section.append(button);
      }
      board.append(section);
    }
    await loadNotifications();
  }

  async function loadNotifications() {
    const result = await apiFetch("/api/v1/notifications");
    const list = byId("government-notifications");
    list.replaceChildren();
    for (const notification of result.data) {
      const item = document.createElement("li");
      item.textContent = `${notification.created_at} — ${notification.message}`;
      list.append(item);
    }
  }

  async function downloadCsv() {
    const response = await apiFetch("/api/v1/government/reports/requests.csv");
    const link = document.createElement("a");
    link.href = URL.createObjectURL(await response.blob());
    link.download = "foi-requests.csv";
    link.click();
    URL.revokeObjectURL(link.href);
  }

  function fillUnitSelect(id, units) {
    byId(id).replaceChildren(option("", "Unassigned"), ...units.map((unit) => option(unit.id, unit.name)));
  }

  async function openRequest(reference) {
    const result = await apiFetch(`/api/v1/government/requests/${encodeURIComponent(reference)}`);
    const data = result.data;
    currentReference = reference;
    rememberSession(data.session);
    byId("government-request").hidden = false;
    byId("government-request-title").textContent = `${reference} — ${data.request.title}`;
    byId("government-request-summary").textContent = `${data.request.status}\nDue: ${data.request.extended_due_at || data.request.due_at}`;
    byId("workflow-action").replaceChildren(...data.actions.map((action) => option(action, action)));
    fillUnitSelect("workflow-unit", data.units);
    fillUnitSelect("triage-unit", data.units);
    currentAiDrafts = data.ai_drafts.filter((draft) => draft.status === "completed" && draft.output_hash && draft.output);
    byId("ai-draft-list").replaceChildren(
      option("", "Select a completed draft"),
      ...currentAiDrafts.map((draft) => option(draft.id, `${draft.created_at} — ${draft.model_id || "model"}`)),
    );
    byId("ai-reviewed-body").value = "";
    byId("ai-factual-review").checked = false;
    byId("government-request-data").textContent = JSON.stringify({
      request: data.request,
      events: data.events,
      messages: data.messages,
      documents: data.documents,
      triage: data.triage,
      ai_drafts: data.ai_drafts,
    }, null, 2);
  }

  async function mutateRequest(suffix, body) {
    if (!currentReference) throw new Error("Pumili muna ng request.");
    await apiFetch(
      `/api/v1/government/requests/${encodeURIComponent(currentReference)}${suffix}`,
      jsonRequest("POST", { csrf_token: csrfToken, ...body }),
    );
    await openRequest(currentReference);
    await loadBoard();
  }

  async function releaseDocument() {
    if (!currentReference) throw new Error("Pumili muna ng request.");
    const file = byId("document-file").files[0];
    if (!file) throw new Error("Pumili ng document.");
    const body = new FormData();
    body.append("csrf_token", csrfToken);
    body.append("title", byId("document-title").value);
    body.append("file", file);
    await apiFetch(`/api/v1/government/requests/${encodeURIComponent(currentReference)}/documents`, { method: "POST", body });
    await openRequest(currentReference);
  }

  function loadSelectedAiDraft() {
    const draft = currentAiDrafts.find((item) => item.id === byId("ai-draft-list").value);
    if (!draft) {
      byId("ai-reviewed-body").value = "";
      return;
    }
    try {
      byId("ai-reviewed-body").value = JSON.parse(draft.output).body || "";
    } catch {
      throw new Error("Invalid ang stored AI draft JSON; huwag itong i-publish.");
    }
  }

  async function publishAiDraft() {
    const draft = currentAiDrafts.find((item) => item.id === byId("ai-draft-list").value);
    if (!draft) throw new Error("Pumili ng completed AI draft.");
    if (!byId("ai-factual-review").checked) throw new Error("Kumpirmahin muna ang factual review.");
    await mutateRequest(`/ai-drafts/${encodeURIComponent(draft.id)}/publish`, {
      source_output_hash: draft.output_hash,
      edited_body: byId("ai-reviewed-body").value,
      factual_review_confirmed: "yes",
    });
  }

  async function run(operation) {
    try {
      byId("government-status").textContent = "";
      await operation();
    } catch (error) {
      byId("government-status").textContent = error.message;
    }
  }

  byId("select-persona").addEventListener("click", () => run(selectPersona));
  byId("refresh-notifications").addEventListener("click", () => run(loadNotifications));
  byId("download-csv").addEventListener("click", () => run(downloadCsv));
  byId("apply-workflow-action").addEventListener("click", () => run(() => mutateRequest("/action", {
    action: byId("workflow-action").value,
    destination_unit_id: byId("workflow-unit").value,
    note: byId("workflow-note").value,
  })));
  byId("save-government-message").addEventListener("click", () => run(() => mutateRequest("/messages", {
    visibility: byId("message-visibility").value,
    body: byId("government-message").value,
  })));
  byId("save-triage").addEventListener("click", () => run(() => mutateRequest("/triage", {
    categories: byId("triage-categories").value,
    destination_unit_id: byId("triage-unit").value,
  })));
  byId("queue-ai-draft").addEventListener("click", () => run(() => mutateRequest("/ai-draft", {
    verified_facts: byId("verified-facts").value,
  })));
  byId("ai-draft-list").addEventListener("change", () => run(loadSelectedAiDraft));
  byId("publish-ai-draft").addEventListener("click", () => run(publishAiDraft));
  byId("release-document").addEventListener("click", () => run(releaseDocument));

  run(async () => {
    if (!apiBaseUrl) throw new Error("I-set ang HTTPS Rust API URL sa GitHub repository variable ITANONGMO_API_BASE_URL.");
    try {
      const session = await apiFetch("/api/v1/government/session");
      rememberSession(session.data);
      await loadBoard();
    } catch {
      await loadPersonas();
    }
  });
})();
