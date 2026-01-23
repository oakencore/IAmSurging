//! Surge API Server - Production-ready API for cryptocurrency prices
//!
//! Environment variables:
//! - SURGE_API_KEY: Required API key for authentication
//! - SURGE_PORT: Server port (default: 9000)
//! - SURGE_HOST: Server host (default: 0.0.0.0)
//! - RUST_LOG: Log level filter (default: info)

use i_am_surging::server::{app::ServerConfig, create_app, metrics::init_metrics};
use std::net::SocketAddr;
use tokio::signal;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing (JSON format for production)
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    // Check for API key
    match std::env::var("SURGE_API_KEY") {
        Ok(key) if !key.is_empty() => tracing::info!("API key authentication enabled"),
        _ => tracing::warn!("SURGE_API_KEY not set - API authentication is disabled"),
    }

    // Initialize metrics
    let _handle = init_metrics();
    tracing::info!("Prometheus metrics initialized");

    // Load configuration
    let config = ServerConfig::default();

    // Build application
    let app = match create_app() {
        Ok(app) => app,
        Err(e) => {
            tracing::error!("Failed to create application: {}", e);
            std::process::exit(1);
        }
    };

    // Parse bind address
    let addr: SocketAddr = config.addr().parse().expect("Invalid bind address");
    tracing::info!("Starting server on {}", addr);

    // Create TCP listener
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("Health check: http://{}/health", addr);
    tracing::info!("Metrics: http://{}/metrics", addr);
    tracing::info!("API docs: See API.md for endpoint documentation");

    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");

    tracing::info!("Server shutdown complete");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}
