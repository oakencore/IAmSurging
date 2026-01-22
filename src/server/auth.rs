//! API key authentication middleware

use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};

/// Middleware to validate API key from Authorization header
pub async fn require_api_key(request: Request, next: Next) -> Result<Response, StatusCode> {
    let expected_key = std::env::var("SURGE_API_KEY").unwrap_or_default();

    // If no API key is configured, skip auth
    if expected_key.is_empty() {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());

    match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            let token = &header[7..];
            if token == expected_key {
                Ok(next.run(request).await)
            } else {
                tracing::warn!("Invalid API key provided");
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        Some(_) => {
            tracing::warn!("Invalid authorization header format");
            Err(StatusCode::UNAUTHORIZED)
        }
        None => {
            tracing::warn!("Missing authorization header");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
