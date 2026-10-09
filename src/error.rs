use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Hindi nakita ang hinahanap na record")]
    NotFound,
    #[error("Wala kang pahintulot para sa action na ito")]
    Forbidden,
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
        let (status, public_message) = match &self {
            Self::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            Self::Validation(_) => (StatusCode::UNPROCESSABLE_ENTITY, self.to_string()),
            Self::RateLimited => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            Self::AiUnavailable => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            Self::Database(error) => {
                tracing::error!(event = "database.operation_failed", error = %error);
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        (status, public_message).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
