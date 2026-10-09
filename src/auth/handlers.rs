use std::time::Instant;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    app::AppState,
    error::{AppError, AppResult},
    identity,
};

use super::{
    model::{AuthUserRow, LoginInput, LogoutInput},
    password, session,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/session", get(current_session))
        .route("/api/v1/auth/logout", post(logout))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<LoginInput>,
) -> AppResult<Response> {
    let started = Instant::now();
    let correlation_id = correlation_id(&headers);
    let email = input.email.trim().to_lowercase();
    if email.is_empty() || input.password.trim().is_empty() {
        return Err(AppError::AuthValidation(
            "Email and password are required.".into(),
        ));
    }
    if state.auth.limiter.is_limited(&email) {
        tracing::warn!(
            event = "auth.login_rate_limited",
            component = "auth",
            operation = "login",
            outcome = "rejected",
            reason = "failure_window_exceeded",
            correlation_id = %correlation_id,
            duration_ms = started.elapsed().as_millis()
        );
        return Err(AppError::RateLimited);
    }

    let user = sqlx::query_as::<_, AuthUserRow>(
        "SELECT id,password_hash,persona_id,requester_id,active FROM auth_users WHERE email=? COLLATE NOCASE",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await?;
    let candidate_hash = user
        .as_ref()
        .map(|row| row.password_hash.clone())
        .unwrap_or_else(|| state.auth.dummy_password_hash.clone());
    let supplied_password = input.password.clone();
    let verified = tokio::task::spawn_blocking(move || {
        password::verify_password(&supplied_password, &candidate_hash)
    })
    .await
    .map_err(|_| AppError::Internal)?;
    let authenticated = user.filter(|row| verified && row.active == 1);
    let Some(user) = authenticated else {
        state.auth.limiter.record_failure(email);
        tracing::warn!(
            event = "auth.login_failed",
            component = "auth",
            operation = "login",
            outcome = "rejected",
            reason = "invalid_credentials",
            correlation_id = %correlation_id,
            duration_ms = started.elapsed().as_millis()
        );
        return Err(AppError::InvalidCredentials);
    };

    state.auth.limiter.clear(&email);
    if password::needs_rehash(&user.password_hash) {
        let password_value = input.password;
        let upgraded =
            tokio::task::spawn_blocking(move || password::hash_password(&password_value))
                .await
                .map_err(|_| AppError::Internal)??;
        sqlx::query("UPDATE auth_users SET password_hash=?,updated_at=? WHERE id=?")
            .bind(upgraded)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(&user.id)
            .execute(&state.pool)
            .await?;
    }
    let actor = identity::load_actor(&state.pool, &user.persona_id)
        .await
        .map_err(|_| AppError::InvalidCredentials)?;
    let created =
        session::create(&state.pool, &user.id, &user.persona_id, user.requester_id).await?;
    let mut response = Json(json!({
        "data": session_json(&actor, &created.csrf_token, created.context.expires_at)
    }))
    .into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        session::set_cookie_header(&created.session_token, &state.config.public_origin)?,
    );
    tracing::info!(
        event = "auth.login_succeeded",
        component = "auth",
        operation = "login",
        outcome = "success",
        actor_id = %actor.subject_id,
        user_id = %user.id,
        correlation_id = %correlation_id,
        duration_ms = started.elapsed().as_millis()
    );
    Ok(response)
}

async fn current_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let mut authenticated = session::required(&state.pool, &headers).await?;
    let csrf_token = session::rotate_csrf(&state.pool, &mut authenticated).await?;
    Ok(Json(json!({
        "data": session_json(
            &authenticated.actor,
            &csrf_token,
            authenticated.expires_at
        )
    })))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<LogoutInput>>,
) -> AppResult<Response> {
    let authenticated = session::required(&state.pool, &headers).await?;
    session::verify_csrf(
        &authenticated,
        &headers,
        body.as_ref().map(|Json(input)| input.csrf_token.as_str()),
    )?;
    session::revoke(&state.pool, &authenticated).await?;
    let mut response = Json(json!({"data":{"signed_out":true}})).into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        session::clear_cookie_header(&state.config.public_origin)?,
    );
    Ok(response)
}

pub fn session_json(
    actor: &crate::identity::ActorContext,
    csrf_token: &str,
    expires_at: chrono::DateTime<chrono::Utc>,
) -> serde_json::Value {
    json!({
        "actor": {
            "id": actor.subject_id,
            "display_name": actor.display_name,
            "actor_type": actor.actor_type,
            "agency_id": actor.agency_id,
            "organization_unit_id": actor.organization_unit_id,
            "roles": actor.roles.iter().map(crate::identity::Role::as_str).collect::<Vec<_>>()
        },
        "csrf_token": csrf_token,
        "expires_at": expires_at.to_rfc3339()
    })
}

fn correlation_id(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        })
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}
