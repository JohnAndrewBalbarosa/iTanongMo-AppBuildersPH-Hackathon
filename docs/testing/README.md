# Local AI triage evidence campaign

Primary feature: government local AI triage. The portal places classification before the board and response drafting. Model suggestions remain advisory; classification MUST NOT save triage, route cases, or publish responses.

Run from the repository root on Windows with Node 24, Chrome, Rust GNU 1.98.0, the existing `ai-local/.venv`, and the verified pinned model in `var/models/mdeberta-triage`:

```powershell
& C:\Users\jbalb\.codex-shared\tools\observability-hub\interval-supervisor.cmd --label triage-evidence --log-dir "$env:TEMP\itanongmo-triage-evidence" --interval 30 -- node tests/evidence/campaign.cjs
```

Existing application-test failures remain failures and cause a nonzero final exit; independent measurements continue so the report captures the complete campaign. Static-check or evidence-metric failures stop the campaign. The fixed campaign accepts no target URL. It creates disposable loopback APIs, SQLite databases and Chrome profiles. No live deployment, shared-provider credential or production request is used. Expected duration: approximately 15–20 minutes, including a 10-minute soak. Stop limits: 5-second HTTP timeout and abort after at least 20 samples if error rate exceeds 10%; completed measurements retain failed attempts.

## Order and scope

1. JavaScript syntax, Python Ruff lint/format, Rust fmt/clippy.
2. Complete Rust, frontend, browser-triage policy, evidence-metric and Python companion/evidence suites.
3. Verify model file hashes; load offline; one warmup; 32 synthetic English/Filipino/Taglish cases, twice each, at a predeclared 0.5 threshold. Record per-category confusion counts, precision, recall, F1, macro-F1, exact match, repeated label agreement, latency and process peak working set.
4. Chrome: citizen AI preview/manual submission; real local-model classification; zero automatic backend mutations; Tagalog copy; triage prominence; desktop/mobile alignment and overflow; focused/accessible classify button; reviewed publication. Citizen draft output is mocked; government response output is seeded.
5. HTTP security negatives: role/agency/CSRF boundaries, revoked-session lookup, pending-read logout, FRO publish rejection, required human confirmation, stale hash and repeated publish rejection. Verify the citizen receives the reviewed response.
6. Local companion load: 1/2/4 workers, at most eight classifications per stage and 60 seconds per stage. Authenticated API board probes: 1/5/10/20 workers, at most 200 requests or 15 seconds per stage. Closed-loop client limits MUST NOT be described as server capacity or saturation discovery.
7. Ten-minute soak: two API workers with one-second per-worker pause; one local-classifier worker with 20-second pause. Record error counts, p50/p95/p99 latency and resource snapshots. Windows peak working set is a process-lifetime counter; snapshots do not prove absence of leaks.
8. SQLite Online Backup API, service restart, restore into a separate temporary database, compare request/message/job/review fingerprints and verify the reviewed message through the recovered API. No production recovery-point claim is supported.

## Evidence contract

`var/test-evidence/<timestamp>/` contains `report.json`, a readable `report.md`, per-suite outcome/count records, benchmark records, security/browser records, screenshots, source hashes and an artifact SHA-256 manifest. Node counts come from the documented `TestsStream` final summary, not console text. Successful command output is not retained; failures retain only a bounded redacted tail. Cookies, CSRF values, provider keys and real citizen text MUST NOT appear in reports.

Every record includes test type, ISO/IEC 25010 quality codes, method, workload, numeric measurements and limitations. Environment and source identities are recorded once. Evidence initially has `verification: unverified`, `maturity: controlled_measured` and `resume_eligible: false`. Promotion requires explicit user/officer verification, independently checked labels when claiming classification quality, an exact supporting artifact/measurement reference, and honest scope. Repeated calls within one run do not constitute independent replication. ISO labels do not imply certification.

The report uses an intermediate measurement format; it MUST NOT be presented as an already validated PyTorch PH `evidence-backed-resume/v2` document. Verified results can subsequently be linked to normalized evidence and claims using their exact measurement IDs.

## Pending evaluations

- Real shared-AI drafting quality requires a configured evaluation provider, verified source records and human factual review; mock/seeded output cannot establish it.
- The synthetic FOI labels are assistant-authored and unverified. Macro-F1 on this set is an exploratory measurement, not representative real-world accuracy.
- Chrome emulation is not physical-device, Firefox or Safari evidence. Focus/accessibility-tree checks are not a complete WCAG or screen-reader audit.
- Test companion origin/port are adapted only in its isolated process and served test JavaScript. Live Pages local-network permission and Release EXE download/installation remain separate.
- Local inference stays offline after pinned artifacts exist. Offline benchmark does not prove the entire citizen/government application works without its API.

## Reuse and measurement references

Existing Rust tests, Node tests, Chrome CDP, Python `unittest`, pinned `triage.classify` and `companion.create_server` own the tested behavior. No new dependency is introduced. Grafana k6 was considered for performance testing but is not installed; this smaller fixed local probe reuses Node and has explicitly limited conclusions. The installed observability CLI lacks the global `tech`/`reuse` commands; selection is recorded in the campaign limitations and change assessment.

- [Node performance measurement API](https://nodejs.org/api/perf_hooks.html): monotonic client timing.
- [SQLite Online Backup API](https://www.sqlite.org/backup.html): consistent live snapshot, including WAL-managed state.
- [scikit-learn F1 definition](https://scikit-learn.org/stable/modules/generated/sklearn.metrics.f1_score.html): F1 and unweighted per-label macro aggregation; the bounded implementation records confusion counts and uses zero for undefined division.
- [Grafana k6](https://github.com/grafana/k6): maintained load-testing candidate for a separately designed capacity campaign.

- [Node TestsStream summaries](https://nodejs.org/docs/latest-v24.x/api/test.html): structured final test counts.
