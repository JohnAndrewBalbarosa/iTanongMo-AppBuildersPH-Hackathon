PRAGMA foreign_keys = ON;

CREATE TABLE auth_users (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL UNIQUE COLLATE NOCASE,
    password_hash TEXT NOT NULL,
    persona_id TEXT NOT NULL UNIQUE REFERENCES demo_personas(id),
    requester_id TEXT REFERENCES requesters(id),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES auth_users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    csrf_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    revoked_at TEXT
);

CREATE INDEX idx_auth_sessions_user_active
ON auth_sessions(user_id, revoked_at, expires_at, created_at DESC);

CREATE INDEX idx_auth_sessions_expiry
ON auth_sessions(expires_at)
WHERE revoked_at IS NULL;
