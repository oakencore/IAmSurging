//! Production API server for I Am Surging
//!
//! Provides REST and WebSocket APIs for cryptocurrency price data.

pub mod app;
pub mod auth;
pub mod metrics;
pub mod routes;
pub mod websocket;

pub use app::create_app;
