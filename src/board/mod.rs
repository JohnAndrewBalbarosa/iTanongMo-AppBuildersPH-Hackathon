//! Government Kanban projection. Columns are derived from authoritative request status.

use sqlx::{FromRow, SqlitePool};

use crate::error::AppResult;

#[derive(Clone, Debug, FromRow)]
pub struct BoardCard {
    pub public_reference: String,
    pub title: String,
    pub status: String,
    pub due_at: String,
    pub agency_id: String,
    pub unit_name: Option<String>,
    pub ai_status: Option<String>,
    pub rule_hits_json: Option<String>,
}

impl BoardCard {
    pub fn column(&self) -> &'static str {
        match self.status.as_str() {
            "submitted" => "New",
            "acknowledged" => "Needs Triage",
            "clarification_needed" => "Needs Clarification",
            "routed" => "Routed",
            "in_review" | "extended" => "In Review",
            "granted" | "partially_granted" | "denied" => "Ready to Release",
            "appealed" | "appeal_resolved" => "Appeal",
            "closed" => "Released/Closed",
            _ => "New",
        }
    }

    pub fn has_rule_hits(&self) -> bool {
        self.rule_hits_json
            .as_deref()
            .is_some_and(|value| value != "[]")
    }
}

pub async fn load_board(pool: &SqlitePool, agency_id: &str) -> AppResult<Vec<BoardCard>> {
    Ok(sqlx::query_as::<_, BoardCard>("SELECT r.public_reference,r.title,r.status,r.due_at,r.agency_id,u.name unit_name,t.ai_status,t.rule_hits_json FROM foi_requests r LEFT JOIN organization_units u ON u.id=r.assigned_unit_id LEFT JOIN triage_results t ON t.request_id=r.id WHERE r.agency_id=? ORDER BY r.due_at,r.submitted_at")
        .bind(agency_id).fetch_all(pool).await?)
}
