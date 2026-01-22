use crate::error::{Result, SurgeError};
use crate::feed_loader::FeedLoader;
use crate::normalize_symbol;
use crate::types::FeedPrice;

const CROSSBAR_URL: &str = "https://crossbar.switchboard.xyz";

/// Switchboard Surge client for fetching cryptocurrency prices
pub struct SurgeClient {
    http: reqwest::Client,
    feeds: FeedLoader,
}

#[derive(serde::Deserialize)]
struct SimulateResponse {
    results: Vec<String>,
}

impl SurgeClient {
    /// Create a new Surge client
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: reqwest::Client::new(),
            feeds: FeedLoader::load_default()?,
        })
    }

    /// Get the latest price for a symbol (e.g., "BTC/USD" or "btc")
    pub async fn get_price(&self, symbol: &str) -> Result<FeedPrice> {
        let symbol = normalize_symbol(symbol);
        let feed_id = self.feeds.get_feed_id(&symbol)?;
        let price = self.fetch_price(feed_id).await?;
        Ok(FeedPrice {
            symbol,
            feed_id: feed_id.to_string(),
            value: price,
        })
    }

    /// Get prices for multiple symbols
    pub async fn get_multiple_prices(&self, symbols: &[&str]) -> Result<Vec<FeedPrice>> {
        let mut prices = Vec::new();
        for symbol in symbols {
            match self.get_price(symbol).await {
                Ok(price) => prices.push(price),
                Err(e) => eprintln!("Warning: {}", e),
            }
        }
        Ok(prices)
    }

    /// Check if a symbol is available
    pub fn has_symbol(&self, symbol: &str) -> bool {
        let symbol = normalize_symbol(symbol);
        self.feeds.has_symbol(&symbol)
    }

    /// Get all available symbols
    pub fn get_all_symbols(&self) -> Vec<String> {
        self.feeds.get_all_symbols()
    }

    async fn fetch_price(&self, feed_id: &str) -> Result<f64> {
        let url = format!("{}/simulate/{}", CROSSBAR_URL, feed_id);
        let resp: Vec<SimulateResponse> = self.http.get(&url).send().await?.json().await?;
        resp.first()
            .and_then(|r| r.results.first())
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| SurgeError::ApiError(format!("No price data for feed {}", feed_id)))
    }
}

impl Default for SurgeClient {
    fn default() -> Self {
        Self::new().expect("Failed to create SurgeClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = SurgeClient::new();
        assert!(client.is_ok(), "should create client successfully");
    }

    #[test]
    fn test_client_default() {
        let client = SurgeClient::default();
        assert!(client.has_symbol("BTC/USD"));
    }

    #[test]
    fn test_client_has_symbol_with_shortcuts() {
        let client = SurgeClient::new().unwrap();
        assert!(client.has_symbol("btc"), "should find btc");
        assert!(client.has_symbol("BTC"), "should find BTC");
        assert!(client.has_symbol("BTC/USD"), "should find BTC/USD");
        assert!(client.has_symbol("btc/usd"), "should find btc/usd");
    }

    #[test]
    fn test_client_has_symbol_returns_false_for_invalid() {
        let client = SurgeClient::new().unwrap();
        assert!(!client.has_symbol("NOTREAL"));
        assert!(!client.has_symbol("fake/coin"));
    }

    #[test]
    fn test_client_get_all_symbols() {
        let client = SurgeClient::new().unwrap();
        let symbols = client.get_all_symbols();
        assert!(symbols.len() > 2000, "should have 2000+ symbols");
        assert!(symbols.contains(&"BTC/USD".to_string()));
        assert!(symbols.contains(&"ETH/USD".to_string()));
        assert!(symbols.contains(&"SOL/USD".to_string()));
    }
}
