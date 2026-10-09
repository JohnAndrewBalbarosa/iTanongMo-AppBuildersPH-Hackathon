use sqlx::FromRow;

#[derive(Debug, serde::Deserialize)]
pub struct LoginInput {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct LogoutInput {
    #[serde(default)]
    pub csrf_token: String,
}

#[derive(Clone, Debug, FromRow)]
pub struct AuthUserRow {
    pub id: String,
    pub password_hash: String,
    pub persona_id: String,
    pub requester_id: Option<String>,
    pub active: i64,
}

#[derive(Clone, Debug, FromRow)]
pub struct SessionRow {
    pub session_id: String,
    pub user_id: String,
    pub csrf_hash: String,
    pub expires_at: String,
    pub persona_id: String,
    pub requester_id: Option<String>,
    pub user_active: i64,
}
