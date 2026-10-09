use std::sync::Arc;

use chrono::{Duration, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::{
    audit,
    error::{AppError, AppResult},
    identity::ActorContext,
};

#[derive(Clone, Debug, FromRow)]
pub struct AiDraftRow {
    pub id: String,
    pub status: String,
    pub output_json: Option<String>,
    pub output_hash: Option<String>,
    pub model_id: Option<String>,
    pub created_at: String,
}

use super::{AiProvider, schemas};

#[derive(Debug, PartialEq, Eq)]
pub enum AiJobOutcome {
    Idle,
    Completed,
    Deferred,
}

#[derive(FromRow)]
struct Job {
    id: String,
    request_id: String,
    task_kind: String,
    input_json: String,
    attempts: i64,
}

pub async fn process_next_job(
    pool: &SqlitePool,
    provider: Option<Arc<dyn AiProvider>>,
) -> AppResult<AiJobOutcome> {
    let Some(provider) = provider else {
        return Ok(AiJobOutcome::Deferred);
    };
    let now = Utc::now();
    let mut transaction = pool.begin().await?;
    let job = sqlx::query_as::<_, Job>("SELECT id,request_id,task_kind,input_json,attempts FROM ai_jobs WHERE status='pending' AND available_at<=? ORDER BY created_at LIMIT 1")
        .bind(now.to_rfc3339()).fetch_optional(&mut *transaction).await?;
    let Some(job) = job else {
        return Ok(AiJobOutcome::Idle);
    };
    let claimed = sqlx::query("UPDATE ai_jobs SET status='processing',claimed_at=?,attempts=attempts+1 WHERE id=? AND status='pending'")
        .bind(now.to_rfc3339()).bind(&job.id).execute(&mut *transaction).await?;
    if claimed.rows_affected() != 1 {
        return Ok(AiJobOutcome::Idle);
    }
    transaction.commit().await?;
    let payload: Value = serde_json::from_str(&job.input_json).map_err(|_| AppError::Internal)?;
    match provider.generate_json(&job.task_kind, payload).await {
        Ok(output) => complete_job(pool, &job, provider.model_id(), output).await,
        Err(error) => defer_job(pool, &job, &error.to_string()).await,
    }
}

async fn complete_job(
    pool: &SqlitePool,
    job: &Job,
    model_id: &str,
    output: Value,
) -> AppResult<AiJobOutcome> {
    let output = schemas::validate_output(&job.task_kind, output)?;
    let output_json = output.to_string();
    let output_hash = hex::encode(Sha256::digest(output_json.as_bytes()));
    let now = Utc::now().to_rfc3339();
    let mut transaction = pool.begin().await?;
    sqlx::query("UPDATE ai_jobs SET status='completed',output_json=?,output_hash=?,model_id=?,model_revision=?,completed_at=?,last_error=NULL WHERE id=? AND status='processing'")
        .bind(&output_json).bind(output_hash).bind(model_id).bind(model_id).bind(&now).bind(&job.id).execute(&mut *transaction).await?;
    if job.task_kind == "triage" {
        sqlx::query("UPDATE triage_results SET ai_status='ai_ready',ai_suggestion_json=?,updated_at=? WHERE request_id=?")
            .bind(&output_json).bind(&now).bind(&job.request_id).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    audit::record(
        pool,
        None,
        audit::Entry {
            operation: "ai.job_completed",
            entity_type: "ai_job",
            entity_id: &job.id,
            revision: None,
            outcome: "success",
            detail: serde_json::json!({"task_kind":job.task_kind,"model":model_id}),
        },
    )
    .await?;
    Ok(AiJobOutcome::Completed)
}

async fn defer_job(pool: &SqlitePool, job: &Job, error: &str) -> AppResult<AiJobOutcome> {
    let attempts = job.attempts + 1;
    let delay_minutes = 2_i64.pow(attempts.min(5) as u32);
    let next = Utc::now() + Duration::minutes(delay_minutes);
    let status = if attempts >= 5 {
        "manual_review"
    } else {
        "pending"
    };
    sqlx::query(
        "UPDATE ai_jobs SET status=?,available_at=?,last_error=?,claimed_at=NULL WHERE id=?",
    )
    .bind(status)
    .bind(next.to_rfc3339())
    .bind(error.chars().take(300).collect::<String>())
    .bind(&job.id)
    .execute(pool)
    .await?;
    tracing::warn!(event="ai.job_deferred", job_id=%job.id, task_kind=%job.task_kind, attempts);
    Ok(AiJobOutcome::Deferred)
}

pub async fn queue_government_draft(
    pool: &SqlitePool,
    actor: &ActorContext,
    request_id: &str,
    verified_facts: &str,
) -> AppResult<()> {
    if !(20..=8_000).contains(&verified_facts.trim().chars().count()) {
        return Err(AppError::Validation(
            "Magbigay ng 20–8,000 characters na verified facts".into(),
        ));
    }
    let payload = serde_json::json!({"request_id":request_id,"verified_facts":verified_facts,"source_record_ids":[]});
    let input_json = payload.to_string();
    let input_hash = hex::encode(Sha256::digest(input_json.as_bytes()));
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO ai_jobs(id,request_id,task_kind,status,input_json,input_hash,prompt_version,available_at,created_at) VALUES (?,?,'government_response','pending',?,?,'government-draft-v1',?,?)")
        .bind(Uuid::new_v4().to_string()).bind(request_id).bind(input_json).bind(input_hash).bind(&now).bind(&now).execute(pool).await?;
    audit::record(
        pool,
        Some(actor),
        audit::Entry {
            operation: "ai.draft_queued",
            entity_type: "foi_request",
            entity_id: request_id,
            revision: None,
            outcome: "success",
            detail: serde_json::json!({"task_kind":"government_response"}),
        },
    )
    .await
}
