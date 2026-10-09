# iTanongMo — AppBuildersPH Hackathon

Production-candidate FOI replacement MVP with public and government interfaces. The static frontend runs on GitHub Pages; its optional local triage classifier is bundled during deployment. The Rust service is a JSON/API-only backend and never serves application HTML.

This repository is the fresh-history AppBuildersPH Hackathon edition. Runtime behavior and local development state continue from the original iTanongMo workspace.

## Run locally

Requirements: Rust 1.89+ and SQLite development libraries.

```sh
cargo run
```

The defaults run locally without environment variables. Use `.env.example` as the list of variables to export in your shell; the binary deliberately does not parse local `.env` files.

Run `npm ci` and `npm run build:triage` in `frontend/`, then serve `frontend/site/` with a static-file server and open its URL. Configure its Rust API origin as described in `frontend/README.md`. The Rust root returns API metadata, while `/health` reports readiness. Submit with synthetic data only; seeded government personas are available from `government.html` when the API is configured.

Each submission returns a requester access token once. Save it: public status is readable through the opaque request reference, but citizen replies and appeals require that token. The backend stores only its SHA-256 hash.

## Verification

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Production shape

Deploy the generated `frontend/site/` to GitHub Pages with the pinned workflow in `.github/workflows/pages.yml`. Build the Rust API with `cargo build --release`, place the binary under `/opt/itanongmo/bin`, put mutable data under `/var/lib/itanongmo`, configure `/etc/itanongmo.env`, and install the files under `deploy/`. Follow the [deployment runbook](deploy/README.md), terminate TLS at Nginx, and test backup restoration before enabling public traffic.

The government-only AI package is separate under `ai-modal/`. It is optional at runtime: deterministic triage and the manual government workflow continue if the private model is unavailable. Qwen3-4B and the Modal/vLLM image are intentionally recorded as **Untested yet use** until the pilot validates output quality, deployment compatibility, and cost.

Requester Gemini BYOK drafting runs directly from the HTTPS frontend to Google Gemini. The user-supplied key stays in tab memory for at most 15 minutes; the iTanongMo backend never receives it. The browser validates one JSON draft containing an email body, form values, and agency search queries; requester name and title remain manual. The separate Rust requester-AI relay remains disabled by default for a future server-managed mode, which requires authentication, rate limits, quotas, and cost controls before public enablement.

For loopback-only testing, set `ITANONGMO_REQUESTER_AI_ENABLED=true` and `ITANONGMO_LOCAL_DEVELOPMENT=true`. Never use the local-development override on a public deployment.

See [architecture](docs/ARCHITECTURE.md), [MVP and canary migration](docs/MVP-AND-MIGRATION.md), and the [low-maintenance proposal](docs/PROPOSAL.md).
