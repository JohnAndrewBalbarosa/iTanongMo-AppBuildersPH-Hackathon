use std::{env, path::PathBuf, sync::Arc};

use sqlx::SqlitePool;

use crate::{
    ai::AiProvider,
    auth::AuthService,
    error::{AppError, AppResult},
    requester_ai::RequesterAiService,
};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub bind: String,
    pub database_url: String,
    pub data_dir: PathBuf,
    pub public_origin: String,
    pub frontend_origin: Option<String>,
    pub ai_chat_completions_url: Option<String>,
    pub ai_api_key: Option<String>,
    pub ai_model: String,
    pub requester_ai_enabled: bool,
    pub guest_ai_enabled: bool,
    pub demo_reset_on_start: bool,
    pub enable_persona_login: bool,
}

impl AppConfig {
    pub fn from_env() -> AppResult<Self> {
        let bind = env::var("ITANONGMO_BIND").unwrap_or_else(|_| "127.0.0.1:4320".into());
        let database_url = env::var("ITANONGMO_DATABASE_URL")
            .unwrap_or_else(|_| "sqlite://var/itanongmo.db?mode=rwc".into());
        let data_dir =
            PathBuf::from(env::var("ITANONGMO_DATA_DIR").unwrap_or_else(|_| "var".into()));
        let public_origin =
            env::var("ITANONGMO_PUBLIC_ORIGIN").unwrap_or_else(|_| format!("http://{bind}"));
        if !(public_origin.starts_with("http://127.0.0.1")
            || public_origin.starts_with("http://localhost")
            || public_origin.starts_with("https://"))
        {
            return Err(AppError::Validation(
                "Production public origin must use HTTPS".into(),
            ));
        }
        let frontend_origin = non_empty_env("ITANONGMO_FRONTEND_ORIGIN");
        if let Some(origin) = &frontend_origin {
            validate_frontend_origin(origin)?;
        }
        let requester_ai_enabled =
            env::var("ITANONGMO_REQUESTER_AI_ENABLED").ok().as_deref() == Some("true");
        let guest_ai_enabled =
            env::var("ITANONGMO_GUEST_AI_ENABLED").ok().as_deref() == Some("true");
        let explicit_local_development =
            env::var("ITANONGMO_LOCAL_DEVELOPMENT").ok().as_deref() == Some("true");
        if requester_ai_enabled
            && !explicit_local_development
            && !public_origin.starts_with("https://")
        {
            return Err(AppError::Validation(
                "Requester AI requires HTTPS outside local development".into(),
            ));
        }
        if guest_ai_enabled && !requester_ai_enabled {
            return Err(AppError::Validation(
                "Guest AI requires requester AI to be enabled".into(),
            ));
        }
        Ok(Self {
            bind,
            database_url,
            data_dir,
            public_origin,
            frontend_origin,
            ai_chat_completions_url: configured_ai_chat_completions_url(),
            ai_api_key: non_empty_env("ITANONGMO_AI_API_KEY")
                .or_else(|| non_empty_env("ITANONGMO_AI_PROXY_TOKEN")),
            ai_model: env::var("ITANONGMO_AI_MODEL")
                .unwrap_or_else(|_| "gemini-2.5-flash-lite".into()),
            requester_ai_enabled,
            guest_ai_enabled,
            demo_reset_on_start: env::var("ITANONGMO_DEMO_RESET_ON_START").ok().as_deref()
                == Some("true"),
            enable_persona_login: env::var("ITANONGMO_ENABLE_PERSONA_LOGIN").ok().as_deref()
                == Some("true"),
        })
    }
}

fn configured_ai_chat_completions_url() -> Option<String> {
    non_empty_env("ITANONGMO_AI_CHAT_COMPLETIONS_URL")
        .map(|url| url.trim_end_matches('/').to_owned())
        .or_else(|| {
            non_empty_env("ITANONGMO_AI_BASE_URL")
                .map(|base_url| format!("{}/v1/chat/completions", base_url.trim_end_matches('/')))
        })
}

fn validate_frontend_origin(origin: &str) -> AppResult<()> {
    let url = reqwest::Url::parse(origin)
        .map_err(|_| AppError::Validation("Invalid frontend origin".into()))?;
    let is_local = matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    let secure_or_local = url.scheme() == "https" || (url.scheme() == "http" && is_local);
    let is_origin_only = url.path() == "/" && url.query().is_none() && url.fragment().is_none();
    if !secure_or_local || !is_origin_only {
        return Err(AppError::Validation(
            "Frontend origin must be an HTTPS origin without a path".into(),
        ));
    }
    Ok(())
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub config: Arc<AppConfig>,
    pub auth: AuthService,
    pub ai: Option<Arc<dyn AiProvider>>,
    pub requester_ai: RequesterAiService,
}
