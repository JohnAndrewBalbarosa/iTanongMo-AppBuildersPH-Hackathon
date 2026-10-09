use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use crate::{error::AppResult, requests::NewRequest};

const RULES_VERSION: &str = "2026-10-high-recall-v1";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleHit {
    pub rule_id: String,
    pub category: String,
    pub source_field: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, FromRow)]
pub struct TriageView {
    pub rules_version: String,
    pub rule_hits_json: String,
    pub ai_status: String,
    pub ai_suggestion_json: Option<String>,
    pub human_categories_json: Option<String>,
}

struct Rule {
    id: &'static str,
    category: &'static str,
    pattern: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        id: "time-sensitive",
        category: "time_sensitive",
        pattern: r"(?i)\b(asap|urgent|immediately|deadline|agad|madalian|kailangan ngayon|before \d{1,2}[/-]\d{1,2})\b",
    },
    Rule {
        id: "safety",
        category: "safety_review",
        pattern: r"(?i)\b(danger|death|threat|hazard|emergency|sakuna|panganib|banta|kaligtasan)\b",
    },
    Rule {
        id: "personal-data",
        category: "sensitive_data",
        pattern: r"(?i)\b(address|birthdate|medical|salary|mobile number|personal data|tirahan|kapanganakan|kalusugan|sweldo)\b",
    },
    Rule {
        id: "large-range",
        category: "voluminous",
        pattern: r"(?i)\b(all records|complete records|every document|lahat ng record|mula 19\d\d|mula 20\d\d)\b",
    },
    Rule {
        id: "multi-agency",
        category: "multi_unit",
        pattern: r"(?i)\b(all agencies|every department|multiple agencies|lahat ng ahensya|iba't ibang ahensya)\b",
    },
    Rule {
        id: "appeal-language",
        category: "appeal",
        pattern: r"(?i)\b(appeal|denied|denial|reconsider|apela|tinanggihan|muling suriin)\b",
    },
    Rule {
        id: "preservation",
        category: "records_preservation",
        pattern: r"(?i)\b(delete|destroy|preserve|retain|burahin|sirain|panatilihin)\b",
    },
    Rule {
        id: "unclear-record",
        category: "clarification",
        pattern: r"(?i)\b(information about|anything about|details about|impormasyon tungkol|kahit anong detalye)\b",
    },
];

pub async fn run_rules_and_queue_ai(
    pool: &SqlitePool,
    request_id: &str,
    input: &NewRequest,
) -> AppResult<Vec<RuleHit>> {
    let fields = [
        ("title", input.title.as_str()),
        ("description", input.description.as_str()),
        ("purpose", input.purpose.as_str()),
        ("urgency_details", input.urgency_details.as_str()),
    ];
    let mut hits = Vec::new();
    for rule in RULES {
        let expression = Regex::new(rule.pattern).expect("static triage regex is valid");
        for (field, value) in fields {
            for matched in expression.find_iter(value) {
                hits.push(RuleHit {
                    rule_id: rule.id.into(),
                    category: rule.category.into(),
                    source_field: field.into(),
                    start: matched.start(),
                    end: matched.end(),
                });
            }
        }
    }
    if !input.urgency_details.trim().is_empty()
        && !hits.iter().any(|hit| hit.source_field == "urgency_details")
    {
        hits.push(RuleHit {
            rule_id: "form-urgency-details".into(),
            category: "time_sensitive".into(),
            source_field: "urgency_details".into(),
            start: 0,
            end: input.urgency_details.len(),
        });
    }
    let now = Utc::now().to_rfc3339();
    let status = if hits.is_empty() {
        "not_flagged"
    } else {
        "pending_ai"
    };
    let hits_json = serde_json::to_string(&hits).expect("rule hits serialize");
    sqlx::query("INSERT INTO triage_results(request_id,rules_version,rule_hits_json,ai_status,updated_at) VALUES (?,?,?,?,?)")
        .bind(request_id).bind(RULES_VERSION).bind(&hits_json).bind(status).bind(&now).execute(pool).await?;
    if !hits.is_empty() {
        let payload = serde_json::json!({"request_id": request_id, "title": input.title, "description": input.description, "purpose": input.purpose, "urgency_details": input.urgency_details, "rule_hits": hits});
        let input_json = payload.to_string();
        let input_hash = hex::encode(Sha256::digest(input_json.as_bytes()));
        sqlx::query("INSERT OR IGNORE INTO ai_jobs(id,request_id,task_kind,status,input_json,input_hash,prompt_version,available_at,created_at) VALUES (?,?,'triage','pending',?,?,'triage-v1',?,?)")
            .bind(Uuid::new_v4().to_string()).bind(request_id).bind(input_json).bind(input_hash).bind(&now).bind(&now).execute(pool).await?;
    }
    Ok(hits)
}

pub async fn load_triage(pool: &SqlitePool, request_id: &str) -> AppResult<Option<TriageView>> {
    Ok(sqlx::query_as::<_, TriageView>("SELECT rules_version,rule_hits_json,ai_status,ai_suggestion_json,human_categories_json FROM triage_results WHERE request_id=?")
        .bind(request_id).fetch_optional(pool).await?)
}
