(() => {
  "use strict";

  const configuredUrl = window.ITANONGMO_CONFIG?.apiBaseUrl || "";
  const apiBaseUrl = configuredUrl.replace(/\/$/, "");
  const currentPage = window.location.pathname.split("/").pop() || "index.html";
  const iconPaths = Object.freeze({
    about: "m11.25 11.25.041-.02a.75.75 0 0 1 1.063.852l-.708 2.836a.75.75 0 0 0 1.063.853l.041-.021M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9-3.75h.008v.008H12V8.25Z",
    government: "M2.25 21h19.5m-18-18v18m10.5-18v18m6-13.5V21M6.75 6.75h.75m-.75 3h.75m-.75 3h.75m3-6h.75m-.75 3h.75m-.75 3h.75M6.75 21v-3.375c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21M3 3h12m-.75 4.5H21m-3.75 3.75h.008v.008h-.008v-.008Zm0 3h.008v.008h-.008v-.008Zm0 3h.008v.008h-.008v-.008Z",
    key: "M15.75 5.25a3 3 0 0 1 3 3m3 0a6 6 0 0 1-7.029 5.912c-.563-.097-1.159.026-1.563.43L10.5 17.25H8.25v2.25H6v2.25H2.25v-2.818c0-.597.237-1.17.659-1.591l6.499-6.499c.404-.404.527-1 .43-1.563A6 6 0 1 1 21.75 8.25Z",
    question: "M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z",
    request: "M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m3.75 9v6m3-3H9m1.5-12H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z",
    shield: "M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z",
    sparkles: "M9.813 15.904 9 18.75l-.813-2.846a4.5 4.5 0 0 0-3.09-3.09L2.25 12l2.846-.813a4.5 4.5 0 0 0 3.09-3.09L9 5.25l.813 2.846a4.5 4.5 0 0 0 3.09 3.09L15.75 12l-2.846.813a4.5 4.5 0 0 0-3.09 3.09ZM18.259 8.715 18 9.75l-.259-1.035a3.375 3.375 0 0 0-2.455-2.456L14.25 6l1.036-.259a3.375 3.375 0 0 0 2.455-2.456L18 2.25l.259 1.035a3.375 3.375 0 0 0 2.456 2.456L21.75 6l-1.035.259a3.375 3.375 0 0 0-2.456 2.456ZM16.894 20.567 16.5 21.75l-.394-1.183a2.25 2.25 0 0 0-1.423-1.423L13.5 18.75l1.183-.394a2.25 2.25 0 0 0 1.423-1.423l.394-1.183.394 1.183a2.25 2.25 0 0 0 1.423 1.423l1.183.394-1.183.394a2.25 2.25 0 0 0-1.423 1.423Z",
    track: "m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z",
  });
  window.iTanongMo = Object.freeze({ apiBaseUrl });

  function navigationItem(label, href, options = {}) {
    const container = document.createElement("div");
    container.className = "navigation-item";
    const link = document.createElement("a");
    link.textContent = label;
    link.href = href;
    if (options.icon) link.prepend(decorativeIcon(options.icon));
    if (options.current) link.setAttribute("aria-current", "page");
    if (options.disabled) {
      link.setAttribute("aria-disabled", "true");
      link.title = "Available after the public Rust API is configured.";
      link.addEventListener("click", (event) => event.preventDefault());
    }
    container.append(link);
    return container;
  }

  function decorativeIcon(name) {
    const namespace = "http://www.w3.org/2000/svg";
    const icon = document.createElementNS(namespace, "svg");
    icon.classList.add("ui-icon");
    icon.setAttribute("viewBox", "0 0 24 24");
    icon.setAttribute("fill", "none");
    icon.setAttribute("stroke", "currentColor");
    icon.setAttribute("stroke-width", "1.5");
    icon.setAttribute("aria-hidden", "true");
    const path = document.createElementNS(namespace, "path");
    path.setAttribute("stroke-linecap", "round");
    path.setAttribute("stroke-linejoin", "round");
    path.setAttribute("d", iconPaths[name]);
    icon.append(path);
    return icon;
  }

  function renderHeader() {
    const header = document.querySelector("header") || document.createElement("header");
    header.className = "site-header";
    const brand = navigationItem("iTanongMo", "index.html", { current: currentPage === "index.html" });
    brand.classList.add("brand");
    const navigation = document.createElement("nav");
    navigation.className = "main-navigation";
    navigation.setAttribute("aria-label", "Main navigation");
    const authItem = navigationItem("Sign in", "login.html", { current: currentPage === "login.html", icon: "key" });
    navigation.append(
      navigationItem("About", "about.html", { current: currentPage === "about.html", icon: "about" }),
      authItem,
    );
    const authStatus = document.createElement("span");
    authStatus.className = "header-auth-status";
    authStatus.setAttribute("role", "status");
    const authLink = authItem.querySelector("a");
    authLink.addEventListener("click", async (event) => {
      if (authLink.dataset.session !== "active") return;
      event.preventDefault();
      await signOut(authLink, authStatus);
    });
    showSessionAction(authLink);
    const headerActions = document.createElement("div");
    headerActions.className = "header-actions";
    headerActions.append(navigation, authStatus);
    header.replaceChildren(brand, headerActions);
    if (!header.isConnected) document.body.prepend(header);
  }

  // Mental model: the backend session selects the header action; logout verifies it again for a current CSRF token.
  async function showSessionAction(authLink) {
    const session = await readSession();
    if (!session) return;
    authLink.dataset.session = "active";
    authLink.textContent = "Sign out";
    authLink.href = "#";
    authLink.removeAttribute("aria-current");
  }

  async function signOut(authLink, authStatus) {
    authStatus.textContent = "";
    const session = await readSession();
    if (!session) {
      window.location.assign("login.html");
      return;
    }
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/auth/logout`, {
        method: "POST",
        credentials: "include",
        headers: { Accept: "application/json", "Content-Type": "application/json", "X-CSRF-Token": session.csrf_token },
        body: "{}",
      });
      if (!response.ok) throw new Error("Sign out failed. Please try again.");
      window.location.replace("login.html");
    } catch {
      authStatus.textContent = "Sign out failed. Please try again.";
      authLink.focus();
    }
  }

  async function readSession() {
    if (!apiBaseUrl) return null;
    try {
      const response = await fetch(`${apiBaseUrl}/api/v1/auth/session`, {
        credentials: "include",
        headers: { Accept: "application/json" },
      });
      if (!response.ok) return null;
      const payload = await response.json();
      return payload.data?.csrf_token && payload.data?.actor ? payload.data : null;
    } catch {
      return null;
    }
  }

  function createLanguagePicker() {
    const languagePicker = document.createElement("label");
    languagePicker.className = "language-picker";
    languagePicker.append(document.createElement("span"));
    languagePicker.firstChild.textContent = "Language";
    const languageSelect = document.createElement("select");
    languageSelect.setAttribute("aria-label", "Language");
    for (const [value, label] of [["fil", "Tagalog"], ["en", "English"]]) {
      const option = document.createElement("option");
      option.value = value;
      option.textContent = label;
      languageSelect.append(option);
    }
    languageSelect.value = window.iTanongMoLanguage?.current || "en";
    languageSelect.addEventListener("change", () => window.iTanongMoLanguage?.setLanguage(languageSelect.value));
    languagePicker.append(languageSelect);
    return languagePicker;
  }

  function renderFooter() {
    const footer = document.querySelector("footer") || document.createElement("footer");
    footer.className = "site-footer";
    const navigation = document.createElement("nav");
    navigation.className = "footer-navigation";
    navigation.setAttribute("aria-label", "Footer navigation");
    navigation.append(
      navigationItem("Privacy", "privacy.html", { current: currentPage === "privacy.html", icon: "shield" }),
      navigationItem("Accessibility", "accessibility.html", { current: currentPage === "accessibility.html", icon: "about" }),
    );
    footer.replaceChildren(navigation, createLanguagePicker());
    if (!footer.isConnected) document.body.append(footer);
  }

  function loadStyles() {
    if (document.querySelector('link[href="assets/site.css"]')) return;
    const stylesheet = document.createElement("link");
    stylesheet.rel = "stylesheet";
    stylesheet.href = "assets/site.css";
    document.head.append(stylesheet);
  }

  function renderActionIcons() {
    const actionIcons = new Map([
      ["open-ai-draft", "sparkles"],
      ["open-ai-information", "question"],
      ["open-api-key", "key"],
    ]);
    actionIcons.forEach((iconName, id) => {
      const action = document.getElementById(id);
      if (action && !action.querySelector(".ui-icon")) action.prepend(decorativeIcon(iconName));
    });
    const primaryLink = document.querySelector('main > p > a[href="request.html"]');
    if (primaryLink && !primaryLink.querySelector(".ui-icon")) primaryLink.prepend(decorativeIcon("request"));
  }

  loadStyles();
  renderHeader();
  renderFooter();
  renderActionIcons();
})();
