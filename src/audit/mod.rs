//! Append-only redacted audit events.

use chrono::Utc;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{error::AppResult, identity::ActorContext};

pub struct Entry<'a> {
    pub operation: &'a str,
    pub entity_type: &'a str,
    pub entity_id: &'a str,
    pub revision: Option<i64>,
    pub outcome: &'a str,
    pub detail: Value,
}

pub async fn record(
    pool: &SqlitePool,
    actor: Option<&ActorContext>,
    entry: Entry<'_>,
) -> AppResult<()> {
    let correlation_id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO audit_events(id,correlation_id,actor_id,actor_assurance,operation,entity_type,entity_id,revision,outcome,detail_json,created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string())
        .bind(&correlation_id)
        .bind(actor.map(|value| value.subject_id.as_str()))
        .bind(actor.map_or("anonymous", |value| value.assurance_level.as_str()))
        .bind(entry.operation).bind(entry.entity_type).bind(entry.entity_id).bind(entry.revision).bind(entry.outcome)
        .bind(entry.detail.to_string()).bind(Utc::now().to_rfc3339()).execute(pool).await?;
    tracing::info!(
        event = entry.operation,
        correlation_id,
        entity_type = entry.entity_type,
        entity_id = entry.entity_id,
        outcome = entry.outcome
    );
    Ok(())
}
