# GitHub Pages frontend

This directory contains the zero-build static citizen and government interfaces. The Rust `itanongmo` binary is a JSON/API-only backend and does not serve these files. The government classifier uses self-hosted browser runtime files under `assets/vendor/`; package versions, licenses, bundle source, and SHA-256 hashes are recorded in `assets/vendor/SOURCES.json`. Rebuild the worker from `triage/worker.mjs` and update the manifest whenever its source or runtime changes.

## Required deployment values

1. Give the Rust API an HTTPS origin such as `https://api.example.gov.ph`.
2. Set `ITANONGMO_FRONTEND_ORIGIN` on the Rust service to the exact GitHub Pages origin, such as `https://owner.github.io`. Do not include a repository path because an HTTP origin never includes a path.
3. Set the repository variable `ITANONGMO_API_BASE_URL` to the HTTPS Rust API origin without a path. When unset, submission and tracking remain unavailable, but browser-only Gemini BYOK drafting works on Pages.
4. Enable GitHub Pages with **GitHub Actions** as its source.

The deployment workflow validates the API origin and writes it to `config.js` inside the deployment artifact. No API key or server credential belongs in repository variables or static files.

Requester drafting calls Gemini directly from the browser using a user-supplied API key kept in tab memory for at most 15 minutes. The frontend validates the returned JSON before applying it. The existing Rust requester-AI relay remains disabled by default and is reserved for a future server-managed offering; public use needs authentication, rate limits, quotas, and cost controls.

The submission response contains a requester capability token once. The citizen client displays it and keeps it in same-tab `sessionStorage`; it never places the token in a URL. Replies and appeals send it through the `Authorization` header. Verified accounts replace this capability after MVP.

## Demonstration portals

`login.html` signs in against the Rust `/api/v1/auth/login` endpoint with the public demo accounts listed on that page. The server returns the actor role; the browser uses it to open the citizen or government portal. Portal requests use a server session cookie, `credentials: "include"`, and the current CSRF token. A missing API configuration or failed request shows an error and never creates a local fake result.

`citizen.html` submits FOI requests through the API and loads only that account's requests and public timelines. `government-demo.html` loads the agency board, saves triage, routes eligible cases to units, and posts public replies through authorized API endpoints. The synthetic cases in `../demo/requests.json` can be submitted to a running demo backend with `../demo/seed-requests.ps1`. These cases appear on the DOST board; they are not owned by the seeded citizen account.

The Pages site needs a reachable HTTPS Rust API and the exact Pages origin in `ITANONGMO_FRONTEND_ORIGIN`. Cross-site session cookies may be blocked by browser privacy settings when Pages and the API use unrelated domains; verify login and mutations in the intended browsers before presenting the live demo.

## Local preview

Set `apiBaseUrl` in `config.js` to the local Rust origin, set the matching `ITANONGMO_FRONTEND_ORIGIN` on the Rust service, and serve this directory with any static-file preview tool. The preview server is development tooling, never part of the backend deployment.

The local triage pilot uses pinned multilingual MiniLM ONNX with a 107 MB INT8 model download on first use. Browser inference uses WebAssembly/CPU; request text remains in the browser. The label map and preliminary thresholds live in `triage/label-map.v1.mjs`. Reviewers must explicitly copy suggested categories and save through the existing authorized triage endpoint. A model download or inference failure leaves the deterministic rules and manual review available. Filipino/Taglish recall, latency on weak laptops, and threshold calibration require labeled pilot data before operational reliance. A native-phone ExecuTorch companion and LAN pairing are separate future work.
