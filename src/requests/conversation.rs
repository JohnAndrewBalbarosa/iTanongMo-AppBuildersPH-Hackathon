use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::{
    audit,
    error::{AppError, AppResult},
    identity::ActorContext,
    notifications,
};

#[derive(Clone, Debug, FromRow)]
pub struct Message {
    pub id: String,
    pub actor_label: String,
    pub visibility: String,
    pub body: String,
    pub created_at: String,
}

pub async fn add_message(
    pool: &SqlitePool,
    actor: &ActorContext,
    request_id: &str,
    visibility: &str,
    body: &str,
) -> AppResult<()> {
    let body = body.trim();
    if !(2..=5_000).contains(&body.chars().count()) {
        return Err(AppError::Validation(
            "Ang message ay dapat 2–5,000 characters".into(),
        ));
    }
    if !matches!(visibility, "public" | "internal") {
        return Err(AppError::Validation("Invalid message visibility".into()));
    }
    let hash = hex::encode(Sha256::digest(body.as_bytes()));
    sqlx::query("INSERT INTO messages(id,request_id,actor_id,actor_label,visibility,body,content_hash,created_at) VALUES (?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(request_id).bind(&actor.subject_id).bind(&actor.display_name).bind(visibility).bind(body).bind(hash).bind(Utc::now().to_rfc3339()).execute(pool).await?;
    if visibility == "public" {
        notify_involved_personas(pool, actor, request_id).await?;
    }
    audit::record(
        pool,
        Some(actor),
        audit::Entry {
            operation: "request.message_added",
            entity_type: "foi_request",
            entity_id: request_id,
            revision: None,
            outcome: "success",
            detail: serde_json::json!({"visibility": visibility}),
        },
    )
    .await
}

async fn notify_involved_personas(
    pool: &SqlitePool,
    actor: &ActorContext,
    request_id: &str,
) -> AppResult<()> {
    let agency_id: String = sqlx::query_scalar("SELECT agency_id FROM foi_requests WHERE id=?")
        .bind(request_id)
        .fetch_one(pool)
        .await?;
    let personas: Vec<String> = sqlx::query_scalar("SELECT id FROM demo_personas WHERE active=1 AND (actor_type='requester' OR agency_id=?) AND id<>?")
        .bind(agency_id).bind(&actor.subject_id).fetch_all(pool).await?;
    for persona in personas {
        notifications::notify(
            pool,
            &persona,
            Some(request_id),
            "request.message",
            "May bagong public message sa FOI request.",
        )
        .await?;
    }
    Ok(())
}

pub async fn list_messages(
    pool: &SqlitePool,
    request_id: &str,
    include_internal: bool,
) -> AppResult<Vec<Message>> {
    let sql = if include_internal {
        "SELECT id,actor_label,visibility,body,created_at FROM messages WHERE request_id=? ORDER BY created_at"
    } else {
        "SELECT id,actor_label,visibility,body,created_at FROM messages WHERE request_id=? AND visibility='public' ORDER BY created_at"
    };
    Ok(sqlx::query_as::<_, Message>(sql)
        .bind(request_id)
        .fetch_all(pool)
        .await?)
}
