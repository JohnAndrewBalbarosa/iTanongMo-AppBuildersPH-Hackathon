# Low-maintenance deployment proposal

## Call to action

Run a 90-day DOST pilot on one small VPS and one optional isolated AI endpoint. The core public and government workflow works without AI, without a mail server, and without paid enterprise infrastructure. The pilot therefore tests service quality and adoption before government commits to larger operating cost.

## Cost controls

| Item | MVP choice | Cost effect |
|---|---|---|
| Web runtime | One compiled Rust binary | Low idle CPU and memory; no application VM runtime |
| Database | SQLite WAL on attached disk | No managed database charge during the single-instance pilot |
| Pages | Static semantic HTML, CSS, and JavaScript with no frontend build | Smaller deployment and maintenance surface |
| Files | Local allowlisted public releases plus backups | No object-store dependency at pilot volume |
| AI | Gemini OpenAI-compatible API for the pilot; private Modal/Qwen remains available | Start within provider free-tier quotas, then measure paid usage before production |
| AI outage | Queued retry plus deterministic/manual workflow | Public service remains usable without model capacity |
| Email | Verified office address links only | No delivery/reputation system in the MVP |
| Operations | systemd, Nginx, daily SQLite backup | Uses standard Linux tools and a short recovery procedure |

The exact monthly amount depends on current Lightsail, domain, backup, traffic, and AI-provider prices. The architectural target is a 512 MiB instance, but acceptance must use measured peak RSS, disk growth, backup restore time, and load-test latency. If the memory ceiling is exceeded, the first upgrade is instance size; application and data interfaces remain the same.

## Pilot acceptance

- No request disappears because of triage. Every submission remains visible and searchable.
- AI cannot publish or decide without a human reviewer.
- Agency staff can complete intake through appeal and download an audit-ready CSV.
- Backup restore is demonstrated on a separate directory.
- The service meets the agreed accessibility, privacy, security, response-time, and incident criteria.
- Pilot metrics show less handling time or fewer routing errors than the old email-only workflow.
