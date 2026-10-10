(() => {
  "use strict";

  const apiBaseUrl = String(window.ITANONGMO_CONFIG?.apiBaseUrl || "").replace(/\/$/, "");
  const page = document.body.dataset.portalPage;
  let session = null;
  let csrfToken = "";
  let requests = [];
  let selectedReference = "";
  let caseDetail = null;
  let governmentDrafting = null;
  let governmentBoardRevision = 0;

  initializePortal();

  // Mental model: restore the server session, then load only data allowed for its role.
  async function initializePortal() {
    if (page === "login") {
      byId("portal-sign-in").addEventListener("submit", signIn);
      return;
    }
    if (!apiBaseUrl) {
      showPageError("The API is not configured. Set ITANONGMO_API_BASE_URL for this deployment.");
      return;
    }
    try {
      session = await api("/auth/session");
      csrfToken = session.csrf_token;
      const isCitizen = session.actor.actor_type === "requester";
      if ((page === "citizen") !== isCitizen) {
        window.location.replace(isCitizen ? "citizen.html" : "government-demo.html");
        return;
      }
      byId("portal-logout").addEventListener("click", signOut);
      if (isCitizen) await initializeCitizenPortal();
      else await initializeGovernmentPortal();
    } catch (error) {
      if (error.status === 401) {
        window.location.replace("login.html");
        return;
      }
      showPageError(error.message);
    }
  }

  async function signIn(event) {
    event.preventDefault();
    const form = event.currentTarget;
    const status = byId("sign-in-status");
    status.textContent = "Checking your account…";
    try {
      const data = await api("/auth/login", {
        method: "POST",
        body: {
          email: form.elements.email.value.trim(),
          password: form.elements.password.value,
        },
      });
      csrfToken = data.csrf_token;
      window.location.assign(data.actor.actor_type === "requester" ? "citizen.html" : "government-demo.html");
    } catch (error) {
      status.textContent = error.message;
    } finally {
      form.elements.password.value = "";
    }
  }

  async function signOut() {
    try {
      await api("/auth/logout", { method: "POST", body: {} });
      window.location.replace("login.html");
    } catch (error) {
      showPageError(error.message);
    }
  }

  async function initializeCitizenPortal() {
    window.iTanongMoCitizenAi.initialize(api);
    const userName = document.querySelector("[data-user-name]");
    userName.setAttribute("data-language-ignore", "");
    userName.textContent = session.actor.display_name;
    byId("citizen-request-form").addEventListener("submit", submitCitizenRequest);
    byId("citizen-refresh").addEventListener("click", () => loadCitizenRequests().catch(showPageError));
    const agencies = await api("/agencies");
    const select = byId("citizen-agency");
    select.replaceChildren(...agencies.map((agency) => option(agency.id, agency.name)));
    await loadCitizenRequests();
  }

  async function loadCitizenRequests(preferredReference = selectedReference) {
    const data = await api("/citizen/requests");
    requests = data.requests;
    selectedReference = requests.some((request) => request.reference === preferredReference)
      ? preferredReference : (requests[0]?.reference || "");
    byId("citizen-total").textContent = String(requests.length);
    byId("citizen-active").textContent = String(requests.filter((request) =>
      !["closed", "granted", "partially_granted", "denied"].includes(request.status)).length);
    byId("citizen-responses").textContent = String(requests.filter((request) =>
      request.latest_public_message).length);
    renderCitizenList();
    await renderCitizenDetail();
  }

  function renderCitizenList() {
    const list = byId("citizen-request-list");
    list.replaceChildren();
    for (const request of requests) {
      const button = element("button", "", "request-list-item");
      button.type = "button";
      if (request.reference === selectedReference) button.setAttribute("aria-current", "true");
      button.append(
        serverText("strong", request.title),
        element("span", `${request.reference} · ${request.agency.name}`),
        statusBadge(request.status),
      );
      button.addEventListener("click", () => {
        selectedReference = request.reference;
        renderCitizenList();
        renderCitizenDetail().catch(showPageError);
      });
      list.append(button);
    }
  }

  async function renderCitizenDetail() {
    const detail = byId("citizen-request-detail");
    detail.replaceChildren();
    if (!selectedReference) {
      detail.append(element("p", "Submit a request to begin tracking."));
      return;
    }
    const data = await api(`/citizen/requests/${encodeURIComponent(selectedReference)}`);
    const request = data.request;
    detail.append(
      element("h3", request.reference),
      statusBadge(request.status),
      serverText("h4", request.title),
      element("p", `${request.agency.name} · Due ${formatDate(request.due_at)}`),
      serverText("p", request.description),
    );
    const timeline = element("div", "", "case-timeline");
    const updates = [
      ...data.timeline.events.map((event) => ({
        label: event.type.replaceAll("_", " "),
        detail: event.summary,
        time: event.created_at,
      })),
      ...data.timeline.messages.map((message) => ({
        label: message.actor,
        detail: message.body,
        time: message.created_at,
      })),
    ].sort((left, right) => Date.parse(left.time) - Date.parse(right.time));
    for (const update of updates) timeline.append(timelineItem(update));
    detail.append(timeline);
  }

  async function submitCitizenRequest(event) {
    event.preventDefault();
    const form = event.currentTarget;
    const status = byId("citizen-submit-status");
    status.textContent = "Submitting your request…";
    try {
      const fields = new FormData(form);
      const data = await api("/submissions", {
        method: "POST",
        body: {
          agency_ids: String(fields.get("agency_ids") || ""),
          requester_name: session.actor.display_name,
          requester_email: String(fields.get("requester_email") || "").trim(),
          title: String(fields.get("title") || "").trim(),
          description: String(fields.get("description") || "").trim(),
          purpose: String(fields.get("purpose") || "").trim(),
          urgency_details: String(fields.get("urgency_details") || "").trim(),
          consent: fields.get("consent") === "yes" ? "yes" : "",
        },
      });
      const reference = data.requests[0]?.reference;
      if (!reference) throw new Error("The API returned no request reference.");
      form.reset();
      status.textContent = `Submitted ${reference}. Your batch reference is ${data.batch_reference}.`;
      await loadCitizenRequests(reference);
    } catch (error) {
      status.textContent = error.message;
    }
  }

  async function initializeGovernmentPortal() {
    try {
      await import("./government-ai.js");
      governmentDrafting = window.iTanongMoGovernmentAi.initialize(api, session.actor.roles,
        (reference) => loadGovernmentBoard(reference));
    } catch (error) {
      showPageError("AI controls could not load. You can still reply manually.");
    }
    byId("triage-form").addEventListener("submit", applyTriage);
    byId("government-reply-form").addEventListener("submit", sendGovernmentReply);
    byId("route-case").addEventListener("click", routeCase);
    byId("government-refresh").addEventListener("click", () => loadGovernmentBoard().catch(showPageError));
    await loadGovernmentBoard();
  }

  async function loadGovernmentBoard(preferredReference = selectedReference) {
    const boardRevision = ++governmentBoardRevision;
    const data = await api("/government/board");
    if (boardRevision !== governmentBoardRevision) return;
    requests = data.cards;
    selectedReference = requests.some((request) => request.reference === preferredReference)
      ? preferredReference : (requests[0]?.reference || "");
    const stages = boardStages();
    byId("government-new").textContent = String(stages[0].cards.length);
    byId("government-review").textContent = String(stages[1].cards.length);
    byId("government-ready").textContent = String(stages[2].cards.length);
    const board = byId("government-board-demo");
    board.replaceChildren();
    for (const stage of stages) {
      const column = element("section", "", "kanban-column");
      const heading = element("div", "", "kanban-heading");
      heading.append(element("h3", stage.label), element("span", String(stage.cards.length)));
      column.append(heading);
      for (const request of stage.cards) {
        const card = element("button", "", "case-card");
        card.type = "button";
        if (request.reference === selectedReference) card.setAttribute("aria-current", "true");
        card.append(
          element("span", request.reference),
          serverText("strong", request.title),
          element("small", `${request.column} · ${request.unit_name || "Unassigned"}`),
          element("small", `Due ${formatDate(request.due_at)}`),
        );
        card.addEventListener("click", () => {
          governmentDrafting?.clear();
          selectedReference = request.reference;
          loadGovernmentBoard(request.reference).catch(showPageError);
        });
        column.append(card);
      }
      if (!stage.cards.length) column.append(element("p", "No cases in this stage."));
      board.append(column);
    }
    await renderGovernmentCase();
  }

  function boardStages() {
    return [
      { label: "New", cards: requests.filter((request) => ["submitted", "acknowledged"].includes(request.status)) },
      { label: "Under review", cards: requests.filter((request) =>
        ["clarification_needed", "routed", "in_review", "extended"].includes(request.status)) },
      { label: "For response", cards: requests.filter((request) =>
        !["submitted", "acknowledged", "clarification_needed", "routed", "in_review", "extended"].includes(request.status)) },
    ];
  }

  async function renderGovernmentCase() {
    if (!selectedReference) {
      governmentDrafting?.clear();
      byId("government-case-title").textContent = "No cases assigned to your agency";
      byId("government-case-meta").textContent = "";
      byId("government-case-description").textContent = "";
      byId("government-case-timeline").replaceChildren();
      return;
    }
    const reference = selectedReference;
    const detail = await api(`/government/requests/${encodeURIComponent(reference)}`);
    if (reference !== selectedReference) return;
    caseDetail = detail;
    governmentDrafting?.setCase(reference, detail);
    const request = caseDetail.request;
    byId("government-case-title").textContent = `${request.reference} — ${request.title}`;
    byId("government-case-meta").textContent =
      `${request.agency_name} · ${request.status.replaceAll("_", " ")} · Due ${formatDate(request.due_at)}`;
    byId("government-case-description").setAttribute("data-language-ignore", "");
    byId("government-case-description").textContent = request.description;
    const units = byId("triage-unit-demo");
    units.replaceChildren(...caseDetail.units.map((unit) => option(unit.id, unit.name)));
    units.value = request.assigned_unit_id || caseDetail.units[0]?.id || "";
    byId("route-case").disabled = !caseDetail.actions.includes("route");
    const timeline = byId("government-case-timeline");
    timeline.replaceChildren(...caseDetail.events.map((event) => timelineItem({
      label: event.type.replaceAll("_", " "),
      detail: event.summary,
      time: event.created_at,
    })));
  }

  async function applyTriage(event) {
    event.preventDefault();
    if (!caseDetail) return;
    const status = byId("government-action-status");
    status.textContent = "Saving triage…";
    try {
      await api(`/government/requests/${encodeURIComponent(selectedReference)}/triage`, {
        method: "POST",
        body: {
          categories: byId("triage-category-demo").value,
          destination_unit_id: byId("triage-unit-demo").value,
        },
      });
      status.textContent = "Triage saved for human review.";
      await loadGovernmentBoard(selectedReference);
    } catch (error) {
      status.textContent = error.message;
    }
  }

  async function routeCase() {
    if (!caseDetail) return;
    const status = byId("government-action-status");
    status.textContent = "Routing case…";
    try {
      await api(`/government/requests/${encodeURIComponent(selectedReference)}/action`, {
        method: "POST",
        body: {
          action: "route",
          destination_unit_id: byId("triage-unit-demo").value,
          note: "Routed during the demonstration review.",
        },
      });
      status.textContent = "Case routed to the selected unit.";
      await loadGovernmentBoard(selectedReference);
    } catch (error) {
      status.textContent = error.message;
    }
  }

  async function sendGovernmentReply(event) {
    event.preventDefault();
    if (!caseDetail) return;
    const textarea = byId("government-reply-demo");
    const status = byId("government-action-status");
    status.textContent = "Sending response…";
    try {
      await api(`/government/requests/${encodeURIComponent(selectedReference)}/messages`, {
        method: "POST",
        body: { visibility: "public", body: textarea.value.trim() },
      });
      textarea.value = "";
      status.textContent = "Public response sent.";
      await loadGovernmentBoard(selectedReference);
    } catch (error) {
      status.textContent = error.message;
    }
  }

  async function api(path, { method = "GET", body } = {}) {
    if (!apiBaseUrl) throw new Error("The API is not configured for this deployment.");
    const headers = { Accept: "application/json" };
    if (body !== undefined) headers["Content-Type"] = "application/json";
    if (csrfToken && method !== "GET") headers["X-CSRF-Token"] = csrfToken;
    let response;
    try {
      response = await fetch(`${apiBaseUrl}/api/v1${path}`, {
        method,
        credentials: "include",
        headers,
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    } catch {
      throw new Error("Could not reach the API. Check its URL, TLS, and cookie settings.");
    }
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) {
      const error = new Error(payload.error?.message || `API request failed (${response.status}).`);
      error.status = response.status;
      throw error;
    }
    if (payload.data?.session?.csrf_token) csrfToken = payload.data.session.csrf_token;
    return payload.data;
  }

  function timelineItem(update) {
    const item = element("div", "", "timeline-item");
    const content = element("div");
    content.append(
      element("strong", update.label),
      serverText("p", update.detail),
      element("small", formatDate(update.time)),
    );
    item.append(element("span", "", "timeline-marker"), content);
    return item;
  }

  function statusBadge(status) {
    return element("span", status.replaceAll("_", " "), `status-badge status-${status.replaceAll("_", "-")}`);
  }

  function option(value, label) {
    const item = document.createElement("option");
    item.setAttribute("data-language-ignore", "");
    item.value = value;
    item.textContent = label;
    return item;
  }

  function element(tag, text = "", className = "") {
    const item = document.createElement(tag);
    if (className) item.className = className;
    item.textContent = text;
    return item;
  }

  function serverText(tag, value) {
    const item = element(tag, value);
    item.setAttribute("data-language-ignore", "");
    return item;
  }

  function formatDate(value) {
    const date = new Date(value);
    return Number.isNaN(date.getTime()) ? String(value) : new Intl.DateTimeFormat("en-PH", {
      dateStyle: "medium",
      timeStyle: value.includes("T") ? "short" : undefined,
    }).format(date);
  }

  function showPageError(error) {
    const status = byId(page === "citizen" ? "citizen-submit-status" : "government-action-status");
    if (status) status.textContent = error instanceof Error ? error.message : String(error);
  }

  function byId(id) {
    return document.getElementById(id);
  }
})();
