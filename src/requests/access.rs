use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::{
    error::{AppError, AppResult},
    identity::{ActorContext, ActorType, Role},
};

pub async fn authorize_requester(
    pool: &SqlitePool,
    request_reference: &str,
    access_token: &str,
) -> AppResult<ActorContext> {
    let access_token = access_token.trim();
    if access_token.len() != 64 || !access_token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        tracing::warn!(
            event = "requester.access_rejected",
            request_reference,
            reason = "malformed_token"
        );
        return Err(AppError::Forbidden);
    }
    let row: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT q.id,q.display_name,b.requester_access_hash FROM foi_requests r JOIN request_batches b ON b.id=r.batch_id JOIN requesters q ON q.id=r.requester_id WHERE r.public_reference=? AND r.hidden_at IS NULL",
    )
    .bind(request_reference)
    .fetch_optional(pool)
    .await?;
    let (requester_id, display_name, expected_hash) = row.ok_or_else(|| {
        tracing::warn!(
            event = "requester.access_rejected",
            request_reference,
            reason = "unknown_request"
        );
        AppError::Forbidden
    })?;
    let supplied_hash = hex::encode(Sha256::digest(access_token.as_bytes()));
    let expected_hash = expected_hash.ok_or(AppError::Forbidden)?;
    if !constant_time_equal(expected_hash.as_bytes(), supplied_hash.as_bytes()) {
        tracing::warn!(
            event = "requester.access_rejected",
            request_reference,
            reason = "token_mismatch"
        );
        return Err(AppError::Forbidden);
    }
    tracing::info!(
        event = "requester.access_authorized",
        request_reference,
        outcome = "success"
    );
    Ok(ActorContext {
        subject_id: requester_id,
        display_name,
        actor_type: ActorType::Requester,
        agency_id: None,
        organization_unit_id: None,
        roles: vec![Role::Requester],
        assurance_level: "request_capability".into(),
        provider: "itanongmo".into(),
    })
}

#[inline]
fn constant_time_equal(expected: &[u8], supplied: &[u8]) -> bool {
    if expected.len() != supplied.len() {
        return false;
    }
    expected
        .iter()
        .zip(supplied)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}
