use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    audit,
    error::{AppError, AppResult},
    identity::{ActorContext, Role, can_review_appeal},
};

use super::{FoiRequest, RequestEvent, add_working_days};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Acknowledge,
    RequestClarification,
    Route,
    BeginReview,
    Extend,
    Grant,
    PartiallyGrant,
    Deny,
    FileAppeal,
    ResolveAppeal,
    Close,
    Hide,
}

impl Action {
    pub fn parse(value: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::String(value.to_owned())).ok()
    }
}

pub fn allowed_actions(status: &str) -> &'static [&'static str] {
    match status {
        "submitted" => &["acknowledge", "request_clarification", "route", "hide"],
        "acknowledged" | "clarification_needed" => &["route", "begin_review", "hide"],
        "routed" => &["begin_review", "request_clarification", "hide"],
        "in_review" => &["extend", "grant", "partially_grant", "deny", "hide"],
        "extended" => &["grant", "partially_grant", "deny", "hide"],
        "granted" | "partially_granted" | "denied" => &["file_appeal", "close"],
        "appealed" => &["resolve_appeal"],
        "appeal_resolved" => &["close"],
        _ => &[],
    }
}

pub async fn apply_action(
    pool: &SqlitePool,
    actor: &ActorContext,
    request: &FoiRequest,
    action: Action,
    note: &str,
    destination_unit_id: Option<&str>,
) -> AppResult<()> {
    authorize(actor, request, action)?;
    let action_name = serde_json::to_value(action)
        .map_err(|_| AppError::Internal)?
        .as_str()
        .unwrap_or_default()
        .to_owned();
    if !allowed_actions(&request.status).contains(&action_name.as_str()) {
        return Err(AppError::Validation(
            "Hindi pinapayagan ang transition mula sa kasalukuyang status".into(),
        ));
    }
    if matches!(
        action,
        Action::PartiallyGrant | Action::Deny | Action::Extend | Action::Hide
    ) && note.trim().len() < 8
    {
        return Err(AppError::Validation(
            "Kailangan ng malinaw na reason".into(),
        ));
    }
    let target = target_status(action);
    let now = Utc::now();
    let next_revision = request.revision + 1;
    let extended_due_at = if action == Action::Extend {
        let base = request.due_at.parse().map_err(|_| AppError::Internal)?;
        Some(add_working_days(pool, base, 20).await?)
    } else {
        None
    };
    let mut transaction = pool.begin().await?;
    match action {
        Action::Route => {
            let destination = destination_unit_id
                .ok_or_else(|| AppError::Validation("Pumili ng destination unit".into()))?;
            let valid: Option<i64> = sqlx::query_scalar(
                "SELECT 1 FROM organization_units WHERE id=? AND agency_id=? AND active=1",
            )
            .bind(destination)
            .bind(&request.agency_id)
            .fetch_optional(&mut *transaction)
            .await?;
            if valid.is_none() {
                return Err(AppError::Validation("Invalid destination unit".into()));
            }
            sqlx::query("UPDATE foi_requests SET assigned_unit_id=? WHERE id=?")
                .bind(destination)
                .bind(&request.id)
                .execute(&mut *transaction)
                .await?;
        }
        Action::Extend => {
            let extended = extended_due_at.ok_or(AppError::Internal)?;
            sqlx::query("UPDATE foi_requests SET extended_due_at=? WHERE id=?")
                .bind(extended.to_string())
                .bind(&request.id)
                .execute(&mut *transaction)
                .await?;
        }
        Action::Grant | Action::PartiallyGrant | Action::Deny => {
            sqlx::query("UPDATE foi_requests SET disposition=?,disposition_reason=? WHERE id=?")
                .bind(target)
                .bind(note.trim())
                .bind(&request.id)
                .execute(&mut *transaction)
                .await?;
        }
        Action::Hide => {
            sqlx::query("UPDATE foi_requests SET hidden_at=?,hidden_reason=? WHERE id=?")
                .bind(now.to_rfc3339())
                .bind(note.trim())
                .bind(&request.id)
                .execute(&mut *transaction)
                .await?;
        }
        _ => {}
    }
    sqlx::query("UPDATE foi_requests SET status=?,revision=?,acknowledged_at=CASE WHEN ?='acknowledged' THEN COALESCE(acknowledged_at,?) ELSE acknowledged_at END,closed_at=CASE WHEN ?='closed' THEN ? ELSE closed_at END WHERE id=?")
        .bind(target).bind(next_revision).bind(target).bind(now.to_rfc3339()).bind(target).bind(now.to_rfc3339()).bind(&request.id).execute(&mut *transaction).await?;
    let summary = public_summary(action, note);
    sqlx::query("INSERT INTO request_events(id,request_id,actor_id,event_type,public_summary,from_status,to_status,created_at) VALUES (?,?,?,?,?,?,?,?)")
        .bind(Uuid::new_v4().to_string()).bind(&request.id).bind(&actor.subject_id).bind(action_name).bind(summary).bind(&request.status).bind(target).bind(now.to_rfc3339()).execute(&mut *transaction).await?;
    transaction.commit().await?;
    audit::record(
        pool,
        Some(actor),
        audit::Entry {
            operation: "request.transitioned",
            entity_type: "foi_request",
            entity_id: &request.id,
            revision: Some(next_revision),
            outcome: "success",
            detail: serde_json::json!({"from": request.status, "to": target}),
        },
    )
    .await
}

fn authorize(actor: &ActorContext, request: &FoiRequest, action: Action) -> AppResult<()> {
    if actor.has_role(Role::PlatformAdmin) {
        return Ok(());
    }
    let same_agency = actor.agency_id.as_deref() == Some(request.agency_id.as_str());
    let authorized = match action {
        Action::FileAppeal => actor.has_role(Role::Requester),
        Action::ResolveAppeal => can_review_appeal(actor, &request.agency_id),
        Action::Acknowledge
        | Action::RequestClarification
        | Action::Route
        | Action::BeginReview => {
            same_agency
                && (actor.has_role(Role::ReceivingOfficer) || actor.has_role(Role::AgencyAdmin))
        }
        Action::Extend | Action::Grant | Action::PartiallyGrant | Action::Deny | Action::Close => {
            same_agency
                && (actor.has_role(Role::DecisionMaker) || actor.has_role(Role::AgencyAdmin))
        }
        Action::Hide => same_agency && actor.has_role(Role::AgencyAdmin),
    };
    if authorized {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn target_status(action: Action) -> &'static str {
    match action {
        Action::Acknowledge => "acknowledged",
        Action::RequestClarification => "clarification_needed",
        Action::Route => "routed",
        Action::BeginReview => "in_review",
        Action::Extend => "extended",
        Action::Grant => "granted",
        Action::PartiallyGrant => "partially_granted",
        Action::Deny => "denied",
        Action::FileAppeal => "appealed",
        Action::ResolveAppeal => "appeal_resolved",
        Action::Close => "closed",
        Action::Hide => "hidden",
    }
}

fn public_summary(action: Action, note: &str) -> String {
    let label = match action {
        Action::Acknowledge => "Tinanggap ng agency ang request.",
        Action::RequestClarification => "Humiling ang agency ng paglilinaw.",
        Action::Route => "Ipinasa ang request sa responsible office.",
        Action::BeginReview => "Sinimulan ang substantive review.",
        Action::Extend => "Pinalawig ang processing period.",
        Action::Grant => "Granted ang request.",
        Action::PartiallyGrant => "Partially granted ang request.",
        Action::Deny => "Denied ang request.",
        Action::FileAppeal => "Naghain ng appeal.",
        Action::ResolveAppeal => "May decision na sa appeal.",
        Action::Close => "Isinara ang request.",
        Action::Hide => "Itinago ang public content habang nirereview.",
    };
    if note.trim().is_empty() {
        label.to_owned()
    } else {
        format!("{label} {}", note.trim())
    }
}

pub async fn list_events(pool: &SqlitePool, request_id: &str) -> AppResult<Vec<RequestEvent>> {
    Ok(sqlx::query_as::<_, RequestEvent>("SELECT event_type,public_summary,from_status,to_status,created_at FROM request_events WHERE request_id=? ORDER BY created_at")
        .bind(request_id).fetch_all(pool).await?)
}
