(() => {
  "use strict";

  const apiBaseUrl = window.iTanongMo?.apiBaseUrl || "";
  const form = document.querySelector("#tracking-form");
  const output = document.querySelector("#tracking-output");
  const actions = document.querySelector("#requester-actions");
  const actionStatus = document.querySelector("#requester-action-status");
  let currentRequest = null;

  function accessStorageKey(reference) {
    return `itanongmo.requester-access.${reference}`;
  }

  function accessToken() {
    return form.elements.access_token.value.trim();
  }

  async function apiFetch(path, options = {}) {
    if (!apiBaseUrl) throw new Error("Hindi pa configured ang HTTPS Rust API.");
    const response = await fetch(`${apiBaseUrl}${path}`, { credentials: "omit", ...options });
    if (!response.ok) throw new Error((await response.text()) || `Request failed (${response.status}).`);
    return response.json();
  }

  function appendText(tagName, value) {
    const element = document.createElement(tagName);
    element.setAttribute("data-language-ignore", "");
    element.textContent = value;
    output.append(element);
  }

  function renderRequest(request) {
    output.replaceChildren();
    appendText("h2", `${request.reference}: ${request.title}`);
    appendText("p", `${request.agency} · ${request.status} · due ${request.due_at}`);
    appendText("p", request.description);
    for (const event of request.events) appendText("div", `${event.created_at} — ${event.summary}`);
    for (const message of request.messages) appendText("div", `${message.created_at} — ${message.actor}: ${message.body}`);
    for (const document of request.documents) {
      const link = document.createElement("a");
      link.href = `${apiBaseUrl}${document.download_url}`;
      link.textContent = `${document.title} (${document.media_type}, ${document.size_bytes} bytes)`;
      output.append(link);
    }
    actions.hidden = !accessToken();
    document.querySelector("#appeal-controls").hidden = !request.can_appeal;
  }

  async function load(reference) {
    const request = (await apiFetch(`/api/v1/requests/${encodeURIComponent(reference)}`)).data;
    currentRequest = request;
    if (accessToken()) sessionStorage.setItem(accessStorageKey(reference), accessToken());
    renderRequest(request);
  }

  async function mutate(suffix, body) {
    if (!currentRequest || !accessToken()) throw new Error("Kailangan ang request reference at requester access token.");
    actionStatus.textContent = "Saving…";
    await apiFetch(`/api/v1/requests/${encodeURIComponent(currentRequest.reference)}${suffix}`, {
      method: "POST",
      headers: { "authorization": `Bearer ${accessToken()}`, "content-type": "application/json" },
      body: JSON.stringify(body),
    });
    sessionStorage.setItem(accessStorageKey(currentRequest.reference), accessToken());
    await load(currentRequest.reference);
    actionStatus.textContent = "Saved.";
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const reference = form.elements.reference.value.trim();
    try {
      const savedToken = sessionStorage.getItem(accessStorageKey(reference));
      if (!accessToken() && savedToken) form.elements.access_token.value = savedToken;
      await load(reference);
    } catch (error) {
      output.textContent = error.message;
    }
  });
  document.querySelector("#send-requester-message").addEventListener("click", () => {
    mutate("/messages", { body: document.querySelector("#requester-message").value })
      .catch((error) => { actionStatus.textContent = error.message; });
  });
  document.querySelector("#file-appeal").addEventListener("click", () => {
    mutate("/appeals", { reason: document.querySelector("#appeal-reason").value })
      .catch((error) => { actionStatus.textContent = error.message; });
  });

  const reference = new URLSearchParams(window.location.search).get("reference");
  if (reference) {
    form.elements.reference.value = reference;
    form.elements.access_token.value = sessionStorage.getItem(accessStorageKey(reference)) || "";
    load(reference).catch((error) => { output.textContent = error.message; });
  }
})();
