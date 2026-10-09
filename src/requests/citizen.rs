use sqlx::{FromRow, SqlitePool};

use crate::error::{AppError, AppResult};

use super::FoiRequest;

#[derive(Clone, Debug, FromRow)]
pub struct CitizenRequestSummary {
    pub public_reference: String,
    pub title: String,
    pub agency_id: String,
    pub agency_name: String,
    pub status: String,
    pub submitted_at: String,
    pub due_at: String,
    pub latest_public_message: Option<String>,
}

pub async fn list_owned_requests(
    pool: &SqlitePool,
    requester_id: &str,
) -> AppResult<Vec<CitizenRequestSummary>> {
    Ok(sqlx::query_as::<_, CitizenRequestSummary>(
        "SELECT r.public_reference,r.title,a.id agency_id,a.name agency_name,r.status,r.submitted_at,COALESCE(r.extended_due_at,r.due_at) due_at,(SELECT m.body FROM messages m WHERE m.request_id=r.id AND m.visibility='public' ORDER BY m.created_at DESC LIMIT 1) latest_public_message FROM foi_requests r JOIN agencies a ON a.id=r.agency_id WHERE r.requester_id=? AND r.hidden_at IS NULL ORDER BY r.submitted_at DESC,r.public_reference",
    )
    .bind(requester_id)
    .fetch_all(pool)
    .await?)
}

pub async fn load_owned_request(
    pool: &SqlitePool,
    requester_id: &str,
    reference: &str,
) -> AppResult<FoiRequest> {
    let owned = sqlx::query_scalar::<_, String>(
        "SELECT id FROM foi_requests WHERE public_reference=? AND requester_id=? AND hidden_at IS NULL",
    )
    .bind(reference)
    .bind(requester_id)
    .fetch_optional(pool)
    .await?;
    if owned.is_none() {
        return Err(AppError::NotFound);
    }
    super::load_request(pool, reference).await
}
