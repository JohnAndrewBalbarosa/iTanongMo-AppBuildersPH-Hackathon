use sqlx::{FromRow, SqlitePool};

use crate::error::AppResult;

#[derive(Clone, Debug, FromRow)]
pub struct OrganizationUnit {
    pub id: String,
    pub parent_id: Option<String>,
    pub slug: String,
    pub kind: String,
    pub name: String,
}

pub async fn organization_tree(
    pool: &SqlitePool,
    agency_id: &str,
) -> AppResult<Vec<OrganizationUnit>> {
    Ok(sqlx::query_as::<_, OrganizationUnit>("SELECT id,parent_id,slug,kind,name FROM organization_units WHERE agency_id=? AND active=1 ORDER BY parent_id,name")
        .bind(agency_id).fetch_all(pool).await?)
}
