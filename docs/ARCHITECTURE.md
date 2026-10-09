# Architecture

## Purpose

iTanongMo separates a static GitHub Pages frontend from a JSON/API-only Rust backend on a small VPS. GitHub Pages owns both citizen and government HTML. The backend exposes public requests, two-way in-app conversation, a government work board, deterministic triage, accountable AI assistance, released documents, deadlines, dispositions, and appeals. Authentication and verified identity federation are post-MVP.

## Module map

```text
src/
  app/             HTTP composition, configuration, startup, security headers
  identity/        demo actor context, roles, authorization helpers
  organization/    agency directory, DOST hierarchy, routing targets
  requests/        intake, deadlines, conversation, documents, workflow
  triage/          versioned high-recall deterministic rules
  ai/              government-only private Modal adapter and advisory jobs
  requester_ai/    disabled-by-default backend relay for a future managed mode
  board/           government Kanban projections
  notifications/   in-app notification records
  audit/           structured, append-only application audit events
frontend/          zero-build GitHub Pages citizen client
migrations/        SQLite schema owned by the application
ai-modal/          isolated remote model runtime
deploy/            systemd, Nginx, and backup assets
```

This hybrid follows the useful traits of the PyTorch PH and Agora Artifacts structures: folders represent business capabilities, while external systems sit behind explicit provider or adapter boundaries. The structure does not copy either repository because their exact source trees were unavailable in this workspace.

## Runtime boundaries

- The Rust process is the only application backend. It owns JSON APIs, validation, authorization, SQLite, audit records, and deterministic decisions. It does not serve application HTML.
- GitHub Pages serves all citizen and government HTML, CSS, JavaScript, and the non-authoritative agency fallback. It contains no provider secret, business decision, or authoritative state.
- The API allows one configured frontend origin through exact-origin CORS. Requests from GitHub Pages require an HTTPS API origin.
- The private Modal AI endpoint is government-only. It returns advisory triage and response suggestions and cannot publish, decide, or send.
- Requester Gemini BYOK drafting calls Google directly from the browser. The user key stays in tab memory for at most 15 minutes and never reaches the Rust API. The browser validates JSON for the email body, form fields, and agency queries without changing the manual name or title. The existing feature-gated Rust relay is reserved for a future managed mode and remains disabled until authentication, rate limits, quotas, and cost controls are implemented.
- The requester UI states that the token leaves the computer over HTTPS for the selected provider call. “Local” means ephemeral tab memory and zero application persistence, not offline inference.
- Every government AI output stores model, prompt version, input/output hashes, and review metadata. Publishing requires a separate authorized human action and a matching output hash.
- Email transport, identity proofing, eGov integration, and mobile push are post-MVP adapters. The domain workflow remains transport-independent.

## Data and operational limits

- SQLite uses foreign keys and WAL mode. One app instance is the supported MVP topology.
- Uploaded public files are allowlisted, size-limited to 10 MiB, hashed, and stored outside the binary.
- The versioned public API supports agency discovery, multi-agency submission, batch confirmation, request tracking, and requester AI drafting. Authentication remains a post-MVP release gate for broader use.
- Submission returns one 64-hex-character requester capability token for the whole batch (two UUIDv4 values; approximately 244 bits of random entropy). Only its SHA-256 hash is stored. Public tracking remains readable by opaque reference, while requester replies and appeals require the token in an `Authorization: Bearer` header. The static client keeps it only in `sessionStorage` for the current browser tab and also shows it once for requester-managed recovery.
- A multi-agency submission creates one batch with an independent child request per agency so deadlines, access, replies, disposition, and appeal do not leak across agencies.
- Logs exclude request bodies, email addresses, credentials, cookies, and AI payloads. Audit metadata uses bounded identifiers and state changes.
