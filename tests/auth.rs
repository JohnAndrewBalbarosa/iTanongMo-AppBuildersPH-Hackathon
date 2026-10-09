use std::{path::PathBuf, sync::Arc};

use axum::{
    Router,
    body::Body,
    http::{HeaderMap, Request, StatusCode, header},
    response::Response,
};
use http_body_util::BodyExt;
use itanongmo::{
    app::{AppConfig, AppState},
    auth::{AuthService, seed_demo_auth},
    organization,
};
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tower::ServiceExt;

const CITIZEN_EMAIL: &str = "citizen.judge@itanongmo.local";
const CITIZEN_PASSWORD: &str = "CitizenJudge2026!";
const OFFICER_EMAIL: &str = "officer.judge@itanongmo.local";
const OFFICER_PASSWORD: &str = "OfficerJudge2026!";

struct Snapshot {
    status: StatusCode,
    headers: HeaderMap,
    body: Value,
}

async fn database() -> SqlitePool {
    let path = format!("target/itanongmo-auth-test-{}.db", uuid::Uuid::new_v4());
    let pool = itanongmo::app::startup::connect(&format!("sqlite://{path}?mode=rwc"))
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    organization::seed_reference_data(&pool).await.unwrap();
    seed_demo_auth(&pool).await.unwrap();
    pool
}

fn api(pool: SqlitePool, public_origin: &str) -> Router {
    let config = AppConfig {
        bind: "127.0.0.1:0".into(),
        database_url: "sqlite::memory:".into(),
        data_dir: PathBuf::from("target"),
        public_origin: public_origin.into(),
        frontend_origin: Some("https://frontend.example".into()),
        ai_base_url: None,
        ai_proxy_token: None,
        ai_model: "test".into(),
        requester_ai_enabled: false,
        requester_ai_token_ttl_seconds: 900,
        demo_reset_on_start: false,
        enable_persona_login: false,
    };
    itanongmo::app::router(AppState {
        pool,
        config: Arc::new(config),
        auth: AuthService::new().unwrap(),
        ai: None,
        requester_ai: itanongmo::requester_ai::RequesterAiService::new().unwrap(),
    })
}

async fn snapshot(response: Response) -> Snapshot {
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    Snapshot {
        status,
        headers,
        body,
    }
}

async fn login(app: &Router, email: &str, password: &str) -> Snapshot {
    snapshot(
        app.clone()
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({"email":email,"password":password}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await
}

fn session_cookie(login: &Snapshot) -> String {
    login
        .headers
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned()
}

fn csrf(login: &Snapshot) -> &str {
    login.body["data"]["csrf_token"].as_str().unwrap()
}

#[tokio::test]
async fn auth_seed_is_idempotent() {
    let pool = database().await;
    seed_demo_auth(&pool).await.unwrap();
    seed_demo_auth(&pool).await.unwrap();
    let users: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM auth_users WHERE id IN ('auth-citizen-judge','auth-officer-judge')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let requests: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM foi_requests WHERE id LIKE 'auth-demo-request-%'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(users, 2);
    assert_eq!(requests, 3);
}

#[tokio::test]
async fn citizen_and_government_logins_resolve_server_side_actors() {
    let pool = database().await;
    let app = api(pool, "https://api.example");
    let citizen = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    assert_eq!(citizen.status, StatusCode::OK);
    assert_eq!(citizen.body["data"]["actor"]["id"], "requester-demo");
    assert_eq!(citizen.body["data"]["actor"]["roles"][0], "requester");
    let officer = login(&app, OFFICER_EMAIL, OFFICER_PASSWORD).await;
    assert_eq!(officer.status, StatusCode::OK);
    assert_eq!(officer.body["data"]["actor"]["id"], "dost-fro");
    assert_eq!(officer.body["data"]["actor"]["agency_id"], "agency-dost");
    assert_eq!(
        officer.body["data"]["actor"]["organization_unit_id"],
        "dost-receiving"
    );
}

#[tokio::test]
async fn blank_credentials_are_validation_errors() {
    let app = api(database().await, "https://api.example");
    let result = login(&app, "  ", "").await;
    assert_eq!(result.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(result.body["error"]["code"], "validation_failed");
}

#[tokio::test]
async fn unknown_email_and_wrong_password_have_identical_responses() {
    let app = api(database().await, "https://api.example");
    let unknown = login(&app, "missing@itanongmo.local", "WrongPassword2026!").await;
    let wrong = login(&app, CITIZEN_EMAIL, "WrongPassword2026!").await;
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown.body, wrong.body);
    assert_eq!(unknown.body["error"]["code"], "invalid_credentials");
}

#[tokio::test]
async fn passwords_are_argon2id_and_plaintext_is_absent() {
    let pool = database().await;
    let hashes = sqlx::query_scalar::<_, String>("SELECT password_hash FROM auth_users")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(hashes.len(), 2);
    assert!(hashes.iter().all(|hash| hash.starts_with("$argon2id$")));
    let plaintext_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM auth_users WHERE password_hash IN (?,?)")
            .bind(CITIZEN_PASSWORD)
            .bind(OFFICER_PASSWORD)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(plaintext_count, 0);
}

#[tokio::test]
async fn cookies_use_https_and_localhost_security_profiles() {
    let https = login(
        &api(database().await, "https://api.example"),
        CITIZEN_EMAIL,
        CITIZEN_PASSWORD,
    )
    .await;
    let https_cookie = https.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(https_cookie.contains("HttpOnly"));
    assert!(https_cookie.contains("Secure"));
    assert!(https_cookie.contains("SameSite=None"));
    assert!(https_cookie.contains("Path=/api/v1"));
    assert!(https_cookie.contains("Max-Age=28800"));
    assert!(!https_cookie.contains("Domain="));

    let local = login(
        &api(database().await, "http://localhost:4320"),
        CITIZEN_EMAIL,
        CITIZEN_PASSWORD,
    )
    .await;
    let local_cookie = local.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(local_cookie.contains("SameSite=Lax"));
    assert!(!local_cookie.contains("Secure"));
}

#[tokio::test]
async fn valid_session_lookup_rotates_and_returns_csrf() {
    let app = api(database().await, "https://api.example");
    let signed_in = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    let result = snapshot(
        app.oneshot(
            Request::get("/api/v1/auth/session")
                .header(header::COOKIE, session_cookie(&signed_in))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(result.status, StatusCode::OK);
    assert_eq!(result.body["data"]["actor"]["id"], "requester-demo");
    assert_ne!(
        result.body["data"]["csrf_token"],
        signed_in.body["data"]["csrf_token"]
    );
}

#[tokio::test]
async fn invalid_expired_inactive_and_revoked_sessions_fail() {
    let pool = database().await;
    let app = api(pool.clone(), "https://api.example");
    for cookie in [None, Some("itanongmo_session=malformed".to_owned())] {
        let mut request = Request::get("/api/v1/auth/session");
        if let Some(cookie) = cookie {
            request = request.header(header::COOKIE, cookie);
        }
        let result = snapshot(
            app.clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(result.status, StatusCode::UNAUTHORIZED);
    }

    let expired = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    sqlx::query(
        "UPDATE auth_sessions SET expires_at='2000-01-01T00:00:00Z' WHERE revoked_at IS NULL",
    )
    .execute(&pool)
    .await
    .unwrap();
    let result = authenticated_get(&app, "/api/v1/auth/session", &session_cookie(&expired)).await;
    assert_eq!(result.status, StatusCode::UNAUTHORIZED);
    assert_eq!(result.body["error"]["code"], "session_expired");

    let inactive = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    sqlx::query("UPDATE auth_users SET active=0 WHERE email=?")
        .bind(CITIZEN_EMAIL)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        authenticated_get(&app, "/api/v1/auth/session", &session_cookie(&inactive))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    sqlx::query("UPDATE auth_users SET active=1 WHERE email=?")
        .bind(CITIZEN_EMAIL)
        .execute(&pool)
        .await
        .unwrap();

    let revoked = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    sqlx::query("UPDATE auth_sessions SET revoked_at=? WHERE revoked_at IS NULL")
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        authenticated_get(&app, "/api/v1/auth/session", &session_cookie(&revoked))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn logout_requires_csrf_revokes_session_and_clears_cookie() {
    let pool = database().await;
    let app = api(pool, "https://api.example");
    let signed_in = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    let cookie = session_cookie(&signed_in);
    let response = snapshot(
        app.clone()
            .oneshot(
                Request::post("/api/v1/auth/logout")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({"csrf_token":csrf(&signed_in)}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body["data"]["signed_out"], true);
    let cleared = response.headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cleared.contains("Max-Age=0"));
    assert!(cleared.contains("Path=/api/v1"));
    assert_eq!(
        authenticated_get(&app, "/api/v1/auth/session", &cookie)
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn government_mutations_require_matching_csrf() {
    let app = api(database().await, "https://api.example");
    let signed_in = login(&app, OFFICER_EMAIL, OFFICER_PASSWORD).await;
    for token in [None, Some("wrong-token")] {
        let mut request = Request::post("/api/v1/government/requests/FOI-2026-002/triage")
            .header(header::COOKIE, session_cookie(&signed_in))
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = token {
            request = request.header("x-csrf-token", token);
        }
        let result = snapshot(
            app.clone()
                .oneshot(
                    request
                        .body(Body::from(json!({"categories":"appeal"}).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;
        assert_eq!(result.status, StatusCode::FORBIDDEN);
        assert_eq!(result.body["error"]["code"], "csrf_invalid");
    }
}

#[tokio::test]
async fn requester_is_denied_government_access_and_officer_is_agency_scoped() {
    let app = api(database().await, "https://api.example");
    let citizen = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    assert_eq!(
        authenticated_get(&app, "/api/v1/government/board", &session_cookie(&citizen))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let officer = login(&app, OFFICER_EMAIL, OFFICER_PASSWORD).await;
    let cookie = session_cookie(&officer);
    assert_eq!(
        authenticated_get(&app, "/api/v1/government/requests/FOI-2026-002", &cookie)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        authenticated_get(&app, "/api/v1/government/requests/FOI-2026-001", &cookie)
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn citizen_sees_only_owned_requests_without_private_fields() {
    let pool = database().await;
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO requesters(id,display_name,contact_email,consent_version,created_at) VALUES ('other-requester','Other Citizen','other@example.test','demo-v2',?)")
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO foi_requests(id,public_reference,agency_id,assigned_unit_id,requester_id,title,description,purpose,status,submitted_at,due_at) VALUES ('other-request','FOI-OTHER-001','agency-dost','dost-receiving','other-requester','Other account request','Must not be visible to the demo citizen.','Ownership isolation','submitted',?,'2026-12-31')")
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO messages(id,request_id,actor_id,actor_label,visibility,body,content_hash,created_at) VALUES ('private-test','auth-demo-request-review','dost-fro','Officer','internal','private note','hash',?)")
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    let app = api(pool, "https://api.example");
    let citizen = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    let cookie = session_cookie(&citizen);
    let list = authenticated_get(&app, "/api/v1/citizen/requests", &cookie).await;
    assert_eq!(list.status, StatusCode::OK);
    assert_eq!(list.body["data"]["requests"].as_array().unwrap().len(), 3);
    let detail = authenticated_get(&app, "/api/v1/citizen/requests/FOI-2026-002", &cookie).await;
    assert_eq!(detail.status, StatusCode::OK);
    let serialized = detail.body.to_string();
    for private_field in [
        "private note",
        "triage",
        "ai_draft",
        "audit",
        "private_detail",
    ] {
        assert!(!serialized.contains(private_field), "{private_field}");
    }
    let other = authenticated_get(&app, "/api/v1/citizen/requests/FOI-OTHER-001", &cookie).await;
    assert_eq!(other.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn authenticated_submission_uses_server_requester_mapping() {
    let pool = database().await;
    let app = api(pool.clone(), "https://api.example");
    let citizen = login(&app, CITIZEN_EMAIL, CITIZEN_PASSWORD).await;
    let response = snapshot(
        app.oneshot(
            Request::post("/api/v1/submissions")
                .header(header::COOKIE, session_cookie(&citizen))
                .header("x-csrf-token", csrf(&citizen))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "agency_ids":"agency-dost",
                        "requester_name":"Client Controlled Name",
                        "requester_email":"attacker@example.test",
                        "title":"Authenticated request",
                        "description":"Please provide the requested public records for review.",
                        "purpose":"Public-interest verification",
                        "urgency_details":"",
                        "consent":"yes"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap(),
    )
    .await;
    assert_eq!(response.status, StatusCode::CREATED);
    let reference = response.body["data"]["requests"][0]["reference"]
        .as_str()
        .unwrap();
    let requester_id: String =
        sqlx::query_scalar("SELECT requester_id FROM foi_requests WHERE public_reference=?")
            .bind(reference)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(requester_id, "requester-demo");
}

#[tokio::test]
async fn login_limiter_returns_rate_limited_without_account_disclosure() {
    let app = api(database().await, "https://api.example");
    for _ in 0..5 {
        assert_eq!(
            login(&app, CITIZEN_EMAIL, "WrongPassword2026!")
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
    let limited = login(&app, CITIZEN_EMAIL, "WrongPassword2026!").await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.body["error"]["code"], "rate_limited");
}

#[tokio::test]
async fn credentialed_cors_allows_only_the_configured_origin_and_csrf_header() {
    let app = api(database().await, "https://api.example");
    let allowed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/auth/login")
                .header(header::ORIGIN, "https://frontend.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "content-type,x-csrf-token",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        allowed.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN],
        "https://frontend.example"
    );
    assert_eq!(
        allowed.headers()[header::ACCESS_CONTROL_ALLOW_CREDENTIALS],
        "true"
    );
    assert!(
        allowed.headers()[header::ACCESS_CONTROL_ALLOW_HEADERS]
            .to_str()
            .unwrap()
            .contains("x-csrf-token")
    );
    assert!(
        allowed.headers()[header::VARY]
            .to_str()
            .unwrap()
            .contains("origin")
    );

    let rejected = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/auth/login")
                .header(header::ORIGIN, "https://evil.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        rejected.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&"*".parse().unwrap())
    );
    assert!(
        rejected
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
}

async fn authenticated_get(app: &Router, path: &str, cookie: &str) -> Snapshot {
    snapshot(
        app.clone()
            .oneshot(
                Request::get(path)
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap(),
    )
    .await
}
