//! In-app notifications. Email, SMS, and mobile push will implement a later delivery adapter.

use chrono::Utc;
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::error::AppResult;

#[derive(Clone, Debug, FromRow)]
pub struct Notification {
    pub id: String,
    pub request_id: Option<String>,
    pub kind: String,
    pub message: String,
    pub read_at: Option<String>,
    pub created_at: String,
}

pub async fn notify(
    pool: &SqlitePool,
    persona_id: &str,
    request_id: Option<&str>,
    kind: &str,
    message: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO notifications(id,persona_id,request_id,kind,message,created_at) VALUES (?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(persona_id).bind(request_id).bind(kind).bind(message).bind(Utc::now().to_rfc3339()).execute(pool).await?;
    Ok(())
}

pub async fn list_for_persona(pool: &SqlitePool, persona_id: &str) -> AppResult<Vec<Notification>> {
    Ok(sqlx::query_as::<_, Notification>("SELECT id,request_id,kind,message,read_at,created_at FROM notifications WHERE persona_id=? ORDER BY created_at DESC LIMIT 100")
        .bind(persona_id).fetch_all(pool).await?)
}
