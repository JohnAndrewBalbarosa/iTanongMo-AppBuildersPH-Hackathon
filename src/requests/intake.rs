use std::collections::HashSet;

use chrono::{Duration, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    audit,
    error::{AppError, AppResult},
    organization::default_receiving_unit,
    triage,
};

use super::{FoiRequest, PublicRequestRow, SubmissionBatch, initial_due_date};

const MAX_AGENCIES_PER_BATCH: usize = 10;

#[derive(Clone, Debug, Deserialize)]
pub struct NewRequest {
    pub agency_ids: String,
    pub requester_name: String,
    pub requester_email: String,
    pub title: String,
    pub description: String,
    pub purpose: String,
    #[serde(default)]
    pub urgency_details: String,
    #[serde(default)]
    pub consent: String,
}

#[derive(Clone)]
struct AgencyDestination {
    id: String,
    short_name: String,
    receiving_unit_id: String,
}

pub async fn create_request(pool: &SqlitePool, input: NewRequest) -> AppResult<SubmissionBatch> {
    let agency_ids = selected_agency_ids(&input.agency_ids)?;
    validate(&input, agency_ids.len())?;
    enforce_submission_rate(pool, &input.requester_email).await?;
    let agencies = load_destinations(pool, &agency_ids).await?;
    let now = Utc::now();
    let due = initial_due_date(pool, now.date_naive()).await?;
    let requester_id = Uuid::new_v4().to_string();
    let batch_id = Uuid::new_v4().to_string();
    let batch_reference = batch_reference();
    let requester_access_token = requester_access_token();
    let requester_access_hash = hex::encode(Sha256::digest(requester_access_token.as_bytes()));
    let mut created_request_ids = Vec::with_capacity(agencies.len());
    let mut transaction = pool.begin().await?;

    sqlx::query("INSERT INTO requesters(id,display_name,contact_email,consent_version,created_at) VALUES (?,?,?,?,?)")
        .bind(&requester_id).bind(input.requester_name.trim()).bind(input.requester_email.trim()).bind("demo-v2").bind(now.to_rfc3339()).execute(&mut *transaction).await?;
    sqlx::query(
        "INSERT INTO request_batches(id,batch_reference,requester_id,created_at,requester_access_hash) VALUES (?,?,?,?,?)",
    )
    .bind(&batch_id)
    .bind(&batch_reference)
    .bind(&requester_id)
    .bind(now.to_rfc3339())
    .bind(requester_access_hash)
    .execute(&mut *transaction)
    .await?;

    for agency in &agencies {
        let request_id = Uuid::new_v4().to_string();
        let reference = public_reference(&agency.short_name);
        sqlx::query("INSERT INTO foi_requests(id,public_reference,agency_id,assigned_unit_id,requester_id,title,description,purpose,urgency_details,status,submitted_at,due_at,batch_id) VALUES (?,?,?,?,?,?,?,?,?,'submitted',?,?,?)")
            .bind(&request_id).bind(&reference).bind(&agency.id).bind(&agency.receiving_unit_id).bind(&requester_id).bind(input.title.trim()).bind(input.description.trim()).bind(input.purpose.trim()).bind(input.urgency_details.trim()).bind(now.to_rfc3339()).bind(due.to_string()).bind(&batch_id).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO request_events(id,request_id,event_type,public_summary,to_status,created_at) VALUES (?,?,'submitted','Isinumite ang FOI request bilang bahagi ng multi-agency submission.','submitted',?)")
            .bind(Uuid::new_v4().to_string()).bind(&request_id).bind(now.to_rfc3339()).execute(&mut *transaction).await?;
        created_request_ids.push(request_id);
    }
    transaction.commit().await?;

    for request_id in &created_request_ids {
        triage::run_rules_and_queue_ai(pool, request_id, &input).await?;
    }
    audit::record(pool, None, audit::Entry {
        operation: "request.batch_submitted", entity_type: "request_batch", entity_id: &batch_id,
        revision: Some(1), outcome: "success",
        detail: serde_json::json!({"batch_reference":batch_reference,"agency_count":agencies.len(),"demo":true}),
    }).await?;
    let mut batch = load_submission_batch(pool, &batch_reference).await?;
    batch.requester_access_token = Some(requester_access_token);
    Ok(batch)
}

pub async fn load_submission_batch(
    pool: &SqlitePool,
    reference: &str,
) -> AppResult<SubmissionBatch> {
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT id,batch_reference,created_at FROM request_batches WHERE batch_reference=?",
    )
    .bind(reference)
    .fetch_optional(pool)
    .await?;
    let (id, batch_reference, created_at) = row.ok_or(AppError::NotFound)?;
    let requests = sqlx::query_as::<_, PublicRequestRow>("SELECT r.public_reference,a.short_name agency_short_name,r.title,r.status,r.submitted_at,r.due_at FROM foi_requests r JOIN agencies a ON a.id=r.agency_id WHERE r.batch_id=? AND r.hidden_at IS NULL ORDER BY a.name")
        .bind(&id).fetch_all(pool).await?;
    Ok(SubmissionBatch {
        id,
        batch_reference,
        created_at,
        requests,
        requester_access_token: None,
    })
}

fn selected_agency_ids(value: &str) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let selected: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .filter(|id| seen.insert((*id).to_owned()))
        .map(str::to_owned)
        .collect();
    if selected.is_empty() || selected.len() > MAX_AGENCIES_PER_BATCH {
        return Err(AppError::Validation(
            "Pumili ng 1–10 government agencies".into(),
        ));
    }
    Ok(selected)
}

async fn load_destinations(
    pool: &SqlitePool,
    agency_ids: &[String],
) -> AppResult<Vec<AgencyDestination>> {
    let mut destinations = Vec::with_capacity(agency_ids.len());
    for agency_id in agency_ids {
        let agency: Option<(String, String)> =
            sqlx::query_as("SELECT id,short_name FROM agencies WHERE id=? AND active=1")
                .bind(agency_id)
                .fetch_optional(pool)
                .await?;
        let (id, short_name) =
            agency.ok_or_else(|| AppError::Validation("May invalid agency selection".into()))?;
        let receiving_unit_id = default_receiving_unit(pool, &id).await?;
        destinations.push(AgencyDestination {
            id,
            short_name,
            receiving_unit_id,
        });
    }
    Ok(destinations)
}

fn validate(input: &NewRequest, agency_count: usize) -> AppResult<()> {
    let required = [
        ("pangalan", input.requester_name.trim(), 2, 120),
        ("email", input.requester_email.trim(), 5, 254),
        ("pamagat", input.title.trim(), 5, 180),
        ("request", input.description.trim(), 20, 8_000),
        ("layunin", input.purpose.trim(), 3, 1_000),
    ];
    for (label, value, minimum, maximum) in required {
        if !(minimum..=maximum).contains(&value.chars().count()) {
            return Err(AppError::Validation(format!("Invalid ang haba ng {label}")));
        }
    }
    if !(1..=MAX_AGENCIES_PER_BATCH).contains(&agency_count) {
        return Err(AppError::Validation(
            "Pumili ng 1–10 government agencies".into(),
        ));
    }
    if !input.requester_email.contains('@') || input.requester_email.contains(char::is_whitespace) {
        return Err(AppError::Validation(
            "Maglagay ng valid email address".into(),
        ));
    }
    if input.consent != "yes" {
        return Err(AppError::Validation(
            "Kailangan ang demo privacy acknowledgment".into(),
        ));
    }
    if input.urgency_details.chars().count() > 1_000 {
        return Err(AppError::Validation(
            "Masyadong mahaba ang urgency details".into(),
        ));
    }
    Ok(())
}

pub async fn enforce_submission_rate(pool: &SqlitePool, email: &str) -> AppResult<()> {
    let subject_hash = hex::encode(Sha256::digest(email.trim().to_lowercase().as_bytes()));
    let now = Utc::now();
    let current: Option<(String, i64)> = sqlx::query_as("SELECT window_started_at,attempts FROM rate_limits WHERE subject_hash=? AND operation='request.submit'")
        .bind(&subject_hash).fetch_optional(pool).await?;
    match current {
        Some((started, attempts))
            if started
                .parse::<chrono::DateTime<Utc>>()
                .is_ok_and(|value| now - value < Duration::hours(1)) =>
        {
            if attempts >= 5 {
                return Err(AppError::RateLimited);
            }
            sqlx::query("UPDATE rate_limits SET attempts=attempts+1 WHERE subject_hash=? AND operation='request.submit'").bind(subject_hash).execute(pool).await?;
        }
        _ => {
            sqlx::query("INSERT INTO rate_limits(subject_hash,operation,window_started_at,attempts) VALUES (?,'request.submit',?,1) ON CONFLICT(subject_hash,operation) DO UPDATE SET window_started_at=excluded.window_started_at,attempts=1")
                .bind(subject_hash).bind(now.to_rfc3339()).execute(pool).await?;
        }
    }
    Ok(())
}

fn batch_reference() -> String {
    let suffix = Uuid::new_v4().simple().to_string()[..12].to_uppercase();
    format!("FOI-BATCH-{suffix}")
}

fn public_reference(short_name: &str) -> String {
    let suffix = Uuid::new_v4().simple().to_string()[..12].to_uppercase();
    format!("{}-{suffix}", short_name.to_uppercase())
}

fn requester_access_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub async fn load_request(pool: &SqlitePool, reference: &str) -> AppResult<FoiRequest> {
    sqlx::query_as::<_, FoiRequest>("SELECT r.id,r.public_reference,r.agency_id,a.name agency_name,r.assigned_unit_id,r.requester_id,r.title,r.description,r.purpose,r.urgency_details,r.status,r.disposition,r.disposition_reason,r.submitted_at,r.due_at,r.extended_due_at,r.revision FROM foi_requests r JOIN agencies a ON a.id=r.agency_id WHERE r.public_reference=? AND r.hidden_at IS NULL")
        .bind(reference).fetch_optional(pool).await?.ok_or(AppError::NotFound)
}

pub async fn list_public_requests(pool: &SqlitePool) -> AppResult<Vec<PublicRequestRow>> {
    Ok(sqlx::query_as::<_, PublicRequestRow>("SELECT r.public_reference,a.short_name agency_short_name,r.title,r.status,r.submitted_at,r.due_at FROM foi_requests r JOIN agencies a ON a.id=r.agency_id WHERE r.hidden_at IS NULL ORDER BY r.submitted_at DESC LIMIT 100")
        .fetch_all(pool).await?)
}
