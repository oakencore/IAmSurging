use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SurgeError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Feed not found: {0}")]
    FeedNotFound(String),

    #[error("API error: {0}")]
    ApiError(String),
}

impl SurgeError {
    /// Map error to HTTP status code
    pub fn status_code(&self) -> StatusCode {
        match self {
            SurgeError::FeedNotFound(_) => StatusCode::NOT_FOUND,
            SurgeError::ApiError(_) => StatusCode::BAD_GATEWAY,
            SurgeError::HttpError(_) => StatusCode::BAD_GATEWAY,
            SurgeError::JsonError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SurgeError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for SurgeError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = Json(serde_json::json!({
            "success": false,
            "error": self.to_string()
        }));
        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, SurgeError>;
