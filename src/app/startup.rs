use std::{str::FromStr, sync::Arc};

use axum::Router;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tokio::net::TcpListener;

use crate::{
    ai::{self, OpenAiCompatibleProvider},
    auth::{AuthService, seed_demo_auth},
    error::AppResult,
    organization,
    requester_ai::RequesterAiService,
};

use super::{AppConfig, AppState, router};

pub struct RunningApp {
    pub bind: String,
    pub listener: TcpListener,
    pub router: Router,
}

pub async fn start() -> AppResult<RunningApp> {
    let config = Arc::new(AppConfig::from_env()?);
    tokio::fs::create_dir_all(config.data_dir.join("files"))
        .await
        .map_err(|_| crate::error::AppError::Internal)?;
    let pool = connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await.map_err(|error| {
        tracing::error!(event="database.migration_failed", error=%error);
        crate::error::AppError::Internal
    })?;
    if config.demo_reset_on_start {
        reset_demo_data(&pool).await?;
    }
    organization::seed_reference_data(&pool).await?;
    seed_demo_auth(&pool).await?;
    let ai = match (&config.ai_chat_completions_url, &config.ai_api_key) {
        (Some(chat_completions_url), Some(api_key)) => {
            Some(Arc::new(OpenAiCompatibleProvider::new(
                chat_completions_url.clone(),
                api_key.clone(),
                config.ai_model.clone(),
            )?) as Arc<dyn ai::AiProvider>)
        }
        _ => {
            tracing::warn!(
                event = "ai.configuration_missing",
                outcome = "degraded",
                "AI jobs will remain pending until an HTTPS provider endpoint and API key are configured"
            );
            None
        }
    };
    let state = AppState {
        pool,
        config: config.clone(),
        auth: AuthService::new()?,
        ai,
        requester_ai: RequesterAiService::new()?,
    };
    spawn_ai_worker(state.clone());
    let listener = TcpListener::bind(&config.bind)
        .await
        .map_err(|_| crate::error::AppError::Internal)?;
    Ok(RunningApp {
        bind: config.bind.clone(),
        listener,
        router: router(state),
    })
}

pub async fn connect(database_url: &str) -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)
        .map_err(|_| crate::error::AppError::Validation("Invalid SQLite URL".into()))?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    Ok(SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(options)
        .await?)
}

fn spawn_ai_worker(state: AppState) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = ai::process_next_job(&state.pool, state.ai.clone()).await {
                tracing::error!(event="ai.worker_failed", error=%error);
            }
        }
    });
}

async fn reset_demo_data(pool: &SqlitePool) -> AppResult<()> {
    let mut transaction = pool.begin().await?;
    for table in [
        "auth_sessions",
        "auth_users",
        "notifications",
        "released_documents",
        "ai_draft_reviews",
        "ai_jobs",
        "triage_results",
        "messages",
        "request_events",
        "foi_requests",
        "request_batches",
        "requesters",
        "audit_events",
        "rate_limits",
    ] {
        sqlx::query(&format!("DELETE FROM {table}"))
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    tracing::info!(event = "demo.reset_completed", outcome = "success");
    Ok(())
}
