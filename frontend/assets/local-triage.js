(() => {
  "use strict";

  const endpoint = "http://127.0.0.1:8765";
  const categoryNames = Object.freeze({
    time_sensitive: "Time sensitive",
    safety_review: "Safety review",
    sensitive_data: "Sensitive data",
    voluminous: "Large volume of records",
    multi_unit: "Multiple units",
    appeal: "Appeal",
    records_preservation: "Records preservation",
    clarification: "Needs clarification",
  });
  const filipinoCategories = Object.freeze({
    time_sensitive: "Kailangang madaliin", safety_review: "Pagsusuri sa kaligtasan",
    sensitive_data: "Sensitibong datos", voluminous: "Maraming rekord",
    multi_unit: "Maraming yunit", appeal: "Apela",
    records_preservation: "Pag-iingat ng rekord", clarification: "Kailangang linawin",
  });
  const copy = Object.freeze({
    eyebrow: ["Optional local review", "Opsiyonal na lokal na pagsusuri"],
    title: ["Run the local classifier", "Patakbuhin ang lokal na classifier"],
    setup: [
      "Download the Windows EXE and open it once. It downloads the model on first launch, then starts a service on this computer. Keep the EXE open while reviewing requests. Later launches use the saved model; Python and an API key are not needed.",
      "I-download ang Windows EXE at buksan ito nang isang beses. Ida-download nito ang model sa unang bukas, saka sisimulan ang service sa computer na ito. Panatilihing bukas ang EXE habang sinusuri ang mga kahilingan. Sa susunod, gagamitin ang naka-save na model; hindi kailangan ang Python o API key.",
    ],
    review: [
      "The browser may ask for local-network permission. Select a case, then run the classifier. Scores are advisory and are never saved or routed automatically.",
      "Maaaring humingi ang browser ng local-network permission. Pumili ng kaso, saka patakbuhin ang classifier. Gabay lamang ang scores at hindi awtomatikong sine-save o niruruta.",
    ],
    download: ["Download Windows EXE", "I-download ang Windows EXE"],
    check: ["Check connection", "Suriin ang koneksiyon"],
    classify: ["Classify selected case", "I-classify ang napiling kaso"],
    initial: ["Open the EXE to connect the local model.", "Buksan ang EXE para ikonekta ang lokal na model."],
    checking: ["Checking the local model…", "Sinusuri ang lokal na model…"],
    ready: ["Local model is ready on this computer.", "Handa na ang lokal na model sa computer na ito."],
    loading: ["The EXE is running but the model is still loading.", "Tumatakbo ang EXE ngunit naglo-load pa ang model."],
    classifying: ["Classifying on this computer…", "Nagki-classify sa computer na ito…"],
    complete: [
      "Local scores are advisory. Review the case and save triage manually.",
      "Gabay lamang ang lokal na scores. Suriin ang kaso at manu-manong i-save ang triage.",
    ],
    connectError: [
      "Open the Windows EXE, allow browser local-network access, then retry.",
      "Buksan ang Windows EXE, payagan ang browser local-network access, saka subukan muli.",
    ],
    selectCase: ["Select a case on the request board first.", "Pumili muna ng kaso sa request board."],
    invalidScores: ["The local model returned invalid scores.", "Hindi valid ang scores mula sa lokal na model."],
  });
  const status = document.getElementById("local-model-status");
  const scores = document.getElementById("local-model-scores");
  const checkButton = document.getElementById("check-local-model");
  const classifyButton = document.getElementById("classify-local-case");
  if (!status || !scores || !checkButton || !classifyButton) return;
  let latestScores = null;

  document.addEventListener("itanongmo:languagechange", localize);
  localize();
  checkButton.addEventListener("click", () => run(checkConnection));
  classifyButton.addEventListener("click", () => run(classifySelectedCase));

  // Mental model: verify the local companion, classify visible case text, then show review-only scores.
  async function run(operation) {
    checkButton.disabled = true;
    classifyButton.disabled = true;
    try {
      await operation();
    } catch (error) {
      const key = Object.keys(copy).find((name) => copy[name][0] === error.message);
      setStatus(key || error.message);
      console.error(JSON.stringify({ event: "local_model.browser_request_failed", component: "government-portal" }));
    } finally {
      checkButton.disabled = false;
      classifyButton.disabled = false;
    }
  }

  async function checkConnection() {
    setStatus("checking");
    const response = await request("/health");
    setStatus(response.ready ? "ready" : "loading");
  }

  async function classifySelectedCase() {
    const text = selectedCaseText();
    scores.replaceChildren();
    setStatus("classifying");
    const result = await request("/classify", { method: "POST", body: JSON.stringify({ text }) });
    renderScores(result.scores);
    latestScores = result.scores;
    setStatus("complete");
  }

  function selectedCaseText() {
    const title = document.getElementById("government-case-title").textContent.trim();
    const description = document.getElementById("government-case-description").textContent.trim();
    if (!description || title === "Choose a request" || title === "No cases assigned to your agency") {
      throw new Error("Select a case on the request board first.");
    }
    return `${title}\n${description}`.slice(0, 8000);
  }

  async function request(path, options = {}) {
    let response;
    try {
      response = await fetch(`${endpoint}${path}`, {
        credentials: "omit",
        cache: "no-store",
        headers: { Accept: "application/json", ...(options.method ? { "Content-Type": "application/json" } : {}) },
        ...options,
      });
    } catch {
      throw new Error("Open the Windows EXE, allow browser local-network access, then retry.");
    }
    const payload = await response.json().catch(() => ({}));
    if (!response.ok) throw new Error(payload.error || `Local model request failed (${response.status}).`);
    return payload;
  }

  function renderScores(result) {
    const categories = Object.keys(categoryNames);
    if (!result || categories.some((category) =>
      typeof result[category] !== "number" || !Number.isFinite(result[category]) ||
      result[category] < 0 || result[category] > 1)) {
      throw new Error("The local model returned invalid scores.");
    }
    scores.replaceChildren();
    for (const category of categories.sort((left, right) => result[right] - result[left])) {
      const row = document.createElement("li");
      const label = document.createElement("span");
      const value = document.createElement("strong");
      label.textContent = window.iTanongMoLanguage?.current === "fil"
        ? filipinoCategories[category] : categoryNames[category];
      value.textContent = `${Math.round(result[category] * 100)}%`;
      row.append(label, value);
      scores.append(row);
    }
  }

  function setStatus(key) {
    status.dataset.localCopy = key;
    localizeElement(status);
  }

  function localize() {
    for (const element of document.querySelectorAll("[data-local-copy]")) localizeElement(element);
    if (latestScores) renderScores(latestScores);
  }

  function localizeElement(element) {
    const languageIndex = window.iTanongMoLanguage?.current === "fil" ? 1 : 0;
    element.textContent = copy[element.dataset.localCopy]?.[languageIndex] || element.dataset.localCopy;
  }
})();
