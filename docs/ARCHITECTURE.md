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
  ai/              shared private Modal adapter and government advisory jobs
  requester_ai/    authenticated shared-provider citizen drafting and bounded usage controls
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
- The private Modal AI endpoint serves citizen request drafts and government advisory triage/response suggestions. It cannot publish, decide, or send.
- Citizen AI drafting uses the shared server-owned backend provider. The citizen signs in and sends a CSRF-protected request; no user API key or provider selection is accepted. AI output is schema-validated, previewed, and applied only on explicit user action. Identity, title, agency selection, consent, and submission remain under citizen control. Limits per API process: 5 attempts/user/minute, 60 attempts/hour total, 2 concurrent calls, 45-second timeout, and 2,048 output tokens. Restarting the process resets in-memory limits; multiple replicas need a shared limiter or upstream budget enforcement.
- Citizen drafting sends only the manual title and request instructions through the authenticated backend to its configured model service; provider credentials remain server-owned. Prompts and draft contents are not persisted by the drafting service.
- Every government AI output stores model, prompt version, input/output hashes, and review metadata. Publishing requires a separate authorized human action and a matching output hash.
- Email transport, identity proofing, eGov integration, and mobile push are post-MVP adapters. The domain workflow remains transport-independent.

## Planned local JEB classification runtime

- The zero-build government web client pilots a pinned multilingual MiniLM zero-shot ONNX model through self-hosted Transformers.js and ONNX Runtime Web files on WebAssembly/CPU. Its versioned label map and deterministic thresholds turn independent scores into reviewable category suggestions. The model is downloaded on demand and cached in the browser; request text is processed locally. It was not validated on Filipino or Taglish FOI requests, so thresholds are provisional and every suggestion requires human review. A project-owned JEB checkpoint, tokenizer, calibrated thresholds, label map, and evaluation results are still required before replacing the pilot model.
- Keep classification advisory and subject to authorized human review. A model result must not grant access to another agency's request or publish a response automatically. Server authorization and deterministic triage remain authoritative.
- A future native phone companion may use ExecuTorch with an export validated for its target backend. Pairing, encrypted LAN transfer, device revocation, and device-specific NPU tests need separate implementation. Browser inference does not require a paired phone.

### ExecuTorch compatibility finding (checked 2026-10-10)

ExecuTorch's ready-made LLM export workflow lists supported architectures, but that list is not a whitelist for all models. The general export path can accept other PyTorch models supported by `torch.export` and the target runtime. Backend operator coverage is the constraint: unsupported delegated operations may fall back to portable CPU kernels; an unsupported export or missing runtime kernel requires a compatible graph, custom operator/kernel, or another runtime. A separate `.pte` export may be needed for each hardware backend. A custom BERT-derived JEB classifier therefore needs export and device benchmarks before compatibility or NPU speed can be claimed.

ONNX hardware backends also have operator limits. ONNX Runtime Web documents broad ONNX operator support on WebAssembly but subsets on WebGPU and WebNN. Use WebAssembly as the tested fallback; enable acceleration only after the actual JEB model loads correctly and improves measured latency on target devices. ONNX is the selected path because the government UI runs in a browser and must work on weak laptops, not because it guarantees NPU execution.

Sources: [ExecuTorch ready-made LLM export](https://docs.pytorch.org/executorch/stable/llm/export-llm.html), [ExecuTorch custom export](https://docs.pytorch.org/executorch/stable/llm/export-custom-llm.html), [ExecuTorch runtime kernels](https://docs.pytorch.org/executorch/stable/using-executorch-runtime-integration.html), [ONNX Runtime Web](https://onnxruntime.ai/docs/tutorials/web/), [ONNX Runtime execution providers](https://onnxruntime.ai/docs/execution-providers/).

## Data and operational limits

- SQLite uses foreign keys and WAL mode. One app instance is the supported MVP topology.
- Uploaded public files are allowlisted, size-limited to 10 MiB, hashed, and stored outside the binary.
- The versioned public API supports agency discovery, multi-agency submission, batch confirmation, request tracking, and requester AI drafting. Authentication remains a post-MVP release gate for broader use.
- Guest submission returns opaque FOI references for public status tracking and does not issue a requester capability token. Guest reply and appeal mutations are unavailable; authenticated citizen accounts authorize those actions through their server-side session.
- A multi-agency submission creates one batch with an independent child request per agency so deadlines, access, replies, disposition, and appeal do not leak across agencies.
- Logs exclude request bodies, email addresses, credentials, cookies, and AI payloads. Audit metadata uses bounded identifiers and state changes.
