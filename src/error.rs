use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Hindi nakita ang hinahanap na record")]
    NotFound,
    #[error("Wala kang pahintulot para sa action na ito")]
    Forbidden,
    #[error("Authentication is required.")]
    AuthenticationRequired,
    #[error("The session has expired.")]
    SessionExpired,
    #[error("Invalid email or password.")]
    InvalidCredentials,
    #[error("Invalid CSRF token.")]
    CsrfInvalid,
    #[error("Invalid request: {0}")]
    AuthValidation(String),
    #[error("Invalid request: {0}")]
    Validation(String),
    #[error("Masyadong maraming request; subukan muli mamaya")]
    RateLimited,
    #[error("AI service is temporarily unavailable")]
    AiUnavailable,
    #[error("Database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("Internal operation failed")]
    Internal,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()).into_response(),
            Self::Validation(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()).into_response()
            }
            Self::AiUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string()).into_response()
            }
            Self::Forbidden => api_error(StatusCode::FORBIDDEN, "forbidden", &self.to_string()),
            Self::AuthenticationRequired => api_error(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                &self.to_string(),
            ),
            Self::SessionExpired => api_error(
                StatusCode::UNAUTHORIZED,
                "session_expired",
                &self.to_string(),
            ),
            Self::InvalidCredentials => api_error(
                StatusCode::UNAUTHORIZED,
                "invalid_credentials",
                &self.to_string(),
            ),
            Self::CsrfInvalid => {
                api_error(StatusCode::FORBIDDEN, "csrf_invalid", &self.to_string())
            }
            Self::AuthValidation(_) => api_error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                &self.to_string(),
            ),
            Self::RateLimited => api_error(
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                &self.to_string(),
            ),
            Self::Database(error) => {
                tracing::error!(event = "database.operation_failed", error = %error);
                api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Internal operation failed",
                )
            }
            Self::Internal => api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                &self.to_string(),
            ),
        }
    }
}

fn api_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(json!({"error":{"code":code,"message":message}})),
    )
        .into_response()
}

pub type AppResult<T> = Result<T, AppError>;
