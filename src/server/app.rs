//! Axum application builder with all routes and middleware

use axum::{
    middleware,
    routing::{get, Router},
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use super::auth::require_api_key;
use super::metrics::track_metrics;
use super::routes::{self, AppState};
use super::websocket;
use crate::error::SurgeError;

/// Create the Axum application with all routes and middleware
pub fn create_app() -> Result<Router, SurgeError> {
    let state = AppState::new()?;

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(routes::health))
        .route("/ready", get(routes::ready).with_state(state.clone()))
        .route("/metrics", get(routes::metrics_handler));

    // Protected API routes (auth required)
    let api_routes = Router::new()
        .route("/prices/:symbol", get(routes::get_price))
        .route("/prices", get(routes::get_prices))
        .route("/symbols", get(routes::list_symbols))
        .route("/stream", get(websocket::ws_handler))
        .with_state(state.clone())
        .layer(middleware::from_fn(require_api_key));

    // Combine all routes
    let app = Router::new()
        .merge(public_routes)
        .nest("/v1", api_routes)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::from_fn(track_metrics))
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(
                    CorsLayer::new()
                        .allow_origin(Any)
                        .allow_methods(Any)
                        .allow_headers(Any),
                ),
        );

    Ok(app)
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("SURGE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("SURGE_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(9000),
        }
    }
}

impl ServerConfig {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
