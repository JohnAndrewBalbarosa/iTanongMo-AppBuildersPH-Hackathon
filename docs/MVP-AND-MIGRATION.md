# MVP and migration

## MVP boundary

Included now:

- Static GitHub Pages citizen and government interfaces. The Rust backend exposes JSON APIs only; legacy server-rendered pages are not part of the runtime.
- All Philippine Executive Departments as shallow directory entries; DOST has the deep demonstration hierarchy.
- Public timeline, released documents, two-way in-app thread, working-day due dates, extension, disposition, and appeal.
- Government personas, agency-scoped Kanban, triage confirmation, routing, internal notes, public replies, document release, CSV export, notifications, and audit events.
- Deterministic high-recall triage. A flagged request queues AI enrichment; an unflagged request remains visible on the board.
- Government Modal AI drafting and triage with explicit human review. Government AI output cannot self-publish.
- Citizen AI drafting uses the shared server-owned backend provider. The citizen signs in and sends a CSRF-protected request; no user API key or provider selection is accepted. AI output is schema-validated, previewed, and applied only on explicit user action. Identity, title, agency selection, consent, and submission remain under citizen control. Limits per API process: 5 attempts/user/minute, 60 attempts/hour total, 2 concurrent calls, 45-second timeout, and 2,048 output tokens. Restarting the process resets in-memory limits; multiple replicas need a shared limiter or upstream budget enforcement.
- Multi-agency intake fans out into independently tracked agency requests under one submission batch.
- Guest users track public status by FOI reference only and cannot reply or appeal. Authenticated citizen accounts authorize reply and appeal mutations through their server-side session.

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
