PRAGMA foreign_keys = ON;

CREATE TABLE agencies (
    id TEXT PRIMARY KEY,
    uacs_code TEXT UNIQUE,
    slug TEXT NOT NULL UNIQUE,
    short_name TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    official_url TEXT NOT NULL,
    office_email TEXT,
    source_url TEXT NOT NULL,
    source_as_of TEXT NOT NULL,
    deep_profile INTEGER NOT NULL DEFAULT 0 CHECK (deep_profile IN (0, 1)),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
);

CREATE TABLE organization_units (
    id TEXT PRIMARY KEY,
    agency_id TEXT NOT NULL REFERENCES agencies(id),
    parent_id TEXT REFERENCES organization_units(id),
    slug TEXT NOT NULL,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    source_url TEXT NOT NULL,
    source_as_of TEXT NOT NULL,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    UNIQUE (agency_id, slug)
);

CREATE TABLE demo_personas (
    id TEXT PRIMARY KEY,
    agency_id TEXT REFERENCES agencies(id),
    organization_unit_id TEXT REFERENCES organization_units(id),
    display_name TEXT NOT NULL,
    role TEXT NOT NULL,
    actor_type TEXT NOT NULL CHECK (actor_type IN ('requester', 'staff', 'platform_admin')),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1))
);

CREATE TABLE requesters (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    contact_email TEXT NOT NULL,
    assurance_level TEXT NOT NULL DEFAULT 'demo',
    consent_version TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE foi_requests (
    id TEXT PRIMARY KEY,
    public_reference TEXT NOT NULL UNIQUE,
    agency_id TEXT NOT NULL REFERENCES agencies(id),
    assigned_unit_id TEXT REFERENCES organization_units(id),
    requester_id TEXT NOT NULL REFERENCES requesters(id),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    purpose TEXT NOT NULL,
    urgency_details TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    disposition TEXT,
    disposition_reason TEXT,
    submitted_at TEXT NOT NULL,
    acknowledged_at TEXT,
    due_at TEXT NOT NULL,
    extended_due_at TEXT,
    closed_at TEXT,
    hidden_at TEXT,
    hidden_reason TEXT,
    revision INTEGER NOT NULL DEFAULT 1,
    demo INTEGER NOT NULL DEFAULT 1 CHECK (demo IN (0, 1))
);

CREATE TABLE request_events (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES foi_requests(id) ON DELETE CASCADE,
    actor_id TEXT,
    event_type TEXT NOT NULL,
    public_summary TEXT NOT NULL,
    private_detail TEXT,
    from_status TEXT,
    to_status TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE messages (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES foi_requests(id) ON DELETE CASCADE,
    actor_id TEXT NOT NULL,
    actor_label TEXT NOT NULL,
    visibility TEXT NOT NULL CHECK (visibility IN ('public', 'internal')),
    body TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE triage_results (
    request_id TEXT PRIMARY KEY REFERENCES foi_requests(id) ON DELETE CASCADE,
    rules_version TEXT NOT NULL,
    rule_hits_json TEXT NOT NULL,
    ai_status TEXT NOT NULL,
    ai_suggestion_json TEXT,
    human_categories_json TEXT,
    human_destination_unit_id TEXT REFERENCES organization_units(id),
    reviewed_by TEXT,
    reviewed_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE ai_jobs (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES foi_requests(id) ON DELETE CASCADE,
    task_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    input_json TEXT NOT NULL,
    input_hash TEXT NOT NULL,
    output_json TEXT,
    output_hash TEXT,
    model_id TEXT,
    model_revision TEXT,
    prompt_version TEXT NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0,
    available_at TEXT NOT NULL,
    claimed_at TEXT,
    completed_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (request_id, task_kind, input_hash)
);

CREATE TABLE ai_draft_reviews (
    id TEXT PRIMARY KEY,
    ai_job_id TEXT NOT NULL REFERENCES ai_jobs(id) ON DELETE CASCADE,
    request_id TEXT NOT NULL REFERENCES foi_requests(id) ON DELETE CASCADE,
    reviewer_id TEXT NOT NULL,
    source_output_hash TEXT NOT NULL,
    published_content_hash TEXT NOT NULL,
    factual_review_confirmed INTEGER NOT NULL CHECK (factual_review_confirmed IN (0, 1)),
    published_at TEXT NOT NULL,
    UNIQUE (ai_job_id)
);

CREATE TABLE released_documents (
    id TEXT PRIMARY KEY,
    request_id TEXT NOT NULL REFERENCES foi_requests(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    stored_name TEXT NOT NULL UNIQUE,
    original_name TEXT NOT NULL,
    media_type TEXT NOT NULL,
    byte_length INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    released_by TEXT NOT NULL,
    released_at TEXT NOT NULL
);

CREATE TABLE notifications (
    id TEXT PRIMARY KEY,
    persona_id TEXT NOT NULL REFERENCES demo_personas(id) ON DELETE CASCADE,
    request_id TEXT REFERENCES foi_requests(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    message TEXT NOT NULL,
    read_at TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE audit_events (
    id TEXT PRIMARY KEY,
    correlation_id TEXT NOT NULL,
    actor_id TEXT,
    actor_assurance TEXT NOT NULL,
    operation TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision INTEGER,
    outcome TEXT NOT NULL,
    detail_json TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE feature_flags (
    capability TEXT NOT NULL,
    agency_id TEXT,
    organization_unit_id TEXT,
    cohort_percent INTEGER NOT NULL DEFAULT 100 CHECK (cohort_percent BETWEEN 0 AND 100),
    routing_mode TEXT NOT NULL DEFAULT 'in_app_primary',
    enabled INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    updated_at TEXT NOT NULL,
    PRIMARY KEY (capability, agency_id, organization_unit_id)
);

CREATE TABLE holidays (
    holiday_date TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    source_url TEXT NOT NULL
);

CREATE TABLE rate_limits (
    subject_hash TEXT NOT NULL,
    operation TEXT NOT NULL,
    window_started_at TEXT NOT NULL,
    attempts INTEGER NOT NULL,
    PRIMARY KEY (subject_hash, operation)
);

CREATE INDEX idx_requests_agency_status ON foi_requests(agency_id, status, submitted_at DESC);
CREATE INDEX idx_requests_due ON foi_requests(status, due_at);
CREATE INDEX idx_events_request ON request_events(request_id, created_at);
CREATE INDEX idx_messages_request ON messages(request_id, visibility, created_at);
CREATE INDEX idx_ai_jobs_claim ON ai_jobs(status, available_at, created_at);
CREATE INDEX idx_notifications_persona ON notifications(persona_id, read_at, created_at DESC);
CREATE INDEX idx_audit_entity ON audit_events(entity_type, entity_id, created_at DESC);
