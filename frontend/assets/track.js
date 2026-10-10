(() => {
  "use strict";

  const apiBaseUrl = window.iTanongMo?.apiBaseUrl || "";
  const form = document.querySelector("#tracking-form");
  const output = document.querySelector("#tracking-output");

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
  }

  async function load(reference) {
    const request = (await apiFetch(`/api/v1/requests/${encodeURIComponent(reference)}`)).data;
    renderRequest(request);
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const reference = form.elements.reference.value.trim();
    try {
      await load(reference);
    } catch (error) {
      output.textContent = error.message;
    }
  });

  const reference = new URLSearchParams(window.location.search).get("reference");
  if (reference) {
    form.elements.reference.value = reference;
    load(reference).catch((error) => { output.textContent = error.message; });
  }
})();
