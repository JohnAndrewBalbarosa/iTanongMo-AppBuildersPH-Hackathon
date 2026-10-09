# Deployment runbook

## Install

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
