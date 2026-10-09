CREATE TABLE request_batches (
    id TEXT PRIMARY KEY,
    batch_reference TEXT NOT NULL UNIQUE,
    requester_id TEXT NOT NULL REFERENCES requesters(id),
    created_at TEXT NOT NULL
);

ALTER TABLE foi_requests
ADD COLUMN batch_id TEXT REFERENCES request_batches(id);

CREATE UNIQUE INDEX idx_requests_batch_agency
ON foi_requests(batch_id, agency_id)
WHERE batch_id IS NOT NULL;

CREATE INDEX idx_requests_batch
ON foi_requests(batch_id, submitted_at);
