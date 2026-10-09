# GitHub Pages frontend

This directory contains the zero-build citizen and government interfaces. The Rust `itanongmo` binary is a JSON/API-only backend and does not serve these files.

## Required deployment values

1. Give the Rust API an HTTPS origin such as `https://api.example.gov.ph`.
2. Set `ITANONGMO_FRONTEND_ORIGIN` on the Rust service to the exact GitHub Pages origin, such as `https://owner.github.io`. Do not include a repository path because an HTTP origin never includes a path.
3. Set the repository variable `ITANONGMO_API_BASE_URL` to the HTTPS Rust API origin without a path. When unset, submission and tracking remain unavailable, but browser-only Gemini BYOK drafting works on Pages.
4. Enable GitHub Pages with **GitHub Actions** as its source.

The deployment workflow validates the API origin and writes it to `config.js` inside the deployment artifact. No API key or server credential belongs in repository variables or static files.

Requester drafting calls Gemini directly from the browser using a user-supplied API key kept in tab memory for at most 15 minutes. The frontend validates the returned JSON before applying it. The existing Rust requester-AI relay remains disabled by default and is reserved for a future server-managed offering; public use needs authentication, rate limits, quotas, and cost controls.

The submission response contains a requester capability token once. The citizen client displays it and keeps it in same-tab `sessionStorage`; it never places the token in a URL. Replies and appeals send it through the `Authorization` header. Verified accounts replace this capability after MVP.

## Local preview

Set `apiBaseUrl` in `config.js` to the local Rust origin, set the matching `ITANONGMO_FRONTEND_ORIGIN` on the Rust service, and serve this directory with any static-file preview tool. The preview server is development tooling, never part of the backend deployment.
