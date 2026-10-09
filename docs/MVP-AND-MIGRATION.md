# MVP and migration

## MVP boundary

Included now:

- Static GitHub Pages citizen and government interfaces. The Rust backend exposes JSON APIs only; legacy server-rendered pages are not part of the runtime.
- All Philippine Executive Departments as shallow directory entries; DOST has the deep demonstration hierarchy.
- Public timeline, released documents, two-way in-app thread, working-day due dates, extension, disposition, and appeal.
- Government personas, agency-scoped Kanban, triage confirmation, routing, internal notes, public replies, document release, CSV export, notifications, and audit events.
- Deterministic high-recall triage. A flagged request queues AI enrichment; an unflagged request remains visible on the board.
- Government Modal AI drafting and triage with explicit human review. Government AI output cannot self-publish.
- Requester Gemini BYOK drafting makes one browser-to-Google call and validates JSON containing an email body, form values, and agency queries. Name and title remain manual. The user key stays in tab memory for at most 15 minutes and is never sent to the Rust backend; the frontend requires HTTPS. The disabled backend relay is reserved for a future managed mode with authentication, rate limits, quotas, and cost controls.
- Multi-agency intake fans out into independently tracked agency requests under one submission batch.
- Citizen replies and appeals use a one-time-disclosed batch capability token until verified identity replaces it. The backend stores only the token hash and authorizes every mutation against the request's batch.

Deferred after MVP:

- Filipino citizen and government official identity proofing, account recovery, MFA, eGov API or OIDC federation, and production authorization administration.
- Backend email delivery, inbound email parsing, office mailbox synchronization, delivery receipts, and bounce handling.
- Native mobile app and push transport.
- Multi-instance database topology, malware scanning service, records-retention automation, and agency-specific legal rule packs.

## Canary migration

1. **Mirror:** Keep the old user-to-email path authoritative. Deploy the GitHub Pages frontend to a bounded audience, import only synthetic or consented cases, and compare triage/routing results.
2. **Pilot:** Enable in-app handling for one DOST receiving unit and a bounded requester cohort. Continue official email notifications from the old system.
3. **Expand:** Add units and users through feature flags. Track completion time, overdue rate, clarification rate, routing corrections, false-negative reviews, and support load.
4. **Default:** Make in-app work the default for proven agencies while retaining a documented fallback and export.
5. **Retire:** Remove the old write path only after records reconciliation, retention checks, accessibility validation, incident exercises, and formal agency acceptance.

Rollback at every stage means disabling the affected feature flag, preserving the audit trail, exporting active cases, and returning that cohort to the previous channel. Schema changes remain backward-compatible during a canary window.

## Triage policy

Recall has priority over precision: likely urgent, sensitive, safety-related, broad, multi-unit, preservation, clarification, and appeal language is over-selected. Human reviewers cull false positives. All unflagged requests stay in the ordinary queue so a regex miss never deletes or hides a case. AI analyzes only flagged cases and remains advisory.
