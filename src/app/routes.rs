use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, Method, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};

use crate::{
    ai, auth, board,
    error::{AppError, AppResult},
    identity::{self, ActorContext, ActorType, Role, can_write_request},
    notifications, organization,
    requester_ai::RequesterAiInput,
    requests::{self, Action, NewRequest},
    triage,
};

use super::AppState;

pub fn router(state: AppState) -> Router {
    let mut api = Router::new()
        .route("/api/v1/config", get(api_config))
        .route("/api/v1/submissions", post(api_create_submission))
        .route("/api/v1/submissions/{reference}", get(api_submission_batch))
        .route("/api/v1/agencies", get(api_agencies))
        .route("/api/v1/agencies/search", get(api_agency_search))
        .route("/api/v1/requester-ai/draft", post(requester_ai_draft))
        .route("/api/v1/requester-ai/guest-draft", post(guest_ai_draft))
        .route("/api/v1/requests", get(api_requests))
        .route("/api/v1/requests/{reference}", get(api_request))
        .route("/api/v1/notifications", get(api_notifications))
        .route("/api/v1/citizen/requests", get(citizen_requests))
        .route("/api/v1/citizen/requests/{reference}", get(citizen_request))
        .route("/api/v1/government/session", get(government_session))
        .route("/api/v1/government/board", get(government_dashboard))
        .route(
            "/api/v1/government/requests/{reference}",
            get(government_request),
        )
        .route(
            "/api/v1/government/requests/{reference}/action",
            post(government_action),
        )
        .route(
            "/api/v1/government/requests/{reference}/messages",
            post(government_message),
        )
        .route(
            "/api/v1/government/requests/{reference}/triage",
            post(government_triage),
        )
        .route(
            "/api/v1/government/requests/{reference}/ai-draft",
            post(government_ai_draft),
        )
        .route(
            "/api/v1/government/requests/{reference}/ai-drafts/{job_id}/publish",
            post(government_publish_ai_draft),
        )
        .route(
            "/api/v1/government/requests/{reference}/documents",
            post(government_document),
        )
        .route(
            "/api/v1/government/reports/requests.csv",
            get(government_csv),
        )
        .route("/api/v1/documents/{id}", get(download_document))
        .merge(auth::routes());
    if state.config.enable_persona_login {
        api = api
            .route("/api/v1/government/personas", get(personas))
            .route("/api/v1/government/session", post(select_persona));
    }
    if let Some(origin) = &state.config.frontend_origin {
        let origin = HeaderValue::from_str(origin).expect("validated frontend origin");
        api = api.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(move |request_origin, _| {
                    request_origin == origin
                }))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    header::CONTENT_TYPE,
                    HeaderName::from_static("x-csrf-token"),
                ])
                .vary([header::ORIGIN])
                .allow_credentials(true),
        );
    }

    Router::new()
        .route("/", get(api_root))
        .route("/health", get(health))
        .merge(api)
        .layer(DefaultBodyLimit::disable())
        .layer(RequestBodyLimitLayer::new(11 * 1024 * 1024))
        .layer(middleware::from_fn(security_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn api_root() -> Json<serde_json::Value> {
    Json(json!({
        "service": "iTanongMo API",
        "api_version": "v1",
        "health": "/health",
        "api": "/api/v1"
    }))
}

async fn health(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(
        json!({"status":"ok","database":"ok","ai":if state.ai.is_some(){"configured"}else{"degraded"},"mode":"demo"}),
    ))
}

async fn personas(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let personas = identity::list_personas(&state.pool).await?;
    Ok(Json(json!({
        "data": personas.into_iter()
            .filter(|persona| persona.actor_type != "requester")
            .map(|persona| json!({
                "id": persona.id,
                "display_name": persona.display_name,
                "role": persona.role,
                "actor_type": persona.actor_type,
                "agency_id": persona.agency_id
            }))
            .collect::<Vec<_>>()
    })))
}

#[derive(Deserialize)]
struct GovernmentSessionInput {
    persona_id: String,
}

async fn select_persona(
    State(state): State<AppState>,
    Json(input): Json<GovernmentSessionInput>,
) -> AppResult<Response> {
    let actor = identity::load_actor(&state.pool, &input.persona_id).await?;
    if actor.actor_type == ActorType::Requester {
        return Err(AppError::Forbidden);
    }
    let account = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT id,requester_id FROM auth_users WHERE persona_id=? AND active=1",
    )
    .bind(&input.persona_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Forbidden)?;
    let created =
        auth::session::create(&state.pool, &account.0, &input.persona_id, account.1).await?;
    let mut response = Json(json!({
        "data": auth::session_json(&actor, &created.csrf_token, created.context.expires_at)
    }))
    .into_response();
    response.headers_mut().append(
        header::SET_COOKIE,
        auth::session::set_cookie_header(&created.session_token, &state.config.public_origin)?,
    );
    Ok(response)
}

async fn government_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let (actor, csrf) = government_session_context(&state, &headers).await?;
    Ok(Json(json!({
        "data": government_session_json(&actor, Some(&csrf))
    })))
}

fn government_session_json(actor: &ActorContext, csrf: Option<&str>) -> serde_json::Value {
    let mut session = json!({
        "actor": {
            "id": actor.subject_id,
            "display_name": actor.display_name,
            "actor_type": &actor.actor_type,
            "agency_id": actor.agency_id,
            "roles": actor.roles.iter().map(Role::as_str).collect::<Vec<_>>()
        }
    });
    if let Some(csrf) = csrf {
        session["csrf_token"] = json!(csrf);
    }
    session
}

async fn government_dashboard(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let actor = government_context(&state, &headers).await?.actor;
    let agency_id = actor.agency_id.as_deref().ok_or(AppError::Forbidden)?;
    let cards = board::load_board(&state.pool, agency_id).await?;
    let notifications = notifications::list_for_persona(&state.pool, &actor.subject_id).await?;
    Ok(Json(json!({
        "data": {
            "session": government_session_json(&actor, None),
            "cards": cards.into_iter().map(|card| json!({
                "reference": card.public_reference,
                "title": card.title,
                "status": card.status,
                "column": card.column(),
                "due_at": card.due_at,
                "agency_id": card.agency_id,
                "unit_name": card.unit_name,
                "ai_status": card.ai_status,
                "rule_hits": card.rule_hits_json
            })).collect::<Vec<_>>(),
            "notifications": notifications.into_iter().map(|item| json!({
                "id": item.id,
                "request_id": item.request_id,
                "kind": item.kind,
                "message": item.message,
                "read_at": item.read_at,
                "created_at": item.created_at
            })).collect::<Vec<_>>()
        }
    })))
}

async fn government_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let actor = government_context(&state, &headers).await?.actor;
    let request = requests::load_request(&state.pool, &reference).await?;
    ensure_government_read(&actor, &request.agency_id)?;
    let events = requests::list_events(&state.pool, &request.id).await?;
    let messages = requests::list_messages(&state.pool, &request.id, true).await?;
    let documents = requests::list_documents(&state.pool, &request.id).await?;
    let units = organization::organization_tree(&state.pool, &request.agency_id).await?;
    let triage = triage::load_triage(&state.pool, &request.id).await?;
    let ai_drafts = sqlx::query_as::<_, ai::AiDraftRow>("SELECT id,status,output_json,output_hash,model_id,created_at FROM ai_jobs WHERE request_id=? AND task_kind='government_response' ORDER BY created_at DESC")
        .bind(&request.id).fetch_all(&state.pool).await?;
    let actions = requests::allowed_actions(&request.status);
    Ok(Json(json!({
        "data": {
            "session": government_session_json(&actor, None),
            "request": {
                "reference": request.public_reference,
                "agency_id": request.agency_id,
                "agency_name": request.agency_name,
                "assigned_unit_id": request.assigned_unit_id,
                "title": request.title,
                "description": request.description,
                "purpose": request.purpose,
                "urgency_details": request.urgency_details,
                "status": request.status,
                "disposition": request.disposition,
                "disposition_reason": request.disposition_reason,
                "submitted_at": request.submitted_at,
                "due_at": request.due_at,
                "extended_due_at": request.extended_due_at,
                "revision": request.revision
            },
            "actions": actions,
            "events": events.into_iter().map(|event| json!({
                "type": event.event_type,
                "summary": event.public_summary,
                "from_status": event.from_status,
                "to_status": event.to_status,
                "created_at": event.created_at
            })).collect::<Vec<_>>(),
            "messages": messages.into_iter().map(|message| json!({
                "id": message.id,
                "actor": message.actor_label,
                "visibility": message.visibility,
                "body": message.body,
                "created_at": message.created_at
            })).collect::<Vec<_>>(),
            "documents": documents.into_iter().map(|document| json!({
                "id": document.id,
                "title": document.title,
                "original_name": document.original_name,
                "media_type": document.media_type,
                "byte_length": document.byte_length,
                "sha256": document.sha256,
                "released_at": document.released_at
            })).collect::<Vec<_>>(),
            "units": units.into_iter().map(|unit| json!({
                "id": unit.id,
                "parent_id": unit.parent_id,
                "slug": unit.slug,
                "kind": unit.kind,
                "name": unit.name
            })).collect::<Vec<_>>(),
            "triage": triage.map(|value| json!({
                "rules_version": value.rules_version,
                "rule_hits": value.rule_hits_json,
                "ai_status": value.ai_status,
                "ai_suggestion": value.ai_suggestion_json,
                "human_categories": value.human_categories_json
            })),
            "ai_drafts": ai_drafts.into_iter().map(|draft| json!({
                "id": draft.id,
                "status": draft.status,
                "output": draft.output_json,
                "output_hash": draft.output_hash,
                "model_id": draft.model_id,
                "created_at": draft.created_at
            })).collect::<Vec<_>>()
        }
    })))
}

#[derive(Deserialize)]
struct ActionForm {
    #[serde(default)]
    csrf_token: String,
    action: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    destination_unit_id: String,
}

async fn government_action(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(form): Json<ActionForm>,
) -> AppResult<Json<serde_json::Value>> {
    let authenticated = government_context(&state, &headers).await?;
    auth::session::verify_csrf(&authenticated, &headers, Some(&form.csrf_token))?;
    let actor = authenticated.actor;
    let request = requests::load_request(&state.pool, &reference).await?;
    let action = Action::parse(&form.action)
        .ok_or_else(|| AppError::Validation("Invalid workflow action".into()))?;
    requests::apply_action(
        &state.pool,
        &actor,
        &request,
        action,
        &form.note,
        non_empty(&form.destination_unit_id),
    )
    .await?;
    Ok(Json(
        json!({"data":{"reference":reference,"outcome":"updated"}}),
    ))
}

#[derive(Deserialize)]
struct GovernmentMessageForm {
    #[serde(default)]
    csrf_token: String,
    visibility: String,
    body: String,
}

async fn government_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(form): Json<GovernmentMessageForm>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let authenticated = government_context(&state, &headers).await?;
    auth::session::verify_csrf(&authenticated, &headers, Some(&form.csrf_token))?;
    let actor = authenticated.actor;
    let request = requests::load_request(&state.pool, &reference).await?;
    if !can_write_request(&actor, &request.agency_id) {
        return Err(access_denied(&actor, "government_write_scope"));
    }
    requests::add_message(
        &state.pool,
        &actor,
        &request.id,
        &form.visibility,
        &form.body,
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"data":{"reference":reference,"outcome":"message_created"}})),
    ))
}

#[derive(Deserialize)]
struct TriageForm {
    #[serde(default)]
    csrf_token: String,
    categories: String,
    #[serde(default)]
    destination_unit_id: String,
}

async fn government_triage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(form): Json<TriageForm>,
) -> AppResult<Json<serde_json::Value>> {
    let authenticated = government_context(&state, &headers).await?;
    auth::session::verify_csrf(&authenticated, &headers, Some(&form.csrf_token))?;
    let actor = authenticated.actor;
    let request = requests::load_request(&state.pool, &reference).await?;
    if !identity::can_triage(&actor, &request.agency_id) {
        return Err(access_denied(&actor, "government_triage_scope"));
    }
    let categories: Vec<String> = form
        .categories
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect();
    if categories.len() > 12 {
        return Err(AppError::Validation(
            "Masyadong maraming triage categories".into(),
        ));
    }
    sqlx::query("UPDATE triage_results SET human_categories_json=?,human_destination_unit_id=?,reviewed_by=?,reviewed_at=?,updated_at=? WHERE request_id=?")
        .bind(serde_json::to_string(&categories).map_err(|_| AppError::Internal)?).bind(non_empty(&form.destination_unit_id)).bind(&actor.subject_id).bind(Utc::now().to_rfc3339()).bind(Utc::now().to_rfc3339()).bind(&request.id).execute(&state.pool).await?;
    Ok(Json(
        json!({"data":{"reference":reference,"outcome":"triage_updated"}}),
    ))
}

#[derive(Deserialize)]
struct AiDraftForm {
    #[serde(default)]
    csrf_token: String,
    verified_facts: String,
}

async fn government_ai_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    Json(form): Json<AiDraftForm>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let authenticated = government_context(&state, &headers).await?;
    auth::session::verify_csrf(&authenticated, &headers, Some(&form.csrf_token))?;
    let actor = authenticated.actor;
    let request = requests::load_request(&state.pool, &reference).await?;
    if !can_write_request(&actor, &request.agency_id) {
        return Err(access_denied(&actor, "government_write_scope"));
    }
    ai::queue_government_draft(&state.pool, &actor, &request.id, &form.verified_facts).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({"data":{"reference":reference,"outcome":"draft_queued"}})),
    ))
}

#[derive(Deserialize)]
struct PublishAiDraftForm {
    #[serde(default)]
    csrf_token: String,
    source_output_hash: String,
    edited_body: String,
    #[serde(default)]
    factual_review_confirmed: String,
}

async fn government_publish_ai_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((reference, job_id)): Path<(String, String)>,
    Json(form): Json<PublishAiDraftForm>,
) -> AppResult<Json<serde_json::Value>> {
    let authenticated = government_context(&state, &headers).await?;
    auth::session::verify_csrf(&authenticated, &headers, Some(&form.csrf_token))?;
    let actor = authenticated.actor;
    let request = requests::load_request(&state.pool, &reference).await?;
    let may_publish = actor.has_role(Role::AgencyAdmin)
        || actor.has_role(Role::DecisionMaker)
        || actor.has_role(Role::PlatformAdmin);
    if !may_publish
        || (actor.agency_id.as_deref() != Some(request.agency_id.as_str())
            && !actor.has_role(Role::PlatformAdmin))
    {
        return Err(AppError::Forbidden);
    }
    if form.factual_review_confirmed != "yes" {
        return Err(AppError::Validation(
            "Kailangang kumpirmahin ang factual review bago i-publish".into(),
        ));
    }
    let already_published: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM ai_draft_reviews WHERE ai_job_id=? AND request_id=?)",
    )
    .bind(&job_id)
    .bind(&request.id)
    .fetch_one(&state.pool)
    .await?;
    if already_published {
        return Err(AppError::Validation(
            "Na-publish na ang AI draft na ito".into(),
        ));
    }
    let stored_hash: Option<String> = sqlx::query_scalar("SELECT output_hash FROM ai_jobs WHERE id=? AND request_id=? AND task_kind='government_response' AND status='completed'")
        .bind(&job_id).bind(&request.id).fetch_optional(&state.pool).await?.flatten();
    if stored_hash.as_deref() != Some(form.source_output_hash.as_str()) {
        return Err(AppError::Validation(
            "Nagbago o hindi valid ang AI draft; i-review muli".into(),
        ));
    }
    requests::add_message(
        &state.pool,
        &actor,
        &request.id,
        "public",
        &form.edited_body,
    )
    .await?;
    let published_hash = hex::encode(Sha256::digest(form.edited_body.trim().as_bytes()));
    sqlx::query("INSERT INTO ai_draft_reviews(id,ai_job_id,request_id,reviewer_id,source_output_hash,published_content_hash,factual_review_confirmed,published_at) VALUES (?,?,?,?,?,?,1,?)")
        .bind(uuid::Uuid::new_v4().to_string()).bind(&job_id).bind(&request.id).bind(&actor.subject_id).bind(&form.source_output_hash).bind(published_hash).bind(Utc::now().to_rfc3339()).execute(&state.pool).await?;
    sqlx::query("UPDATE ai_jobs SET status='published' WHERE id=?")
        .bind(&job_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(
        json!({"data":{"reference":reference,"outcome":"draft_published"}}),
    ))
}

async fn government_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let authenticated = government_context(&state, &headers).await?;
    let actor = authenticated.actor.clone();
    let request = requests::load_request(&state.pool, &reference).await?;
    if !can_write_request(&actor, &request.agency_id) {
        return Err(access_denied(&actor, "government_write_scope"));
    }
    let mut submitted_csrf = None;
    let mut title = None;
    let mut file = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::Validation("Invalid multipart form".into()))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        match name.as_str() {
            "csrf_token" => {
                submitted_csrf = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::Validation("Invalid CSRF field".into()))?,
                )
            }
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::Validation("Invalid title".into()))?,
                )
            }
            "file" => {
                let original = field.file_name().unwrap_or("document").to_owned();
                let media = field
                    .content_type()
                    .unwrap_or("application/octet-stream")
                    .to_owned();
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| AppError::Validation("Invalid document".into()))?;
                file = Some((original, media, bytes));
            }
            _ => {}
        }
    }
    auth::session::verify_csrf(&authenticated, &headers, submitted_csrf.as_deref())?;
    let (original, media, bytes) =
        file.ok_or_else(|| AppError::Validation("Pumili ng document".into()))?;
    requests::release_document(
        &state.pool,
        &state.config.data_dir,
        &actor,
        &request.id,
        requests::DocumentUpload {
            title: title.as_deref().unwrap_or("Released document"),
            original_name: &original,
            media_type: &media,
            bytes: &bytes,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({"data":{"reference":reference,"outcome":"document_released"}})),
    ))
}

async fn download_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Response> {
    let document = requests::load_document(&state.pool, &id).await?;
    let bytes = tokio::fs::read(
        state
            .config
            .data_dir
            .join("files")
            .join(&document.stored_name),
    )
    .await
    .map_err(|_| AppError::NotFound)?;
    if bytes.len() as i64 != document.byte_length {
        return Err(AppError::Internal);
    }
    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&document.media_type).map_err(|_| AppError::Internal)?,
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}\"",
            document.original_name.replace('"', "")
        ))
        .map_err(|_| AppError::Internal)?,
    );
    Ok(response)
}

async fn government_csv(State(state): State<AppState>, headers: HeaderMap) -> AppResult<Response> {
    let actor = government_context(&state, &headers).await?.actor;
    let agency_id = actor.agency_id.as_deref().ok_or(AppError::Forbidden)?;
    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as("SELECT public_reference,title,status,submitted_at,due_at FROM foi_requests WHERE agency_id=? ORDER BY submitted_at")
        .bind(agency_id).fetch_all(&state.pool).await?;
    let mut csv = String::from("reference,title,status,submitted_at,due_at\n");
    for row in rows {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            csv_cell(&row.0),
            csv_cell(&row.1),
            csv_cell(&row.2),
            csv_cell(&row.3),
            csv_cell(&row.4)
        ));
    }
    let mut response = Response::new(Body::from(csv));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=foi-requests.csv"),
    );
    Ok(response)
}

#[derive(Deserialize)]
struct AgencySearchQuery {
    #[serde(default)]
    q: String,
}

async fn api_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "data": {
            "api_version": "v1",
            "government_workspace_path": "government.html",
            "requester_ai_enabled": state.config.requester_ai_enabled && state.ai.is_some(),
            "guest_ai_enabled": state.config.guest_ai_enabled && state.config.requester_ai_enabled && state.ai.is_some(),
            "maximum_agencies_per_submission": 10
        }
    }))
}

async fn api_create_submission(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<NewRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let authenticated = auth::session::optional(&state.pool, &headers)
        .await?
        .map(auth::session::require_requester)
        .transpose()?;
    if let Some(session) = &authenticated {
        auth::session::verify_csrf(session, &headers, None)?;
    }
    let requester_id = authenticated.and_then(|session| session.requester_id);
    let batch = requests::create_request_for(&state.pool, input, requester_id.as_deref()).await?;
    tracing::info!(
        event = "api.submission_created",
        batch_reference = %batch.batch_reference,
        request_count = batch.requests.len(),
        outcome = "success"
    );
    Ok((StatusCode::CREATED, Json(submission_batch_json(batch))))
}

async fn api_submission_batch(
    State(state): State<AppState>,
    Path(reference): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let batch = requests::load_submission_batch(&state.pool, &reference).await?;
    Ok(Json(submission_batch_json(batch)))
}

fn submission_batch_json(batch: crate::requests::SubmissionBatch) -> serde_json::Value {
    json!({"data": {
        "batch_reference": batch.batch_reference,
        "created_at": batch.created_at,
        "requests": batch.requests.into_iter().map(|request| json!({
            "reference": request.public_reference,
            "agency": request.agency_short_name,
            "title": request.title,
            "status": request.status,
            "submitted_at": request.submitted_at,
            "due_at": request.due_at
        })).collect::<Vec<_>>()
    }})
}

async fn api_agency_search(
    State(state): State<AppState>,
    Query(query): Query<AgencySearchQuery>,
) -> AppResult<Json<serde_json::Value>> {
    if query.q.chars().count() > 160 {
        return Err(AppError::Validation("Agency search is too long".into()));
    }
    let rows = organization::search_agencies(&state.pool, &query.q).await?;
    Ok(Json(
        json!({"data":rows.into_iter().take(25).map(|agency|json!({
        "id":agency.id,"slug":agency.slug,"short_name":agency.short_name,"name":agency.name,
        "office_email":agency.office_email
    })).collect::<Vec<_>>() }),
    ))
}

async fn requester_ai_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<RequesterAiInput>,
) -> AppResult<Json<serde_json::Value>> {
    ensure_requester_ai_enabled(&state)?;
    let authenticated = auth::session::required(&state.pool, &headers).await?;
    let authenticated = auth::session::require_requester(authenticated)?;
    auth::session::verify_csrf(&authenticated, &headers, None)?;
    let provider = state.ai.as_deref().ok_or(AppError::AiUnavailable)?;
    let draft = state
        .requester_ai
        .draft(&authenticated.user_id, provider, input)
        .await?;
    Ok(Json(json!({"data": draft})))
}

async fn guest_ai_draft(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<RequesterAiInput>,
) -> AppResult<Json<serde_json::Value>> {
    if !state.config.guest_ai_enabled {
        return Err(AppError::AiUnavailable);
    }
    ensure_requester_ai_enabled(&state)?;
    let allowed_origin = state
        .config
        .frontend_origin
        .as_deref()
        .ok_or(AppError::Forbidden)?;
    let request_origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok());
    if request_origin != Some(allowed_origin) {
        return Err(AppError::Forbidden);
    }
    let provider = state.ai.as_deref().ok_or(AppError::AiUnavailable)?;
    let draft = state.requester_ai.draft_guest(provider, input).await?;
    Ok(Json(json!({"data": draft})))
}

fn ensure_requester_ai_enabled(state: &AppState) -> AppResult<()> {
    if state.config.requester_ai_enabled {
        Ok(())
    } else {
        Err(AppError::AiUnavailable)
    }
}

async fn api_agencies(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let rows = organization::list_agencies(&state.pool).await?;
    Ok(Json(
        json!({"data":rows.into_iter().map(|a|json!({"id":a.id,"slug":a.slug,"short_name":a.short_name,"name":a.name,"official_url":a.official_url,"office_email":a.office_email,"source_url":a.source_url,"source_as_of":a.source_as_of})).collect::<Vec<_>>() }),
    ))
}

async fn api_requests(State(state): State<AppState>) -> AppResult<Json<serde_json::Value>> {
    let rows = requests::list_public_requests(&state.pool).await?;
    Ok(Json(
        json!({"data":rows.into_iter().map(|r|json!({"reference":r.public_reference,"agency":r.agency_short_name,"title":r.title,"status":r.status,"submitted_at":r.submitted_at,"due_at":r.due_at})).collect::<Vec<_>>() }),
    ))
}

async fn citizen_requests(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let authenticated =
        auth::session::require_requester(auth::session::required(&state.pool, &headers).await?)?;
    let requester_id = authenticated
        .requester_id
        .as_deref()
        .ok_or(AppError::Forbidden)?;
    let rows = requests::list_owned_requests(&state.pool, requester_id).await?;
    Ok(Json(json!({
        "data": {
            "requests": rows.into_iter().map(|request| json!({
                "reference": request.public_reference,
                "title": request.title,
                "agency": {
                    "id": request.agency_id,
                    "name": request.agency_name
                },
                "status": request.status,
                "submitted_at": request.submitted_at,
                "due_at": request.due_at,
                "latest_public_message": request.latest_public_message
            })).collect::<Vec<_>>()
        }
    })))
}

async fn citizen_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(reference): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let authenticated =
        auth::session::require_requester(auth::session::required(&state.pool, &headers).await?)?;
    let requester_id = authenticated
        .requester_id
        .as_deref()
        .ok_or(AppError::Forbidden)?;
    let request = requests::load_owned_request(&state.pool, requester_id, &reference).await?;
    let events = requests::list_events(&state.pool, &request.id).await?;
    let messages = requests::list_messages(&state.pool, &request.id, false).await?;
    let documents = requests::list_documents(&state.pool, &request.id).await?;
    Ok(Json(json!({
        "data": {
            "request": {
                "reference": request.public_reference,
                "title": request.title,
                "description": request.description,
                "purpose": request.purpose,
                "agency": {"id":request.agency_id,"name":request.agency_name},
                "status": request.status,
                "submitted_at": request.submitted_at,
                "due_at": request.extended_due_at.unwrap_or(request.due_at)
            },
            "timeline": {
                "events": events.into_iter().map(|event| json!({
                    "type": event.event_type,
                    "summary": event.public_summary,
                    "created_at": event.created_at
                })).collect::<Vec<_>>(),
                "messages": messages.into_iter().map(|message| json!({
                    "actor": message.actor_label,
                    "body": message.body,
                    "created_at": message.created_at
                })).collect::<Vec<_>>(),
                "documents": documents.into_iter().map(|document| json!({
                    "id": document.id,
                    "title": document.title,
                    "media_type": document.media_type,
                    "size_bytes": document.byte_length,
                    "released_at": document.released_at,
                    "download_url": format!("/api/v1/documents/{}", document.id)
                })).collect::<Vec<_>>()
            }
        }
    })))
}

async fn api_request(
    State(state): State<AppState>,
    Path(reference): Path<String>,
) -> AppResult<Json<serde_json::Value>> {
    let request = requests::load_request(&state.pool, &reference).await?;
    let events = requests::list_events(&state.pool, &request.id).await?;
    let messages = requests::list_messages(&state.pool, &request.id, false).await?;
    let documents = requests::list_documents(&state.pool, &request.id).await?;
    let can_appeal = matches!(
        request.status.as_str(),
        "granted" | "partially_granted" | "denied"
    );
    let effective_due_at = request
        .extended_due_at
        .clone()
        .unwrap_or_else(|| request.due_at.clone());
    Ok(Json(
        json!({"data":{"reference":request.public_reference,"agency":request.agency_name,"title":request.title,"description":request.description,"purpose":request.purpose,"status":request.status,"due_at":effective_due_at,"can_appeal":can_appeal,"events":events.into_iter().map(|e|json!({"type":e.event_type,"summary":e.public_summary,"created_at":e.created_at})).collect::<Vec<_>>(),"messages":messages.into_iter().map(|m|json!({"actor":m.actor_label,"body":m.body,"created_at":m.created_at})).collect::<Vec<_>>(),"documents":documents.into_iter().map(|d|json!({"id":d.id,"title":d.title,"media_type":d.media_type,"size_bytes":d.byte_length,"released_at":d.released_at,"download_url":format!("/api/v1/documents/{}",d.id)})).collect::<Vec<_>>()}}),
    ))
}

async fn api_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<serde_json::Value>> {
    let actor = government_context(&state, &headers).await?.actor;
    let rows = notifications::list_for_persona(&state.pool, &actor.subject_id).await?;
    Ok(Json(
        json!({"data":rows.into_iter().map(|n|json!({"id":n.id,"request_id":n.request_id,"kind":n.kind,"message":n.message,"read_at":n.read_at,"created_at":n.created_at})).collect::<Vec<_>>() }),
    ))
}

async fn government_context(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<auth::AuthenticatedSession> {
    let authenticated = auth::session::required(&state.pool, headers).await?;
    auth::session::require_government(authenticated)
}

// Rotate only when establishing a session; data reads must preserve pending writes' token.
async fn government_session_context(
    state: &AppState,
    headers: &HeaderMap,
) -> AppResult<(ActorContext, String)> {
    let mut authenticated = government_context(state, headers).await?;
    let csrf = auth::session::rotate_csrf(&state.pool, &mut authenticated).await?;
    Ok((authenticated.actor, csrf))
}

fn ensure_government_read(actor: &ActorContext, agency_id: &str) -> AppResult<()> {
    if actor.has_role(Role::PlatformAdmin) || actor.agency_id.as_deref() == Some(agency_id) {
        Ok(())
    } else {
        Err(access_denied(actor, "government_read_scope"))
    }
}

fn access_denied(actor: &ActorContext, reason: &'static str) -> AppError {
    tracing::warn!(
        event = "auth.access_denied",
        component = "auth",
        operation = "authorization.check",
        outcome = "rejected",
        actor_id = %actor.subject_id,
        reason
    );
    AppError::Forbidden
}

fn non_empty(value: &str) -> Option<&str> {
    (!value.trim().is_empty()).then_some(value.trim())
}

fn csv_cell(value: &str) -> String {
    let safe = if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_owned()
    };
    format!("\"{}\"", safe.replace('"', "\"\""))
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("same-origin"));
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self'; connect-src 'self'; form-action 'self'; frame-ancestors 'none'; base-uri 'none'",
        ),
    );
    response
}
