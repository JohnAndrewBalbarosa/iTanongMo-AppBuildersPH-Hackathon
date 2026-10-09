use std::{env, path::PathBuf, sync::Arc};

use sqlx::SqlitePool;

use crate::{
    ai::AiProvider,
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
    pub ai_base_url: Option<String>,
    pub ai_proxy_token: Option<String>,
    pub ai_model: String,
    pub requester_ai_enabled: bool,
    pub requester_ai_token_ttl_seconds: u64,
    pub demo_reset_on_start: bool,
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
        Ok(Self {
            bind,
            database_url,
            data_dir,
            public_origin,
            frontend_origin,
            ai_base_url: non_empty_env("ITANONGMO_AI_BASE_URL"),
            ai_proxy_token: non_empty_env("ITANONGMO_AI_PROXY_TOKEN"),
            ai_model: env::var("ITANONGMO_AI_MODEL").unwrap_or_else(|_| "Qwen/Qwen3-4B".into()),
            requester_ai_enabled,
            requester_ai_token_ttl_seconds: env::var("ITANONGMO_REQUESTER_AI_TOKEN_TTL_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(900)
                .clamp(60, 3_600),
            demo_reset_on_start: env::var("ITANONGMO_DEMO_RESET_ON_START").ok().as_deref()
                == Some("true"),
        })
    }
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
    pub ai: Option<Arc<dyn AiProvider>>,
    pub requester_ai: RequesterAiService,
}
