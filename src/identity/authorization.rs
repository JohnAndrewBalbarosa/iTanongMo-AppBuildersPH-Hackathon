use super::{ActorContext, Role};

pub fn can_write_request(actor: &ActorContext, agency_id: &str) -> bool {
    actor.has_role(Role::PlatformAdmin)
        || actor
            .agency_id
            .as_deref()
            .is_some_and(|assigned| assigned == agency_id)
            && actor.roles.iter().any(|role| {
                matches!(
                    role,
                    Role::AgencyAdmin
                        | Role::ReceivingOfficer
                        | Role::DecisionMaker
                        | Role::AppealsAuthority
                )
            })
}

pub fn can_triage(actor: &ActorContext, agency_id: &str) -> bool {
    can_write_request(actor, agency_id)
        && actor
            .roles
            .iter()
            .any(|role| matches!(role, Role::AgencyAdmin | Role::ReceivingOfficer))
}

pub fn can_review_appeal(actor: &ActorContext, agency_id: &str) -> bool {
    can_write_request(actor, agency_id)
        && actor
            .roles
            .iter()
            .any(|role| matches!(role, Role::AgencyAdmin | Role::AppealsAuthority))
}

pub fn can_manage_staff(actor: &ActorContext, agency_id: &str) -> bool {
    actor.has_role(Role::PlatformAdmin)
        || actor.agency_id.as_deref() == Some(agency_id) && actor.has_role(Role::AgencyAdmin)
}
