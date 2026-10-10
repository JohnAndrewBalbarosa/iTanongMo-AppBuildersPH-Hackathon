# Demonstration Authentication API

## Scope

`/api/v1/auth` provides password-validated demonstration identities backed by SQLite sessions. These accounts demonstrate application authentication and authorization only. They are **not production identity verification**, proof of citizenship, or government employee identity assurance.

Passwords use Argon2id PHC hashes. Session and CSRF tokens are cryptographically random; SQLite stores only their SHA-256 hashes. A session lasts eight hours and is revocable server-side.

## Seeded credentials

| Identity | Email | Password | Server role |
|---|---|---|---|
| Juan Dela Cruz | `citizen.judge@itanongmo.local` | `CitizenJudge2026!` | `requester` |
| DOST Receiving Officer | `officer.judge@itanongmo.local` | `OfficerJudge2026!` | `receiving_officer`, `agency-dost` |

These credentials are intentionally public demonstration data. Never reuse them in production.

## Login

`POST /api/v1/auth/login`

```json
{
  "email": "citizen.judge@itanongmo.local",
  "password": "CitizenJudge2026!"
}
```

The server trims and lowercases the email, validates the password, resolves the actor from seeded server data, creates a bounded server-side session, and returns:

```json
{
  "data": {
    "actor": {
      "id": "requester-demo",
      "display_name": "Juan Dela Cruz",
      "actor_type": "requester",
      "agency_id": null,
      "organization_unit_id": null,
      "roles": ["requester"]
    },
    "csrf_token": "<opaque-token>",
    "expires_at": "<RFC3339 UTC>"
  }
}
```

Unknown email and incorrect password return the same `401 invalid_credentials` response. Five failures for one normalized email within 60 seconds cause later attempts in that window to return `429 rate_limited`.

## Session lookup

`GET /api/v1/auth/session` requires the `itanongmo_session` cookie. It returns the login response shape and rotates the CSRF token. Replace the previously held CSRF token with the returned value.

Missing, malformed, revoked, inactive, or unknown sessions return `401 authentication_required`. An expired session returns `401 session_expired` and is revoked.

## Logout

`POST /api/v1/auth/logout` requires the session cookie and CSRF token. Supply CSRF through `X-CSRF-Token` or the compatibility JSON field:

```json
{"csrf_token":"<opaque-token>"}
```

Success returns:

```json
{"data":{"signed_out":true}}
```

The server revokes the database session and clears the cookie using the same path and security profile.

## Cookie contract

HTTPS responses set:

```text
itanongmo_session=<opaque-token>; Path=/api/v1; HttpOnly; Secure; SameSite=None; Max-Age=28800
```

For an explicit `http://localhost` or `http://127.0.0.1` API origin only, `Secure` is omitted and `SameSite=Lax` is used. No `Domain` attribute is set.

## CSRF

Every authenticated `POST`, `PUT`, `PATCH`, or `DELETE` must present the current CSRF token. Government JSON forms retain `csrf_token` body compatibility, but new clients should use `X-CSRF-Token`. Tokens are compared through constant-time hash comparison; raw tokens are never persisted or logged.

## CORS

Set `ITANONGMO_FRONTEND_ORIGIN` to one exact origin, for example `https://example.github.io`. Credentialed responses allow only that origin, set `Access-Control-Allow-Credentials: true`, allow `Content-Type`, `Authorization`, and `X-CSRF-Token`, and include `Vary: Origin`. Wildcard origins are never combined with credentials.

The legacy persona selector is disabled by default. It is registered only when `ITANONGMO_ENABLE_PERSONA_LOGIN=true`, and then creates the same server-side session rather than client-controlled role cookies.

## Citizen endpoints

- `GET /api/v1/citizen/requests` returns only requests owned by the authenticated requester.
- `GET /api/v1/citizen/requests/{reference}` returns an allowlisted public timeline: request fields, public event summaries, public messages, and released documents.
- An authenticated `POST /api/v1/submissions` requires CSRF and binds new requests to the session’s server-side `requester_id`. Client-supplied requester identity does not control ownership.
- Anonymous submission and request-capability access remain available for backward compatibility.

Citizen responses exclude internal messages, private event details, triage records, AI drafts, and audit data. A non-owned reference returns the same `404` behavior as an unknown reference.

## Government endpoints

Government routes resolve `ActorContext` exclusively from `itanongmo_session`. Requester sessions receive `403 forbidden`. Agency-scoped helpers continue to enforce that a DOST receiving officer can access only `agency-dost`. Government mutations require CSRF.

## Error envelope

Authentication and authorization failures use:

```json
{
  "error": {
    "code": "invalid_credentials",
    "message": "Invalid email or password."
  }
}
```

Codes are `validation_failed`, `invalid_credentials`, `authentication_required`, `session_expired`, `csrf_invalid`, `forbidden`, `rate_limited`, and `internal_error`.

## Frontend integration

```js
const login = await fetch(`${apiBaseUrl}/api/v1/auth/login`, {
  method: "POST",
  credentials: "include",
  headers: { "content-type": "application/json" },
  body: JSON.stringify({ email, password }),
});

const mutation = await fetch(url, {
  method: "POST",
  credentials: "include",
  headers: {
    "content-type": "application/json",
    "x-csrf-token": csrfToken,
  },
  body: JSON.stringify(payload),
});
```

Do not send role, agency, unit, actor ID, or requester ID from the client as an authorization claim.

## Citizen AI drafting

`POST /api/v1/requester-ai/draft` requires an active citizen session cookie and `X-CSRF-Token`. JSON input contains only `what_to_ask` (20–8,000 characters) and `manual_title` (5–180 characters). Provider/model/key fields are rejected. The server selects its configured provider and credentials.

Success: `{"data":{"email_body":"...","description":"...","purpose":"...","urgency_details":"...","agency_queries":["..."]}}`. Drafting does not create an FOI request. The citizen reviews the draft, applies editable details, selects an agency, and submits separately.

Outcomes: `401` missing/expired session; `403` wrong role or CSRF; `422` invalid input/output; `429` usage/concurrency limit; `503` disabled/unconfigured/unavailable provider. `/api/v1/config` advertises `requester_ai_enabled` only when the feature and provider are configured. BYOK is retired; `request.html` redirects to the citizen portal.

Usage controls per API process: 5 attempts/user/minute, 60 attempts/hour total, 2 concurrent calls, 45-second timeout, 2,048 maximum output tokens. Failed provider attempts consume quota. In-memory quotas reset on restart; replicas need a shared limiter/upstream budget. Logs include bounded outcomes and correlation IDs, never prompts, drafts, or credentials.
