use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Sqlite, SqlitePool, Transaction};

use crate::error::AppResult;

use super::password;

const CITIZEN_EMAIL: &str = "citizen.judge@itanongmo.local";
const CITIZEN_PASSWORD: &str = "CitizenJudge2026!";
const OFFICER_EMAIL: &str = "officer.judge@itanongmo.local";
const OFFICER_PASSWORD: &str = "OfficerJudge2026!";

pub async fn seed_demo_auth(pool: &SqlitePool) -> AppResult<()> {
    let citizen_hash = current_or_new_hash(pool, "auth-citizen-judge", CITIZEN_PASSWORD).await?;
    let officer_hash = current_or_new_hash(pool, "auth-officer-judge", OFFICER_PASSWORD).await?;
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let mut transaction = pool.begin().await?;

    sqlx::query("UPDATE demo_personas SET display_name='Juan Dela Cruz',role='requester',actor_type='requester',active=1 WHERE id='requester-demo'")
        .execute(&mut *transaction)
        .await?;
    sqlx::query("INSERT INTO requesters(id,display_name,contact_email,assurance_level,consent_version,created_at) VALUES ('requester-demo','Juan Dela Cruz',?,'demo_account','demo-v2',?) ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name,contact_email=excluded.contact_email,assurance_level=excluded.assurance_level")
        .bind(CITIZEN_EMAIL)
        .bind(&now_text)
        .execute(&mut *transaction)
        .await?;

    upsert_auth_user(
        &mut transaction,
        "auth-citizen-judge",
        CITIZEN_EMAIL,
        &citizen_hash,
        "requester-demo",
        Some("requester-demo"),
        &now_text,
    )
    .await?;
    upsert_auth_user(
        &mut transaction,
        "auth-officer-judge",
        OFFICER_EMAIL,
        &officer_hash,
        "dost-fro",
        None,
        &now_text,
    )
    .await?;
    seed_citizen_requests(&mut transaction, now).await?;
    transaction.commit().await?;
    Ok(())
}

async fn current_or_new_hash(
    pool: &SqlitePool,
    user_id: &str,
    password_value: &str,
) -> AppResult<String> {
    let existing =
        sqlx::query_scalar::<_, String>("SELECT password_hash FROM auth_users WHERE id=?")
            .bind(user_id)
            .fetch_optional(pool)
            .await?;
    if let Some(existing) = existing
        && password::verify_password(password_value, &existing)
        && !password::needs_rehash(&existing)
    {
        return Ok(existing);
    }
    password::hash_password(password_value)
}

async fn upsert_auth_user(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &str,
    email: &str,
    password_hash: &str,
    persona_id: &str,
    requester_id: Option<&str>,
    now: &str,
) -> AppResult<()> {
    sqlx::query("INSERT INTO auth_users(id,email,password_hash,persona_id,requester_id,active,created_at,updated_at) VALUES (?,?,?,?,?,1,?,?) ON CONFLICT(id) DO UPDATE SET email=excluded.email,password_hash=excluded.password_hash,persona_id=excluded.persona_id,requester_id=excluded.requester_id,active=1,updated_at=excluded.updated_at")
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(persona_id)
        .bind(requester_id)
        .bind(now)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn seed_citizen_requests(
    transaction: &mut Transaction<'_, Sqlite>,
    now: chrono::DateTime<Utc>,
) -> AppResult<()> {
    let submitted_at = (now - Duration::days(4)).to_rfc3339();
    sqlx::query("INSERT OR IGNORE INTO request_batches(id,batch_reference,requester_id,created_at,requester_access_hash) VALUES ('auth-demo-batch','BATCH-AUTH-DEMO-2026','requester-demo',?,NULL)")
        .bind(&submitted_at)
        .execute(&mut **transaction)
        .await?;
    let samples = [
        (
            "auth-demo-request-new",
            "FOI-2026-001",
            "agency-doh",
            "doh-receiving",
            "Public hospital medicine inventory",
            "Current inventory and replenishment records for essential medicines in public hospitals.",
            "Public transparency and service monitoring.",
            "submitted",
        ),
        (
            "auth-demo-request-review",
            "FOI-2026-002",
            "agency-dost",
            "dost-receiving",
            "Regional science scholarship statistics",
            "Annual scholarship applications, approvals, and completion statistics by region.",
            "Academic research on access to science education.",
            "acknowledged",
        ),
        (
            "auth-demo-request-response",
            "FOI-2026-003",
            "agency-dswd",
            "dswd-receiving",
            "Disaster assistance distribution summary",
            "Aggregate distribution counts for disaster assistance without beneficiary identifiers.",
            "Community preparedness review.",
            "in_review",
        ),
    ];
    for (index, sample) in samples.iter().enumerate() {
        let due_at = (now + Duration::days(11 + index as i64))
            .date_naive()
            .to_string();
        let acknowledged_at = (sample.7 != "submitted").then_some(&submitted_at);
        sqlx::query("INSERT OR IGNORE INTO foi_requests(id,public_reference,agency_id,assigned_unit_id,requester_id,title,description,purpose,urgency_details,status,submitted_at,acknowledged_at,due_at,batch_id,demo) VALUES (?,?,?,?,?,?,?,?,?, ?,?,?,?,?,1)")
            .bind(sample.0)
            .bind(sample.1)
            .bind(sample.2)
            .bind(sample.3)
            .bind("requester-demo")
            .bind(sample.4)
            .bind(sample.5)
            .bind(sample.6)
            .bind("")
            .bind(sample.7)
            .bind(&submitted_at)
            .bind(acknowledged_at)
            .bind(due_at)
            .bind("auth-demo-batch")
            .execute(&mut **transaction)
            .await?;
        sqlx::query("INSERT OR IGNORE INTO request_events(id,request_id,actor_id,event_type,public_summary,from_status,to_status,created_at) VALUES (?,?,NULL,'submitted','Isinumite ang FOI request.',NULL,'submitted',?)")
            .bind(format!("{}-submitted", sample.0))
            .bind(sample.0)
            .bind(&submitted_at)
            .execute(&mut **transaction)
            .await?;
    }
    seed_progress_event(
        transaction,
        "auth-demo-request-review",
        "acknowledged",
        "submitted",
        "acknowledged",
        "Tinanggap ng receiving officer ang request.",
        &submitted_at,
    )
    .await?;
    seed_progress_event(
        transaction,
        "auth-demo-request-response",
        "in-review",
        "acknowledged",
        "in_review",
        "Sinimulan ng agency ang records review.",
        &submitted_at,
    )
    .await?;
    let message = "The request is under records review. A public response will appear here.";
    sqlx::query("INSERT OR IGNORE INTO messages(id,request_id,actor_id,actor_label,visibility,body,content_hash,created_at) VALUES ('auth-demo-public-message','auth-demo-request-response','dswd-receiving','DSWD FOI Receiving Office','public',?,?,?)")
        .bind(message)
        .bind(hex::encode(Sha256::digest(message.as_bytes())))
        .bind(&submitted_at)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn seed_progress_event(
    transaction: &mut Transaction<'_, Sqlite>,
    request_id: &str,
    suffix: &str,
    from_status: &str,
    to_status: &str,
    summary: &str,
    created_at: &str,
) -> AppResult<()> {
    sqlx::query("INSERT OR IGNORE INTO request_events(id,request_id,actor_id,event_type,public_summary,from_status,to_status,created_at) VALUES (?,?,'dost-fro',?,?,?,?,?)")
        .bind(format!("{request_id}-{suffix}"))
        .bind(request_id)
        .bind(to_status)
        .bind(summary)
        .bind(from_status)
        .bind(to_status)
        .bind(created_at)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}
