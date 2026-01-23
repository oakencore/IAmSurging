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

    let Some(header) = auth_header else {
        tracing::warn!("Missing authorization header");
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Some(token) = header.strip_prefix("Bearer ") else {
        tracing::warn!("Invalid authorization header format");
        return Err(StatusCode::UNAUTHORIZED);
    };

    if token != expected_key {
        tracing::warn!("Invalid API key provided");
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}
