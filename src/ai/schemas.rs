use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriageSuggestion {
    pub schema_version: String,
    pub request_id: String,
    pub suggested_categories: Vec<String>,
    pub suggested_destination_units: Vec<String>,
    pub urgency: String,
    pub complexity: String,
    pub sensitive_data_risk: String,
    pub clarification_questions: Vec<String>,
    pub supporting_field_offsets: Vec<SupportingOffset>,
    pub limitations: Vec<String>,
    pub requires_human_review: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupportingOffset {
    pub field: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSuggestion {
    pub schema_version: String,
    pub request_id: String,
    pub subject: String,
    pub body: String,
    pub source_record_ids: Vec<String>,
    pub limitations: Vec<String>,
    pub requires_human_review: bool,
}

pub fn validate_output(task_kind: &str, value: serde_json::Value) -> AppResult<serde_json::Value> {
    match task_kind {
        "triage" => {
            let parsed: TriageSuggestion = serde_json::from_value(value.clone())
                .map_err(|_| AppError::Validation("Hindi valid ang AI triage JSON".into()))?;
            if !parsed.requires_human_review
                || !matches!(
                    parsed.urgency.as_str(),
                    "normal" | "time_sensitive" | "safety_review"
                )
                || !matches!(
                    parsed.complexity.as_str(),
                    "simple" | "multi_unit" | "voluminous" | "unclear"
                )
                || !matches!(
                    parsed.sensitive_data_risk.as_str(),
                    "none" | "possible" | "likely"
                )
            {
                return Err(AppError::Validation(
                    "Lumabag ang AI triage sa safety contract".into(),
                ));
            }
        }
        "government_response" => {
            let parsed: DraftSuggestion = serde_json::from_value(value.clone())
                .map_err(|_| AppError::Validation("Hindi valid ang AI draft JSON".into()))?;
            if !parsed.requires_human_review || parsed.body.trim().len() < 20 {
                return Err(AppError::Validation(
                    "Lumabag ang AI draft sa human-review contract".into(),
                ));
            }
        }
        _ => return Err(AppError::Validation("Unknown AI task kind".into())),
    }
    Ok(value)
}

pub fn json_schema(task_kind: &str) -> serde_json::Value {
    match task_kind {
        "triage" => serde_json::json!({
            "type":"object","additionalProperties":false,
            "required":["schema_version","request_id","suggested_categories","suggested_destination_units","urgency","complexity","sensitive_data_risk","clarification_questions","supporting_field_offsets","limitations","requires_human_review"],
            "properties":{
                "schema_version":{"type":"string"},"request_id":{"type":"string"},
                "suggested_categories":{"type":"array","items":{"type":"string"}},
                "suggested_destination_units":{"type":"array","items":{"type":"string"}},
                "urgency":{"enum":["normal","time_sensitive","safety_review"]},
                "complexity":{"enum":["simple","multi_unit","voluminous","unclear"]},
                "sensitive_data_risk":{"enum":["none","possible","likely"]},
                "clarification_questions":{"type":"array","items":{"type":"string"}},
                "supporting_field_offsets":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["field","start","end"],"properties":{"field":{"type":"string"},"start":{"type":"integer"},"end":{"type":"integer"}}}},
                "limitations":{"type":"array","items":{"type":"string"}},
                "requires_human_review":{"const":true}
            }
        }),
        "government_response" => serde_json::json!({
            "type":"object","additionalProperties":false,
            "required":["schema_version","request_id","subject","body","source_record_ids","limitations","requires_human_review"],
            "properties":{
                "schema_version":{"type":"string"},"request_id":{"type":"string"},"subject":{"type":"string"},"body":{"type":"string"},
                "source_record_ids":{"type":"array","items":{"type":"string"}},"limitations":{"type":"array","items":{"type":"string"}},
                "requires_human_review":{"const":true}
            }
        }),
        _ => serde_json::json!({}),
    }
}
