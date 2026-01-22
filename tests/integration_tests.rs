use i_am_surging::{get_price, get_prices, list_symbols, FeedLoader, FeedPrice, SurgeClient, SurgeError};

// =============================================================================
// Convenience Function Tests
// =============================================================================

#[tokio::test]
async fn test_get_price_btc() {
    let price = get_price("btc").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
    assert!(price.value > 0.0);
    assert!(!price.feed_id.is_empty());
}

#[tokio::test]
async fn test_get_price_eth() {
    let price = get_price("eth").await.unwrap();
    assert_eq!(price.symbol, "ETH/USD");
    assert!(price.value > 0.0);
}

#[tokio::test]
async fn test_get_price_sol() {
    let price = get_price("sol").await.unwrap();
    assert_eq!(price.symbol, "SOL/USD");
    assert!(price.value > 0.0);
}

#[tokio::test]
async fn test_get_prices_multiple() {
    let prices = get_prices(&["btc", "eth", "sol"]).await.unwrap();
    assert_eq!(prices.len(), 3);

    let symbols: Vec<&str> = prices.iter().map(|p| p.symbol.as_str()).collect();
    assert!(symbols.contains(&"BTC/USD"));
    assert!(symbols.contains(&"ETH/USD"));
    assert!(symbols.contains(&"SOL/USD"));
}

#[tokio::test]
async fn test_get_prices_empty() {
    let prices = get_prices(&[]).await.unwrap();
    assert!(prices.is_empty());
}

#[test]
fn test_list_symbols() {
    let symbols = list_symbols().unwrap();
    assert!(symbols.len() > 2000, "should have 2000+ symbols");
    assert!(symbols.contains(&"BTC/USD".to_string()));
    assert!(symbols.contains(&"ETH/USD".to_string()));
    assert!(symbols.contains(&"SOL/USD".to_string()));
}

// =============================================================================
// Symbol Shortcut Tests
// =============================================================================

#[tokio::test]
async fn test_shortcut_lowercase() {
    let price = get_price("btc").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
}

#[tokio::test]
async fn test_shortcut_uppercase() {
    let price = get_price("BTC").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
}

#[tokio::test]
async fn test_shortcut_mixed_case() {
    let price = get_price("Btc").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
}

#[tokio::test]
async fn test_full_symbol_lowercase() {
    let price = get_price("btc/usd").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
}

#[tokio::test]
async fn test_full_symbol_uppercase() {
    let price = get_price("BTC/USD").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
}

#[tokio::test]
async fn test_shortcut_with_whitespace() {
    let price = get_price("  btc  ").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
}

// =============================================================================
// Price Validation Tests
// =============================================================================

#[tokio::test]
async fn test_btc_price_reasonable() {
    let price = get_price("btc").await.unwrap();
    assert!(price.value > 1_000.0, "BTC should be > $1,000");
    assert!(price.value < 10_000_000.0, "BTC should be < $10M");
}

#[tokio::test]
async fn test_eth_price_reasonable() {
    let price = get_price("eth").await.unwrap();
    assert!(price.value > 100.0, "ETH should be > $100");
    assert!(price.value < 1_000_000.0, "ETH should be < $1M");
}

#[tokio::test]
async fn test_sol_price_reasonable() {
    let price = get_price("sol").await.unwrap();
    assert!(price.value > 1.0, "SOL should be > $1");
    assert!(price.value < 100_000.0, "SOL should be < $100K");
}

#[tokio::test]
async fn test_price_has_feed_id() {
    let price = get_price("btc").await.unwrap();
    assert_eq!(price.feed_id.len(), 64, "feed_id should be 64 chars");
    assert!(
        price.feed_id.chars().all(|c| c.is_ascii_hexdigit()),
        "feed_id should be hex"
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_invalid_symbol_error() {
    let result = get_price("NOTACOIN").await;
    assert!(result.is_err(), "should fail for invalid symbol");
}

#[tokio::test]
async fn test_invalid_full_symbol_error() {
    let result = get_price("FAKE/COIN").await;
    assert!(result.is_err(), "should fail for invalid full symbol");
}

#[tokio::test]
async fn test_empty_symbol_error() {
    let result = get_price("").await;
    assert!(result.is_err(), "should fail for empty symbol");
}

// =============================================================================
// SurgeClient Tests
// =============================================================================

#[test]
fn test_client_new() {
    let client = SurgeClient::new();
    assert!(client.is_ok(), "should create client");
}

#[test]
fn test_client_default() {
    let client = SurgeClient::default();
    assert!(client.has_symbol("btc"));
}

#[test]
fn test_client_has_symbol_shortcuts() {
    let client = SurgeClient::new().unwrap();
    assert!(client.has_symbol("btc"));
    assert!(client.has_symbol("BTC"));
    assert!(client.has_symbol("btc/usd"));
    assert!(client.has_symbol("BTC/USD"));
}

#[test]
fn test_client_has_symbol_invalid() {
    let client = SurgeClient::new().unwrap();
    assert!(!client.has_symbol("FAKECOIN"));
    assert!(!client.has_symbol("NOT/REAL"));
}

#[test]
fn test_client_get_all_symbols() {
    let client = SurgeClient::new().unwrap();
    let symbols = client.get_all_symbols();
    assert!(symbols.len() > 2000);
    assert!(symbols.contains(&"BTC/USD".to_string()));
}

#[tokio::test]
async fn test_client_get_price() {
    let client = SurgeClient::new().unwrap();
    let price = client.get_price("btc").await.unwrap();
    assert_eq!(price.symbol, "BTC/USD");
    assert!(price.value > 0.0);
}

#[tokio::test]
async fn test_client_get_multiple_prices() {
    let client = SurgeClient::new().unwrap();
    let prices = client.get_multiple_prices(&["btc", "eth"]).await.unwrap();
    assert_eq!(prices.len(), 2);
}

// =============================================================================
// FeedLoader Tests
// =============================================================================

#[test]
fn test_feed_loader_and_client_consistency() {
    let loader = FeedLoader::load_default().unwrap();
    let client = SurgeClient::new().unwrap();
    assert_eq!(loader.get_all_symbols(), client.get_all_symbols());
}

#[test]
fn test_feed_loader_len() {
    let loader = FeedLoader::load_default().unwrap();
    assert!(loader.len() > 2000);
    assert!(!loader.is_empty());
}

// =============================================================================
// FeedPrice Tests
// =============================================================================

#[test]
fn test_feed_price_display() {
    let price = FeedPrice {
        symbol: "BTC/USD".to_string(),
        feed_id: "abc".to_string(),
        value: 50000.123456,
    };
    let display = format!("{}", price);
    assert!(display.contains("BTC/USD"));
    assert!(display.contains("50000.123456"));
}

#[test]
fn test_feed_price_json_roundtrip() {
    let price = FeedPrice {
        symbol: "ETH/USD".to_string(),
        feed_id: "def".to_string(),
        value: 3000.50,
    };
    let json = serde_json::to_string(&price).unwrap();
    let parsed: FeedPrice = serde_json::from_str(&json).unwrap();
    assert_eq!(price.symbol, parsed.symbol);
    assert_eq!(price.feed_id, parsed.feed_id);
    assert_eq!(price.value, parsed.value);
}

// =============================================================================
// SurgeError Tests
// =============================================================================

#[test]
fn test_surge_error_feed_not_found_status_code() {
    use axum::http::StatusCode;
    let err = SurgeError::FeedNotFound("TEST/COIN".to_string());
    assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
}

#[test]
fn test_surge_error_api_error_status_code() {
    use axum::http::StatusCode;
    let err = SurgeError::ApiError("upstream error".to_string());
    assert_eq!(err.status_code(), StatusCode::BAD_GATEWAY);
}

#[test]
fn test_surge_error_display() {
    let err = SurgeError::FeedNotFound("FAKE/COIN".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Feed not found"));
    assert!(display.contains("FAKE/COIN"));
}

#[test]
fn test_surge_error_api_error_display() {
    let err = SurgeError::ApiError("Connection timeout".to_string());
    let display = format!("{}", err);
    assert!(display.contains("API error"));
    assert!(display.contains("Connection timeout"));
}
