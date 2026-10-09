use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};

pub async fn default_receiving_unit(pool: &SqlitePool, agency_id: &str) -> AppResult<String> {
    sqlx::query_scalar("SELECT id FROM organization_units WHERE agency_id=? AND kind='foi_office' AND active=1 ORDER BY id LIMIT 1")
        .bind(agency_id).fetch_optional(pool).await?.ok_or(AppError::NotFound)
}
