use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequesterAiInput {
    pub what_to_ask: String,
    pub manual_title: String,
}

impl RequesterAiInput {
    pub fn validate(&self) -> AppResult<()> {
        if !(20..=8_000).contains(&self.what_to_ask.trim().chars().count())
            || self.what_to_ask.chars().count() > 8_000
        {
            return Err(AppError::Validation(
                "The AI request must be 20–8,000 characters".into(),
            ));
        }
        if !(5..=180).contains(&self.manual_title.trim().chars().count())
            || self.manual_title.chars().count() > 180
        {
            return Err(AppError::Validation(
                "Enter the manual request title first".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequesterAiDraft {
    pub email_body: String,
    pub description: String,
    pub purpose: String,
    pub urgency_details: String,
    pub agency_queries: Vec<String>,
}

impl RequesterAiDraft {
    pub fn validate(self) -> AppResult<Self> {
        let valid_lengths = (20..=12_000).contains(&self.email_body.trim().chars().count())
            && self.email_body.chars().count() <= 12_000
            && (20..=8_000).contains(&self.description.trim().chars().count())
            && self.description.chars().count() <= 8_000
            && (3..=1_000).contains(&self.purpose.trim().chars().count())
            && self.purpose.chars().count() <= 1_000
            && self.urgency_details.chars().count() <= 1_000;
        let valid_agencies = !self.agency_queries.is_empty()
            && self.agency_queries.len() <= 10
            && self.agency_queries.iter().all(|query| {
                (2..=120).contains(&query.trim().chars().count()) && query.chars().count() <= 120
            });
        if !valid_lengths || !valid_agencies {
            return Err(AppError::Validation("Invalid AI form values".into()));
        }
        Ok(self)
    }
}

pub fn response_schema() -> serde_json::Value {
    serde_json::json!({
        "type":"object","additionalProperties":false,
        "required":["email_body","description","purpose","urgency_details","agency_queries"],
        "properties":{
            "email_body":{"type":"string"},
            "description":{"type":"string"},
            "purpose":{"type":"string"},
            "urgency_details":{"type":"string"},
            "agency_queries":{"type":"array","minItems":1,"maxItems":10,"items":{"type":"string"}}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::{RequesterAiDraft, RequesterAiInput, response_schema};

    #[test]
    fn one_call_contract_excludes_manual_identity_and_title_fields() {
        let schema = response_schema();
        let properties = schema["properties"].as_object().unwrap();
        assert!(!properties.contains_key("requester_name"));
        assert!(!properties.contains_key("title"));

        let input = RequesterAiInput {
            what_to_ask: "Please request the latest public program reports.".into(),
            manual_title: "Public program reports".into(),
        };
        assert!(input.validate().is_ok());
    }

    #[test]
    fn padded_input_cannot_exceed_the_provider_input_budget() {
        let input = RequesterAiInput {
            what_to_ask: format!("Please provide public program reports.{}", " ".repeat(8000)),
            manual_title: "Public program reports".into(),
        };
        assert!(input.validate().is_err());
    }

    #[test]
    fn one_call_draft_requires_authoritative_agency_queries() {
        let draft = RequesterAiDraft {
            email_body: "Please provide the public records described in this request.".into(),
            description: "Latest public program reports and their supporting summaries.".into(),
            purpose: "Public-interest research".into(),
            urgency_details: String::new(),
            agency_queries: vec!["Department of Science and Technology".into()],
        };
        assert!(draft.validate().is_ok());
    }
}
