(() => {
  "use strict";

  const configuredUrl = window.ITANONGMO_CONFIG?.apiBaseUrl || "";
  const apiBaseUrl = configuredUrl.replace(/\/$/, "");
  const currentPage = window.location.pathname.split("/").pop() || "index.html";
  window.iTanongMo = Object.freeze({ apiBaseUrl });

  function navigationItem(label, href, options = {}) {
    const container = document.createElement("div");
    container.className = "navigation-item";
    const link = document.createElement("a");
    link.textContent = label;
    link.href = href;
    if (options.current) link.setAttribute("aria-current", "page");
    if (options.disabled) {
      link.setAttribute("aria-disabled", "true");
      link.title = "Available after the public Rust API is configured.";
      link.addEventListener("click", (event) => event.preventDefault());
    }
    container.append(link);
    return container;
  }

  function renderHeader() {
    const header = document.querySelector("header") || document.createElement("header");
    header.className = "site-header";
    const brand = navigationItem("iTanongMo", "index.html", { current: currentPage === "index.html" });
    brand.classList.add("brand");
    const navigation = document.createElement("nav");
    navigation.className = "main-navigation";
    navigation.setAttribute("aria-label", "Main navigation");
    navigation.append(
      navigationItem("Make a request", "request.html", { current: currentPage === "request.html" }),
      navigationItem("Track a request", "track.html", { current: currentPage === "track.html" }),
      navigationItem("About", "about.html", { current: currentPage === "about.html" }),
    );
    if (currentPage === "index.html") {
      navigation.append(navigationItem("Government workspace", "government.html"));
    }
    header.replaceChildren(brand, navigation);
    if (!header.isConnected) document.body.prepend(header);
  }

  function renderFooter() {
    const footer = document.querySelector("footer") || document.createElement("footer");
    footer.className = "site-footer";
    const navigation = document.createElement("nav");
    navigation.className = "footer-navigation";
    navigation.setAttribute("aria-label", "Footer navigation");
    navigation.append(
      navigationItem("Privacy", "privacy.html", { current: currentPage === "privacy.html" }),
      navigationItem("Accessibility", "accessibility.html", { current: currentPage === "accessibility.html" }),
    );
    footer.replaceChildren(navigation);
    if (!footer.isConnected) document.body.append(footer);
  }

  function loadStyles() {
    if (document.querySelector('link[href="assets/site.css"]')) return;
    const stylesheet = document.createElement("link");
    stylesheet.rel = "stylesheet";
    stylesheet.href = "assets/site.css";
    document.head.append(stylesheet);
  }

  loadStyles();
  renderHeader();
  renderFooter();
})();
