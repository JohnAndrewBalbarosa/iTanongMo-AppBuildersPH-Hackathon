# iTanongMo — AppBuildersPH Hackathon

Production-candidate FOI replacement MVP with public and government interfaces. The static frontend runs on GitHub Pages, including an optional local triage classifier. The Rust service is a JSON/API-only backend and never serves application HTML.

This repository is the fresh-history AppBuildersPH Hackathon edition. Runtime behavior and local development state continue from the original iTanongMo workspace.

## Run locally

Requirements: Rust 1.89+ and SQLite development libraries.

```sh
cargo run
```

The defaults run locally without environment variables. Use `.env.example` as the list of variables to export in your shell; the binary deliberately does not parse local `.env` files.

Serve `frontend/` with a static-file server and open its URL. Configure its Rust API origin as described in `frontend/README.md`. The Rust root returns API metadata, while `/health` reports readiness. Submit with synthetic data only; seeded government personas are available from `government.html` when the API is configured.

Guest submissions return FOI references for public status tracking. Guest reply and appeal mutations are unavailable; authenticated citizen accounts use their server-side session for those actions.

## Verification

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Production shape

Deploy `frontend/` to GitHub Pages with the pinned workflow in `.github/workflows/pages.yml`. Build the Rust API with `cargo build --release`, place the binary under `/opt/itanongmo/bin`, put mutable data under `/var/lib/itanongmo`, configure `/etc/itanongmo.env`, and install the files under `deploy/`. Follow the [deployment runbook](deploy/README.md), terminate TLS at Nginx, and test backup restoration before enabling public traffic.

The shared AI package is separate under `ai-modal/`. It is optional at runtime: manual citizen requests, deterministic triage, and the government workflow continue if the private model is unavailable. Qwen3-4B and the Modal/vLLM image are intentionally recorded as **Untested yet use** until the pilot validates output quality, deployment compatibility, and cost.

Citizen AI drafting uses the shared server-owned backend provider. The citizen signs in and sends a CSRF-protected request; no user API key or provider selection is accepted. AI output is schema-validated, previewed, and applied only on explicit user action. Identity, title, agency selection, consent, and submission remain under citizen control. Limits per API process: 5 attempts/user/minute, 60 attempts/hour total, 2 concurrent calls, 45-second timeout, and 2,048 output tokens. Restarting the process resets in-memory limits; multiple replicas need a shared limiter or upstream budget enforcement.

For loopback-only testing, set `ITANONGMO_REQUESTER_AI_ENABLED=true` and `ITANONGMO_LOCAL_DEVELOPMENT=true`. Never use the local-development override on a public deployment.

See [architecture](docs/ARCHITECTURE.md), [MVP and canary migration](docs/MVP-AND-MIGRATION.md), and the [low-maintenance proposal](docs/PROPOSAL.md).
