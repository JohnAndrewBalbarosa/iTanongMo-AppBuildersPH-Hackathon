(() => {
  "use strict";

  const apiBaseUrl = window.iTanongMo?.apiBaseUrl || "";
  const form = document.getElementById("foi-request-form");
  if (!form) return;
  const byId = (id) => document.getElementById(id);
  const selectedAgencies = new Map();
  let agencies = [];

  initialize();
  byId("agency-search").addEventListener("input", renderAgencyOptions);
  byId("add-agency").addEventListener("click", addAgency);
  form.addEventListener("submit", submitRequest);

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
    if (!data?.batch_reference || !data?.requester_access_token || !data.requests?.length ||
        data.requests.some((request) => !request.reference || !request.agency)) {
      throw new Error("Submission response is incomplete. Contact support before retrying.");
    }
    byId("batch-reference").textContent = data.batch_reference;
    byId("requester-access-token").textContent = data.requester_access_token;
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
