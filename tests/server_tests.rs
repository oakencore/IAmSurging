//! Integration tests for the Surge API server
//!
//! These tests verify the REST API endpoints work correctly.
//!
//! Note: These tests run with authentication DISABLED (SURGE_API_KEY unset).
//! Auth middleware behavior is tested via unit tests in auth.rs.

use axum::{body::Body, http::{Request, StatusCode}, Router};
use i_am_surging::server::{app::ServerConfig, create_app};
use serde_json::Value;
use std::sync::Once;
use tower::ServiceExt;

// Ensure env is clean before all tests
static INIT: Once = Once::new();

fn ensure_no_auth() {
    INIT.call_once(|| {
        std::env::remove_var("SURGE_API_KEY");
    });
}

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a test app instance (no auth)
fn create_test_app() -> Router {
    ensure_no_auth();
    create_app().expect("Failed to create app")
}

/// Parse JSON response body
async fn parse_json_body(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");
    serde_json::from_slice(&body).expect("Failed to parse JSON")
}

// =============================================================================
// Health Check Tests
// =============================================================================

#[tokio::test]
async fn test_health_endpoint_returns_200() {
    let app = create_test_app();

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint_returns_healthy_status() {
    let app = create_test_app();

    let response = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["status"], "healthy");
}


// =============================================================================
// Readiness Check Tests
// =============================================================================

#[tokio::test]
async fn test_ready_endpoint_returns_200_when_ready() {
    let app = create_test_app();

    let response = app
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ready_endpoint_returns_ready_status() {
    let app = create_test_app();

    let response = app
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["status"], "ready");
}

// Note: Authentication is tested via unit tests in src/server/auth.rs
// Integration tests with env var changes don't work reliably in parallel

// =============================================================================
// Single Price Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_get_price_btc_returns_200() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_price_btc_returns_correct_symbol() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);
    assert_eq!(json["data"]["symbol"], "BTC/USD");
}

#[tokio::test]
async fn test_get_price_btc_returns_positive_price() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    let price = json["data"]["price"].as_f64().unwrap();
    assert!(price > 0.0, "Price should be positive");
}

#[tokio::test]
async fn test_get_price_btc_returns_feed_id() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    let feed_id = json["data"]["feed_id"].as_str().unwrap();
    assert_eq!(feed_id.len(), 64, "Feed ID should be 64 hex chars");
}

#[tokio::test]
async fn test_get_price_eth_returns_correct_symbol() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/eth")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["data"]["symbol"], "ETH/USD");
}

#[tokio::test]
async fn test_get_price_sol_returns_correct_symbol() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/sol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["data"]["symbol"], "SOL/USD");
}

#[tokio::test]
async fn test_get_price_invalid_symbol_returns_404() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/NOTACOIN")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_price_invalid_symbol_returns_error_message() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/FAKECOIN")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], false);
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

// =============================================================================
// Multiple Prices Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_get_prices_multiple_returns_200() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices?symbols=btc,eth,sol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_prices_multiple_returns_all_symbols() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices?symbols=btc,eth")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);

    let data = json["data"].as_array().unwrap();
    assert_eq!(data.len(), 2);

    let symbols: Vec<&str> = data.iter().map(|p| p["symbol"].as_str().unwrap()).collect();
    assert!(symbols.contains(&"BTC/USD"));
    assert!(symbols.contains(&"ETH/USD"));
}

#[tokio::test]
async fn test_get_prices_with_spaces_in_symbols() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices?symbols=btc%20,%20eth")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);
}

#[tokio::test]
async fn test_get_prices_single_symbol() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices?symbols=btc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);

    let data = json["data"].as_array().unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data[0]["symbol"], "BTC/USD");
}

// =============================================================================
// Symbols Listing Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_list_symbols_returns_200() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/symbols")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_symbols_returns_many_symbols() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/symbols")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);

    let count = json["data"]["count"].as_u64().unwrap();
    assert!(count > 2000, "Should have 2000+ symbols");
}

#[tokio::test]
async fn test_list_symbols_contains_major_symbols() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/symbols")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    let symbols = json["data"]["symbols"].as_array().unwrap();
    let symbol_strs: Vec<&str> = symbols.iter().map(|s| s.as_str().unwrap()).collect();

    assert!(symbol_strs.contains(&"BTC/USD"));
    assert!(symbol_strs.contains(&"ETH/USD"));
    assert!(symbol_strs.contains(&"SOL/USD"));
}

#[tokio::test]
async fn test_list_symbols_with_filter() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/symbols?filter=sol")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);

    let symbols = json["data"]["symbols"].as_array().unwrap();
    for sym in symbols {
        let s = sym.as_str().unwrap().to_lowercase();
        assert!(s.contains("sol"), "All filtered symbols should contain 'sol'");
    }
}

#[tokio::test]
async fn test_list_symbols_filter_is_case_insensitive() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/symbols?filter=SOL")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    let symbols = json["data"]["symbols"].as_array().unwrap();

    assert!(!symbols.is_empty(), "Should find SOL symbols with uppercase filter");
}

#[tokio::test]
async fn test_list_symbols_filter_no_matches() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/symbols?filter=xyznotacoin123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;
    assert_eq!(json["success"], true);

    let symbols = json["data"]["symbols"].as_array().unwrap();
    assert!(symbols.is_empty(), "Should return empty for non-matching filter");
    assert_eq!(json["data"]["count"], 0);
}

// =============================================================================
// Server Configuration Tests
// =============================================================================

#[test]
fn test_server_config_default_values() {
    std::env::remove_var("SURGE_HOST");
    std::env::remove_var("SURGE_PORT");

    let config = ServerConfig::default();
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 9000);
}

#[test]
fn test_server_config_from_env() {
    std::env::set_var("SURGE_HOST", "127.0.0.1");
    std::env::set_var("SURGE_PORT", "8080");

    let config = ServerConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);

    // Clean up
    std::env::remove_var("SURGE_HOST");
    std::env::remove_var("SURGE_PORT");
}

#[test]
fn test_server_config_addr() {
    let config = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 9000,
    };
    assert_eq!(config.addr(), "0.0.0.0:9000");
}

// =============================================================================
// Error Response Format Tests
// =============================================================================

#[tokio::test]
async fn test_error_response_format() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/prices/INVALID")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let json = parse_json_body(response).await;

    // Verify error response structure
    assert!(json.get("success").is_some(), "Should have 'success' field");
    assert!(json.get("error").is_some(), "Should have 'error' field");
    assert_eq!(json["success"], false);
    assert!(json["error"].is_string());
}

// =============================================================================
// 404 Tests
// =============================================================================

#[tokio::test]
async fn test_unknown_endpoint_returns_404() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_unknown_root_endpoint_returns_404() {
    let app = create_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
