//! REST API route handlers

use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::SurgeError;
use crate::SurgeClient;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub client: Arc<SurgeClient>,
    pub ready: Arc<std::sync::atomic::AtomicBool>,
}

impl AppState {
    pub fn new() -> Result<Self, SurgeError> {
        Ok(Self {
            client: Arc::new(SurgeClient::new()?),
            ready: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        })
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Standard API response envelope
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Json<Self> {
        Json(Self {
            success: true,
            data: Some(data),
            error: None,
        })
    }

    pub fn error(message: impl Into<String>) -> Json<ApiResponse<()>> {
        Json(ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
        })
    }
}

/// Price data response
#[derive(Serialize)]
pub struct PriceResponse {
    pub symbol: String,
    pub feed_id: String,
    pub price: f64,
}

impl From<crate::FeedPrice> for PriceResponse {
    fn from(p: crate::FeedPrice) -> Self {
        Self {
            symbol: p.symbol,
            feed_id: p.feed_id,
            price: p.value,
        }
    }
}

/// Query parameters for multiple prices
#[derive(Deserialize)]
pub struct PricesQuery {
    pub symbols: String,
}

/// Query parameters for symbol listing
#[derive(Deserialize, Default)]
pub struct SymbolsQuery {
    pub filter: Option<String>,
}

/// Health check endpoint - always returns 200
pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy"
    }))
}

/// Readiness check endpoint - returns 200 if feeds are loaded
pub async fn ready(state: axum::extract::State<AppState>) -> impl IntoResponse {
    if state.is_ready() {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ready"
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "status": "not ready"
            })),
        )
    }
}

/// Prometheus metrics endpoint
pub async fn metrics_handler() -> impl IntoResponse {
    match super::metrics::get_prometheus_handle() {
        Some(handle) => (StatusCode::OK, handle.render()),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Metrics not initialized".to_string(),
        ),
    }
}

/// Get price for a single symbol
/// GET /v1/prices/:symbol
pub async fn get_price(
    state: axum::extract::State<AppState>,
    Path(symbol): Path<String>,
) -> impl IntoResponse {
    state
        .client
        .get_price(&symbol)
        .await
        .map(|price| (StatusCode::OK, ApiResponse::success(PriceResponse::from(price))).into_response())
        .unwrap_or_else(|e| (e.status_code(), ApiResponse::<()>::error(e.to_string())).into_response())
}

/// Get prices for multiple symbols
/// GET /v1/prices?symbols=btc,eth,sol
pub async fn get_prices(
    state: axum::extract::State<AppState>,
    Query(query): Query<PricesQuery>,
) -> impl IntoResponse {
    let symbols: Vec<&str> = query.symbols.split(',').map(str::trim).collect();

    if symbols.is_empty() {
        return (StatusCode::BAD_REQUEST, ApiResponse::<()>::error("No symbols provided")).into_response();
    }

    state
        .client
        .get_multiple_prices(&symbols)
        .await
        .map(|prices| {
            let response: Vec<PriceResponse> = prices.into_iter().map(PriceResponse::from).collect();
            (StatusCode::OK, ApiResponse::success(response)).into_response()
        })
        .unwrap_or_else(|e| (e.status_code(), ApiResponse::<()>::error(e.to_string())).into_response())
}

/// List available symbols
/// GET /v1/symbols?filter=sol
pub async fn list_symbols(
    state: axum::extract::State<AppState>,
    Query(query): Query<SymbolsQuery>,
) -> impl IntoResponse {
    let mut symbols = state.client.get_all_symbols();

    if let Some(ref filter_term) = query.filter {
        let filter_lower = filter_term.to_lowercase();
        symbols.retain(|s| s.to_lowercase().contains(&filter_lower));
    }

    let count = symbols.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "data": {
                "symbols": symbols,
                "count": count
            }
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // === AppState tests ===

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert!(state.is_ok(), "should create app state");
    }

    #[test]
    fn test_app_state_is_ready_default() {
        let state = AppState::new().unwrap();
        assert!(state.is_ready(), "should be ready by default");
    }

    #[test]
    fn test_app_state_ready_can_be_changed() {
        let state = AppState::new().unwrap();
        state.ready.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(!state.is_ready(), "should not be ready after change");
    }

    #[test]
    fn test_app_state_clone() {
        let state = AppState::new().unwrap();
        let cloned = state.clone();
        assert!(cloned.is_ready());
    }

    // === ApiResponse tests ===

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("test data");
        let json = serde_json::to_value(&response.0).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"], "test data");
        assert!(json.get("error").is_none() || json["error"].is_null());
    }

    #[test]
    fn test_api_response_success_with_struct() {
        let price = PriceResponse {
            symbol: "BTC/USD".to_string(),
            feed_id: "abc123".to_string(),
            price: 50000.0,
        };
        let response = ApiResponse::success(price);
        let json = serde_json::to_value(&response.0).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["symbol"], "BTC/USD");
        assert_eq!(json["data"]["feed_id"], "abc123");
        assert_eq!(json["data"]["price"], 50000.0);
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("Something went wrong");
        let json = serde_json::to_value(&response.0).unwrap();

        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Something went wrong");
        assert!(json.get("data").is_none() || json["data"].is_null());
    }

    #[test]
    fn test_api_response_error_with_string() {
        let response = ApiResponse::<()>::error(String::from("Error message"));
        let json = serde_json::to_value(&response.0).unwrap();

        assert_eq!(json["success"], false);
        assert_eq!(json["error"], "Error message");
    }

    // === PriceResponse tests ===

    #[test]
    fn test_price_response_serialization() {
        let price = PriceResponse {
            symbol: "ETH/USD".to_string(),
            feed_id: "def456".to_string(),
            price: 3000.50,
        };
        let json = serde_json::to_string(&price).unwrap();

        assert!(json.contains("\"symbol\":\"ETH/USD\""));
        assert!(json.contains("\"feed_id\":\"def456\""));
        assert!(json.contains("\"price\":3000.5"));
    }

    #[test]
    fn test_price_response_from_feed_price() {
        let feed_price = crate::FeedPrice {
            symbol: "BTC/USD".to_string(),
            feed_id: "abc123".to_string(),
            value: 50000.0,
        };
        let price_response = PriceResponse::from(feed_price);

        assert_eq!(price_response.symbol, "BTC/USD");
        assert_eq!(price_response.feed_id, "abc123");
        assert_eq!(price_response.price, 50000.0);
    }

    // === Query parsing tests ===

    #[test]
    fn test_prices_query_deserialization() {
        let query: PricesQuery = serde_json::from_str(r#"{"symbols": "btc,eth,sol"}"#).unwrap();
        assert_eq!(query.symbols, "btc,eth,sol");
    }

    #[test]
    fn test_symbols_query_default() {
        let query = SymbolsQuery::default();
        assert!(query.filter.is_none());
    }

    #[test]
    fn test_symbols_query_with_filter() {
        let query: SymbolsQuery = serde_json::from_str(r#"{"filter": "sol"}"#).unwrap();
        assert_eq!(query.filter, Some("sol".to_string()));
    }

    // === Symbol parsing helper tests ===

    #[test]
    fn test_parse_symbols_from_query() {
        let query = PricesQuery {
            symbols: "btc,eth,sol".to_string(),
        };
        let symbols: Vec<&str> = query.symbols.split(',').map(str::trim).collect();
        assert_eq!(symbols, vec!["btc", "eth", "sol"]);
    }

    #[test]
    fn test_parse_symbols_with_whitespace() {
        let query = PricesQuery {
            symbols: "btc , eth , sol".to_string(),
        };
        let symbols: Vec<&str> = query.symbols.split(',').map(str::trim).collect();
        assert_eq!(symbols, vec!["btc", "eth", "sol"]);
    }

    #[test]
    fn test_filter_symbols() {
        let all_symbols = vec![
            "BTC/USD".to_string(),
            "ETH/USD".to_string(),
            "SOL/USD".to_string(),
            "SOL/USDT".to_string(),
        ];

        let filter = "sol".to_lowercase();
        let filtered: Vec<&String> = all_symbols
            .iter()
            .filter(|s| s.to_lowercase().contains(&filter))
            .collect();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|s| *s == "SOL/USD"));
        assert!(filtered.iter().any(|s| *s == "SOL/USDT"));
    }
}
