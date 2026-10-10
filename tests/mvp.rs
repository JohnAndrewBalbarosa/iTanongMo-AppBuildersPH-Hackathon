use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{NaiveDate, Utc};
use http_body_util::BodyExt;
use itanongmo::{
    ai::AiProvider,
    app::{AppConfig, AppState},
    error::AppError,
    identity::{ActorContext, ActorType, Role},
    organization,
    requests::{self, Action, NewRequest},
};
use sqlx::SqlitePool;
use std::{path::PathBuf, sync::Arc};
use tower::ServiceExt;

async fn database() -> SqlitePool {
    let path = format!("target/itanongmo-test-{}.db", uuid::Uuid::new_v4());
    let pool = itanongmo::app::startup::connect(&format!("sqlite://{path}?mode=rwc"))
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    organization::seed_reference_data(&pool).await.unwrap();
    itanongmo::auth::seed_demo_auth(&pool).await.unwrap();
    pool
}

fn request_input(email: &str, description: &str) -> NewRequest {
    NewRequest {
        agency_ids: "agency-dost".into(),
        requester_name: "Demo Citizen".into(),
        requester_email: email.into(),
        title: "DOST public records request".into(),
        description: description.into(),
        purpose: "Public-interest research".into(),
        urgency_details: String::new(),
        consent: "yes".into(),
    }
}

fn government_actor(role: Role) -> ActorContext {
    ActorContext {
        subject_id: format!("test-{role:?}"),
        display_name: "Test Official".into(),
        actor_type: ActorType::Staff,
        agency_id: Some("agency-dost".into()),
        organization_unit_id: Some("dost-receiving".into()),
        roles: vec![role],
        assurance_level: "demo".into(),
        provider: "test".into(),
    }
}

fn api(pool: SqlitePool) -> axum::Router {
    api_with_persona_login(pool, false)
}

fn api_with_persona_login(pool: SqlitePool, enable_persona_login: bool) -> axum::Router {
    api_with_guest_ai(pool, enable_persona_login, false)
}

struct DraftProvider;

#[async_trait::async_trait]
impl AiProvider for DraftProvider {
    fn model_id(&self) -> &str {
        "test-draft"
    }

    async fn generate_json(
        &self,
        _: &str,
        _: serde_json::Value,
    ) -> itanongmo::error::AppResult<serde_json::Value> {
        Ok(serde_json::json!({
            "email_body": "Please provide the public program reports.",
            "description": "Public program reports and supporting records.",
            "purpose": "Public research",
            "urgency_details": "",
            "agency_queries": ["Department of Science and Technology"]
        }))
    }
}

fn api_with_guest_ai(
    pool: SqlitePool,
    enable_persona_login: bool,
    guest_ai_enabled: bool,
) -> axum::Router {
    let config = AppConfig {
        bind: "127.0.0.1:0".into(),
        database_url: "sqlite::memory:".into(),
        data_dir: PathBuf::from("target"),
        public_origin: "http://127.0.0.1:4320".into(),
        frontend_origin: Some("https://example.github.io".into()),
        ai_chat_completions_url: None,
        ai_api_key: None,
        ai_model: "test".into(),
        requester_ai_enabled: guest_ai_enabled,
        guest_ai_enabled,
        demo_reset_on_start: false,
        enable_persona_login,
    };
    itanongmo::app::router(AppState {
        pool,
        config: Arc::new(config),
        auth: itanongmo::auth::AuthService::new().unwrap(),
        ai: guest_ai_enabled.then(|| Arc::new(DraftProvider) as Arc<dyn AiProvider>),
        requester_ai: itanongmo::requester_ai::RequesterAiService::new().unwrap(),
    })
}

#[tokio::test]
async fn guest_ai_requires_flag_and_frontend_origin_and_enforces_limit() {
    let body = serde_json::json!({
        "manual_title": "Public program records",
        "what_to_ask": "Please provide the latest public program reports and records."
    })
    .to_string();
    let request = |origin: Option<&str>| {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/api/v1/requester-ai/guest-draft")
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(origin) = origin {
            builder = builder.header(header::ORIGIN, origin);
        }
        builder.body(Body::from(body.clone())).unwrap()
    };
    let disabled = api(database().await)
        .oneshot(request(Some("https://example.github.io")))
        .await
        .unwrap();
    assert_eq!(disabled.status(), StatusCode::SERVICE_UNAVAILABLE);

    let router = api_with_guest_ai(database().await, false, true);
    assert_eq!(
        router
            .clone()
            .oneshot(request(None))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        router
            .clone()
            .oneshot(request(Some("https://other.example")))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for _ in 0..5 {
        let response = router
            .clone()
            .oneshot(request(Some("https://example.github.io")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(
        router
            .oneshot(request(Some("https://example.github.io")))
            .await
            .unwrap()
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn urgent_language_is_flagged_and_queued_without_hiding_the_request() {
    let pool = database().await;
    let batch = requests::create_request(
        &pool,
        request_input(
            "urgent@example.test",
            "Please provide all records immediately because there is a safety hazard.",
        ),
    )
    .await
    .unwrap();
    let request = requests::load_request(&pool, &batch.requests[0].public_reference)
        .await
        .unwrap();
    let (hits, ai_status): (String, String) =
        sqlx::query_as("SELECT rule_hits_json,ai_status FROM triage_results WHERE request_id=?")
            .bind(&request.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let jobs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_jobs WHERE request_id=? AND task_kind='triage'",
    )
    .bind(&request.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(hits.contains("time_sensitive") && hits.contains("safety_review"));
    assert_eq!(ai_status, "pending_ai");
    assert_eq!(jobs, 1);
    assert_eq!(
        requests::load_request(&pool, &request.public_reference)
            .await
            .unwrap()
            .status,
        "submitted"
    );
}

#[tokio::test]
async fn receiving_officer_cannot_issue_a_disposition() {
    let pool = database().await;
    let batch = requests::create_request(
        &pool,
        request_input(
            "roles@example.test",
            "Please provide the annual public research grant registry for 2025.",
        ),
    )
    .await
    .unwrap();
    let mut request = requests::load_request(&pool, &batch.requests[0].public_reference)
        .await
        .unwrap();
    sqlx::query("UPDATE foi_requests SET status='in_review' WHERE id=?")
        .bind(&request.id)
        .execute(&pool)
        .await
        .unwrap();
    request.status = "in_review".into();
    let denied = requests::apply_action(
        &pool,
        &government_actor(Role::ReceivingOfficer),
        &request,
        Action::Deny,
        "Records are exempt under the reviewed basis.",
        None,
    )
    .await;
    assert!(matches!(denied, Err(AppError::Forbidden)));
    requests::apply_action(
        &pool,
        &government_actor(Role::DecisionMaker),
        &request,
        Action::Deny,
        "Records are exempt under the reviewed basis.",
        None,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn working_day_deadline_skips_weekends_and_holidays() {
    let pool = database().await;
    sqlx::query("INSERT INTO holidays(holiday_date,label,source_url) VALUES ('2026-10-12','Test holiday','https://example.test/holiday')")
        .execute(&pool)
        .await
        .unwrap();
    let due = requests::add_working_days(&pool, NaiveDate::from_ymd_opt(2026, 10, 9).unwrap(), 1)
        .await
        .unwrap();
    assert_eq!(due, NaiveDate::from_ymd_opt(2026, 10, 13).unwrap());
}

#[tokio::test]
async fn one_submission_creates_independent_requests_for_each_agency() {
    let pool = database().await;
    let mut input = request_input(
        "multi-agency@example.test",
        "Please provide the synthetic project registry used for multi-agency verification.",
    );
    input.agency_ids = "agency-dost,agency-da,agency-dost".into();
    let batch = requests::create_request(&pool, input).await.unwrap();
    assert!(batch.batch_reference.starts_with("FOI-BATCH-"));
    assert_eq!(batch.requests.len(), 2);
    assert_ne!(
        batch.requests[0].public_reference,
        batch.requests[1].public_reference
    );
}

#[tokio::test]
async fn github_pages_origin_can_create_a_multi_agency_submission() {
    let app = api(database().await);
    let payload = serde_json::json!({
        "agency_ids": "agency-dost,agency-da",
        "requester_name": "GitHub Pages Citizen",
        "requester_email": "pages-citizen@example.test",
        "title": "Public project records",
        "description": "Please provide the public project records for the integration test.",
        "purpose": "Public-interest research",
        "urgency_details": "",
        "consent": "yes"
    });
    let response = app
        .oneshot(
            Request::post("/api/v1/submissions")
                .header(header::ORIGIN, "https://example.github.io")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&"https://example.github.io".parse().unwrap())
    );
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(body["data"]["requests"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn cors_preflight_never_echoes_an_unconfigured_origin() {
    let response = api(database().await)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/submissions")
                .header(header::ORIGIN, "https://attacker.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        None
    );
    assert_ne!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&"https://attacker.example".parse().unwrap())
    );
}

#[tokio::test]
async fn configured_origin_preflight_allows_submission_content_type_header() {
    let response = api(database().await)
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/v1/submissions")
                .header(header::ORIGIN, "https://example.github.io")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&"https://example.github.io".parse().unwrap())
    );
    let allowed_headers = response
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_HEADERS)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(allowed_headers.contains("content-type"));
}

#[tokio::test]
async fn backend_exposes_json_only_and_does_not_register_legacy_html_routes() {
    let app = api(database().await);
    let root = app
        .clone()
        .oneshot(Request::get("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(root.status(), StatusCode::OK);
    assert_eq!(
        root.headers().get(header::CONTENT_TYPE),
        Some(&"application/json".parse().unwrap())
    );

    for path in ["/about", "/requests", "/government/personas"] {
        let response = app
            .clone()
            .oneshot(Request::get(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_ne!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&"text/html; charset=utf-8".parse().unwrap())
        );
    }
}

#[tokio::test]
async fn legacy_persona_login_is_disabled_by_default() {
    let pool = database().await;
    let persona_id: String = sqlx::query_scalar(
        "SELECT id FROM demo_personas WHERE actor_type='staff' AND active=1 LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let response = api(pool)
        .oneshot(
            Request::post("/api/v1/government/session")
                .header(header::ORIGIN, "https://example.github.io")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::json!({"persona_id": persona_id}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(
        response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .count(),
        0
    );
}

#[tokio::test]
async fn requester_ai_exposes_one_json_draft_endpoint() {
    let app = api(database().await);
    let payload = serde_json::json!({
        "what_to_ask": "Please provide the latest public program reports.",
        "manual_title": "Public program reports"
    });
    let response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/requester-ai/draft")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    for retired_path in [
        "/api/v1/requester-ai/draft-email",
        "/api/v1/requester-ai/form-values",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::post(retired_path)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{retired_path}");
    }
}

#[tokio::test]
async fn guest_submission_uses_reference_only_without_reply_or_appeal_routes() {
    let pool = database().await;
    let created = requests::create_request(
        &pool,
        request_input(
            "reference-only@example.test",
            "Records for verifying reference-only guest request tracking.",
        ),
    )
    .await
    .unwrap();
    let stored_hash: Option<String> = sqlx::query_scalar(
        "SELECT requester_access_hash FROM request_batches WHERE batch_reference=?",
    )
    .bind(&created.batch_reference)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(stored_hash.is_none());

    let reference = &created.requests[0].public_reference;
    let app = api(pool);
    for path in [
        format!("/api/v1/requests/{reference}/messages"),
        format!("/api/v1/requests/{reference}/appeals"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::post(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn ai_draft_publish_requires_matching_hash_review_and_single_use() {
    let pool = database().await;
    let batch = requests::create_request(
        &pool,
        request_input(
            "ai-review@example.test",
            "Records used to verify accountable AI draft publication controls.",
        ),
    )
    .await
    .unwrap();
    let request = requests::load_request(&pool, &batch.requests[0].public_reference)
        .await
        .unwrap();
    let job_id = uuid::Uuid::new_v4().to_string();
    let output = serde_json::json!({
        "schema_version": "1",
        "request_id": request.id,
        "subject": "Response",
        "body": "This reviewed response cites the verified agency records.",
        "source_record_ids": ["record-1"],
        "limitations": [],
        "requires_human_review": true
    });
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO ai_jobs(id,request_id,task_kind,status,input_json,input_hash,output_json,output_hash,model_id,prompt_version,available_at,completed_at,created_at) VALUES (?,?,'government_response','completed','{}','input-hash',?,'trusted-output-hash','test-model','v1',?,?,?)")
        .bind(&job_id)
        .bind(&request.id)
        .bind(output.to_string())
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    let password_hash: String =
        sqlx::query_scalar("SELECT password_hash FROM auth_users WHERE id='auth-officer-judge'")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO auth_users(id,email,password_hash,persona_id,requester_id,active,created_at,updated_at) VALUES ('auth-decision-test','decision@example.test',?,'dost-decision',NULL,1,?,?)")
        .bind(password_hash)
        .bind(&now)
        .bind(&now)
        .execute(&pool)
        .await
        .unwrap();
    let app = api_with_persona_login(pool.clone(), true);
    let session_response = app
        .clone()
        .oneshot(
            Request::post("/api/v1/government/session")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"persona_id":"dost-decision"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    let cookie = session_response
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let session_body = session_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let session: serde_json::Value = serde_json::from_slice(&session_body).unwrap();
    let csrf = session["data"]["csrf_token"].as_str().unwrap();
    let publish_path = format!(
        "/api/v1/government/requests/{}/ai-drafts/{job_id}/publish",
        batch.requests[0].public_reference
    );

    let mismatch = app
        .clone()
        .oneshot(
            Request::post(&publish_path)
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"csrf_token":"{csrf}","source_output_hash":"wrong","edited_body":"Reviewed response body for release.","factual_review_confirmed":"yes"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mismatch.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let publish_body = format!(
        r#"{{"csrf_token":"{csrf}","source_output_hash":"trusted-output-hash","edited_body":"Reviewed response body for release.","factual_review_confirmed":"yes"}}"#
    );
    let published = app
        .clone()
        .oneshot(
            Request::post(&publish_path)
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(publish_body.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(published.status(), StatusCode::OK);

    let repeated = app
        .oneshot(
            Request::post(&publish_path)
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(publish_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(repeated.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let review_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_draft_reviews WHERE ai_job_id=?")
            .bind(&job_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(review_count, 1);
}
