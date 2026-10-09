use axum::http::{HeaderMap, HeaderValue, header};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use cookie::{Cookie, SameSite, time::OffsetDateTime};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    identity::{self, ActorContext, ActorType},
};

use super::model::SessionRow;

pub const SESSION_COOKIE: &str = "itanongmo_session";
const SESSION_HOURS: i64 = 8;
const MAX_ACTIVE_SESSIONS: i64 = 5;
const TOKEN_BYTES: usize = 32;

#[derive(Clone, Debug)]
pub struct AuthenticatedSession {
    pub session_id: String,
    pub user_id: String,
    pub actor: ActorContext,
    pub requester_id: Option<String>,
    pub csrf_hash: String,
    pub expires_at: DateTime<Utc>,
}

pub struct CreatedSession {
    pub context: AuthenticatedSession,
    pub session_token: String,
    pub csrf_token: String,
}

pub async fn create(
    pool: &SqlitePool,
    user_id: &str,
    persona_id: &str,
    requester_id: Option<String>,
) -> AppResult<CreatedSession> {
    let now = Utc::now();
    let expires_at = now + Duration::hours(SESSION_HOURS);
    let session_token = random_token()?;
    let csrf_token = random_token()?;
    let session_id = Uuid::new_v4().to_string();
    let mut transaction = pool.begin().await?;

    sqlx::query(
        "UPDATE auth_sessions SET revoked_at=? WHERE user_id=? AND revoked_at IS NULL AND expires_at<=?",
    )
    .bind(now.to_rfc3339())
    .bind(user_id)
    .bind(now.to_rfc3339())
    .execute(&mut *transaction)
    .await?;

    let stale_ids = sqlx::query_scalar::<_, String>(
        "SELECT id FROM auth_sessions WHERE user_id=? AND revoked_at IS NULL ORDER BY created_at DESC LIMIT -1 OFFSET ?",
    )
    .bind(user_id)
    .bind(MAX_ACTIVE_SESSIONS - 1)
    .fetch_all(&mut *transaction)
    .await?;
    for stale_id in stale_ids {
        sqlx::query("UPDATE auth_sessions SET revoked_at=? WHERE id=?")
            .bind(now.to_rfc3339())
            .bind(stale_id)
            .execute(&mut *transaction)
            .await?;
    }

    let session_token_hash = token_hash(&session_token);
    let csrf_hash = token_hash(&csrf_token);
    sqlx::query("INSERT INTO auth_sessions(id,user_id,token_hash,csrf_hash,created_at,expires_at,last_seen_at) VALUES (?,?,?,?,?,?,?)")
        .bind(&session_id)
        .bind(user_id)
        .bind(session_token_hash)
        .bind(&csrf_hash)
        .bind(now.to_rfc3339())
        .bind(expires_at.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

    let actor = identity::load_actor(pool, persona_id).await?;
    tracing::info!(
        event = "auth.session_created",
        component = "auth",
        operation = "session.create",
        outcome = "success",
        actor_id = %actor.subject_id,
        session_id = %session_id
    );
    Ok(CreatedSession {
        context: AuthenticatedSession {
            session_id,
            user_id: user_id.to_owned(),
            actor,
            requester_id,
            csrf_hash,
            expires_at,
        },
        session_token,
        csrf_token,
    })
}

pub async fn required(pool: &SqlitePool, headers: &HeaderMap) -> AppResult<AuthenticatedSession> {
    let raw_token =
        cookie_value(headers, SESSION_COOKIE).ok_or(AppError::AuthenticationRequired)?;
    if !valid_token_shape(&raw_token) {
        return Err(AppError::AuthenticationRequired);
    }
    let row = sqlx::query_as::<_, SessionRow>(
        "SELECT s.id session_id,s.user_id,s.csrf_hash,s.expires_at,u.persona_id,u.requester_id,u.active user_active FROM auth_sessions s JOIN auth_users u ON u.id=s.user_id WHERE s.token_hash=? AND s.revoked_at IS NULL",
    )
    .bind(token_hash(&raw_token))
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::AuthenticationRequired)?;
    let expires_at = DateTime::parse_from_rfc3339(&row.expires_at)
        .map_err(|_| AppError::Internal)?
        .with_timezone(&Utc);
    if expires_at <= Utc::now() {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE auth_sessions SET revoked_at=? WHERE id=? AND revoked_at IS NULL")
            .bind(now)
            .bind(&row.session_id)
            .execute(pool)
            .await?;
        tracing::warn!(
            event = "auth.session_expired",
            component = "auth",
            operation = "session.resolve",
            outcome = "rejected",
            reason = "expired",
            session_id = %row.session_id
        );
        return Err(AppError::SessionExpired);
    }
    if row.user_active != 1 {
        return Err(AppError::AuthenticationRequired);
    }
    let actor = identity::load_actor(pool, &row.persona_id)
        .await
        .map_err(|_| AppError::AuthenticationRequired)?;
    sqlx::query("UPDATE auth_sessions SET last_seen_at=? WHERE id=?")
        .bind(Utc::now().to_rfc3339())
        .bind(&row.session_id)
        .execute(pool)
        .await?;
    Ok(AuthenticatedSession {
        session_id: row.session_id,
        user_id: row.user_id,
        actor,
        requester_id: row.requester_id,
        csrf_hash: row.csrf_hash,
        expires_at,
    })
}

pub async fn optional(
    pool: &SqlitePool,
    headers: &HeaderMap,
) -> AppResult<Option<AuthenticatedSession>> {
    if cookie_value(headers, SESSION_COOKIE).is_none() {
        return Ok(None);
    }
    required(pool, headers).await.map(Some)
}

pub async fn rotate_csrf(
    pool: &SqlitePool,
    session: &mut AuthenticatedSession,
) -> AppResult<String> {
    let raw_token = random_token()?;
    let csrf_hash = token_hash(&raw_token);
    sqlx::query("UPDATE auth_sessions SET csrf_hash=? WHERE id=? AND revoked_at IS NULL")
        .bind(&csrf_hash)
        .bind(&session.session_id)
        .execute(pool)
        .await?;
    session.csrf_hash = csrf_hash;
    Ok(raw_token)
}

pub fn verify_csrf(
    session: &AuthenticatedSession,
    headers: &HeaderMap,
    body_token: Option<&str>,
) -> AppResult<()> {
    let submitted = headers
        .get("x-csrf-token")
        .and_then(|value| value.to_str().ok())
        .or(body_token)
        .unwrap_or_default();
    let supplied_hash = token_hash(submitted);
    let matches = session.csrf_hash.len() == supplied_hash.len()
        && bool::from(session.csrf_hash.as_bytes().ct_eq(supplied_hash.as_bytes()));
    if matches {
        return Ok(());
    }
    tracing::warn!(
        event = "auth.csrf_rejected",
        component = "auth",
        operation = "csrf.verify",
        outcome = "rejected",
        actor_id = %session.actor.subject_id,
        reason = if submitted.is_empty() { "missing" } else { "mismatch" }
    );
    Err(AppError::CsrfInvalid)
}

pub async fn revoke(pool: &SqlitePool, session: &AuthenticatedSession) -> AppResult<()> {
    sqlx::query("UPDATE auth_sessions SET revoked_at=? WHERE id=? AND revoked_at IS NULL")
        .bind(Utc::now().to_rfc3339())
        .bind(&session.session_id)
        .execute(pool)
        .await?;
    tracing::info!(
        event = "auth.session_revoked",
        component = "auth",
        operation = "session.revoke",
        outcome = "success",
        actor_id = %session.actor.subject_id,
        session_id = %session.session_id
    );
    Ok(())
}

pub fn require_requester(session: AuthenticatedSession) -> AppResult<AuthenticatedSession> {
    if session.actor.actor_type == ActorType::Requester && session.requester_id.is_some() {
        return Ok(session);
    }
    access_denied(&session, "requester_required");
    Err(AppError::Forbidden)
}

pub fn require_government(session: AuthenticatedSession) -> AppResult<AuthenticatedSession> {
    if matches!(
        session.actor.actor_type,
        ActorType::Staff | ActorType::PlatformAdmin
    ) {
        return Ok(session);
    }
    access_denied(&session, "government_required");
    Err(AppError::Forbidden)
}

pub fn set_cookie_header(token: &str, public_origin: &str) -> AppResult<HeaderValue> {
    cookie_header(token, public_origin, false)
}

pub fn clear_cookie_header(public_origin: &str) -> AppResult<HeaderValue> {
    cookie_header("", public_origin, true)
}

fn cookie_header(value: &str, public_origin: &str, removal: bool) -> AppResult<HeaderValue> {
    let local_http = is_explicit_local_http(public_origin);
    let mut builder = Cookie::build((SESSION_COOKIE, value.to_owned()))
        .path("/api/v1")
        .http_only(true)
        .secure(!local_http)
        .same_site(if local_http {
            SameSite::Lax
        } else {
            SameSite::None
        })
        .max_age(cookie::time::Duration::seconds(if removal {
            0
        } else {
            SESSION_HOURS * 60 * 60
        }));
    if removal {
        builder = builder.expires(OffsetDateTime::UNIX_EPOCH);
    }
    HeaderValue::from_str(&builder.build().to_string()).map_err(|_| AppError::Internal)
}

fn random_token() -> AppResult<String> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|_| AppError::Internal)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn valid_token_shape(value: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(value)
        .is_ok_and(|decoded| decoded.len() == TOKEN_BYTES)
}

fn token_hash(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|value| Cookie::parse(value.trim().to_owned()).ok())
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_owned())
}

fn is_explicit_local_http(origin: &str) -> bool {
    reqwest::Url::parse(origin).is_ok_and(|url| {
        url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))
    })
}

fn access_denied(session: &AuthenticatedSession, reason: &'static str) {
    tracing::warn!(
        event = "auth.access_denied",
        component = "auth",
        operation = "authorization.check",
        outcome = "rejected",
        actor_id = %session.actor.subject_id,
        reason
    );
}
