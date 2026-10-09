use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    Requester,
    Staff,
    PlatformAdmin,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Requester,
    AgencyAdmin,
    ReceivingOfficer,
    DecisionMaker,
    AppealsAuthority,
    Auditor,
    PlatformAdmin,
}

impl Role {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "requester" => Self::Requester,
            "agency_admin" => Self::AgencyAdmin,
            "receiving_officer" => Self::ReceivingOfficer,
            "decision_maker" => Self::DecisionMaker,
            "appeals_authority" => Self::AppealsAuthority,
            "auditor" => Self::Auditor,
            "platform_admin" => Self::PlatformAdmin,
            _ => return None,
        })
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requester => "requester",
            Self::AgencyAdmin => "agency_admin",
            Self::ReceivingOfficer => "receiving_officer",
            Self::DecisionMaker => "decision_maker",
            Self::AppealsAuthority => "appeals_authority",
            Self::Auditor => "auditor",
            Self::PlatformAdmin => "platform_admin",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorContext {
    pub subject_id: String,
    pub display_name: String,
    pub actor_type: ActorType,
    pub agency_id: Option<String>,
    pub organization_unit_id: Option<String>,
    pub roles: Vec<Role>,
    pub assurance_level: String,
    pub provider: String,
}

impl ActorContext {
    pub fn has_role(&self, role: Role) -> bool {
        self.roles.contains(&role)
    }
}
