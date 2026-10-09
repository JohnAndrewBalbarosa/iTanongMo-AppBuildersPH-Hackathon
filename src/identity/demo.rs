use sqlx::{FromRow, SqlitePool};

use crate::error::{AppError, AppResult};

use super::{ActorContext, ActorType, Role};

#[derive(Clone, Debug, FromRow)]
pub struct Persona {
    pub id: String,
    pub agency_id: Option<String>,
    pub organization_unit_id: Option<String>,
    pub display_name: String,
    pub role: String,
    pub actor_type: String,
}

pub async fn list_personas(pool: &SqlitePool) -> AppResult<Vec<Persona>> {
    Ok(sqlx::query_as::<_, Persona>(
        "SELECT id,agency_id,organization_unit_id,display_name,role,actor_type FROM demo_personas WHERE active=1 ORDER BY actor_type,display_name",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn load_actor(pool: &SqlitePool, persona_id: &str) -> AppResult<ActorContext> {
    let persona = sqlx::query_as::<_, Persona>(
        "SELECT id,agency_id,organization_unit_id,display_name,role,actor_type FROM demo_personas WHERE id=? AND active=1",
    )
    .bind(persona_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Forbidden)?;
    let role = Role::parse(&persona.role).ok_or(AppError::Forbidden)?;
    let actor_type = match persona.actor_type.as_str() {
        "requester" => ActorType::Requester,
        "staff" => ActorType::Staff,
        "platform_admin" => ActorType::PlatformAdmin,
        _ => return Err(AppError::Forbidden),
    };
    Ok(ActorContext {
        subject_id: persona.id,
        display_name: persona.display_name,
        actor_type,
        agency_id: persona.agency_id,
        organization_unit_id: persona.organization_unit_id,
        roles: vec![role],
        assurance_level: "demo".to_owned(),
        provider: "seeded_persona".to_owned(),
    })
}
