use sqlx::FromRow;

#[derive(Clone, Debug, FromRow)]
pub struct FoiRequest {
    pub id: String,
    pub public_reference: String,
    pub agency_id: String,
    pub agency_name: String,
    pub assigned_unit_id: Option<String>,
    pub requester_id: String,
    pub title: String,
    pub description: String,
    pub purpose: String,
    pub urgency_details: String,
    pub status: String,
    pub disposition: Option<String>,
    pub disposition_reason: Option<String>,
    pub submitted_at: String,
    pub due_at: String,
    pub extended_due_at: Option<String>,
    pub revision: i64,
}

#[derive(Clone, Debug, FromRow)]
pub struct PublicRequestRow {
    pub public_reference: String,
    pub agency_short_name: String,
    pub title: String,
    pub status: String,
    pub submitted_at: String,
    pub due_at: String,
}

#[derive(Clone, Debug)]
pub struct SubmissionBatch {
    pub id: String,
    pub batch_reference: String,
    pub created_at: String,
    pub requests: Vec<PublicRequestRow>,
}

#[derive(Clone, Debug, FromRow)]
pub struct RequestEvent {
    pub event_type: String,
    pub public_summary: String,
    pub from_status: Option<String>,
    pub to_status: Option<String>,
    pub created_at: String,
}
