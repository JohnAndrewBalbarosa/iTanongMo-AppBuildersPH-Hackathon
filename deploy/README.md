# Deployment runbook

## Install

### Lightsail HTTPS demo

`deploy/Caddyfile` serves `https://54-179-96-27.sslip.io` and proxies to the Rust service on `127.0.0.1:4320`. The hostname resolves to the Lightsail static IP; keep the IP assigned to this instance. Permit inbound TCP 80 and 443 in the Lightsail firewall. Caddy obtains and renews a public certificate automatically. Back up the database, service binary, environment, and existing Nginx configuration before switching the reverse proxy. Set `ITANONGMO_PUBLIC_ORIGIN=https://54-179-96-27.sslip.io` and `ITANONGMO_FRONTEND_ORIGIN=https://johnandrewbalbarosa.github.io`. Set the GitHub repository variable `ITANONGMO_API_BASE_URL` to `https://54-179-96-27.sslip.io`, then redeploy Pages. Keep the Rust service bound to loopback.

The public demo accounts and seeded requests are synthetic; do not use this deployment for real personal or government data. Browser session cookies use `SameSite=None; Secure`, so browsers that block third-party cookies may require a first-party domain arrangement for login.

1. Build the Linux release binary in CI or on a compatible build host.
2. Create the `itanongmo` system user and `/opt/itanongmo/bin`, `/var/lib/itanongmo/files`, and `/var/backups/itanongmo` directories.
3. Copy the binary, `deploy/backup.sh`, systemd units, and Nginx configuration. Mark `backup.sh` executable.
4. Copy `itanongmo.env.example` to `/etc/itanongmo.env`, set mode `0600`, replace the API public host, set `ITANONGMO_FRONTEND_ORIGIN` to the exact GitHub Pages origin, and add the combined Modal proxy token only if government AI is enabled.
5. Run the Rust service on loopback, obtain the TLS certificate, validate Nginx, then enable the application, backup timer, and Nginx.
6. Set the GitHub repository variable `ITANONGMO_API_BASE_URL` to the HTTPS Rust API origin and enable GitHub Pages with GitHub Actions as the source.

## Verify

- `GET /health` returns `status=ok`; `ai=degraded` is acceptable when AI is intentionally disabled.
- Exact-origin CORS emits only the configured GitHub Pages origin and never reflects an unconfigured request origin.
- GitHub Pages can discover agencies, create a multi-agency batch, and track each child reference through the Rust API.
- A requester can use the one-time batch capability token to post a reply and file an eligible appeal, while the same token is rejected for a different batch.
- GitHub Pages government persona selection and Kanban work through the JSON API; CSV export remains available from the API.
- A completed government AI draft cannot publish with a mismatched output hash, without factual-review confirmation, or more than once.
- A test file restores successfully with `sqlite3 restored.db ".restore backup.db"` and passes `PRAGMA integrity_check`.
- Service restart preserves requests, files, audit events, and pending jobs.
- RSS stays below the systemd memory limit under the agreed pilot load.

## Recover

Stop the service, preserve the failed database and WAL files, restore the latest verified backup into a new path, run `PRAGMA integrity_check`, update the database URL if needed, and start the service. Keep the failed copy for bounded diagnosis; do not overwrite it.

## Shared citizen AI

Configure `ITANONGMO_AI_BASE_URL`, `ITANONGMO_AI_PROXY_TOKEN`, and `ITANONGMO_AI_MODEL` in the private server environment, then set `ITANONGMO_REQUESTER_AI_ENABLED=true`. The existing HTTPS OpenAI-compatible provider must support structured JSON output and the `requester_draft` task contract. Reuse the same configured provider as government drafting. Do not place credentials in Pages, repository variables, or static assets.

Deploy the API and frontend together after approval. Verify citizen login, AI draft preview/apply, manual submission, wrong-role/CSRF rejection, `429` limits, and provider-unavailable fallback. BYOK is retired; old `request.html` URLs redirect to the citizen portal. Rollback: restore the prior binary/frontend and keep `ITANONGMO_REQUESTER_AI_ENABLED=false` until both versions match. The token-TTL setting is retired.
