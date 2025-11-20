use reqwest::Client;

use crate::error::{Result, SurgeError};
use crate::feed_loader::FeedLoader;
use crate::types::FeedPrice;

/// Default Switchboard Crossbar URL
pub const DEFAULT_GATEWAY_URL: &str = "http://crossbar.switchboard.xyz";

/// Switchboard Surge client for fetching price feeds
pub struct SurgeClient {
    /// HTTP client
    #[allow(dead_code)]
    http_client: Client,
    /// Feed loader
    feed_loader: FeedLoader,
    /// Gateway URL
    #[allow(dead_code)]
    gateway_url: String,
    /// API key (wallet address)
    #[allow(dead_code)]
    api_key: String,
}

impl SurgeClient {
    /// Create a new Surge client with API key
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let feed_loader = FeedLoader::load_default()?;

        Ok(Self {
            http_client: Client::new(),
            feed_loader,
            gateway_url: DEFAULT_GATEWAY_URL.to_string(),
            api_key: api_key.into(),
        })
    }

    /// Create a new Surge client with custom gateway URL
    pub fn with_gateway(api_key: impl Into<String>, gateway_url: impl Into<String>) -> Result<Self> {
        let feed_loader = FeedLoader::load_default()?;

        Ok(Self {
            http_client: Client::new(),
            feed_loader,
            gateway_url: gateway_url.into(),
            api_key: api_key.into(),
        })
    }

    /// Get the latest price for a symbol
    pub async fn get_price(&self, symbol: &str) -> Result<FeedPrice> {
        let feed = self.feed_loader.get_feed(symbol)?;
        let value = self.simulate_feed(&feed.feed_id).await?;

        Ok(FeedPrice::new(
            feed.symbol.to_string(),
            feed.feed_id,
            value,
        ))
    }

    /// Get prices for multiple symbols
    pub async fn get_multiple_prices(&self, symbols: &[&str]) -> Result<Vec<FeedPrice>> {
        let mut prices = Vec::new();

        for symbol in symbols {
            match self.get_price(symbol).await {
                Ok(price) => prices.push(price),
                Err(e) => {
                    eprintln!("Warning: Failed to get price for {}: {}", symbol, e);
                }
            }
        }

        Ok(prices)
    }

    /// Check if a symbol is available
    pub fn has_symbol(&self, symbol: &str) -> bool {
        self.feed_loader.has_symbol(symbol)
    }

    /// Get all available symbols
    pub fn get_all_symbols(&self) -> Vec<String> {
        self.feed_loader.get_all_symbols()
    }

    /// Simulate a feed to get the current price
    ///
    /// Note: This currently uses a Node.js helper script to handle protobuf encoding/decoding
    /// until full protobuf support is added to the Rust client.
    async fn simulate_feed(&self, feed_id: &str) -> Result<f64> {
        use std::process::Command;

        // Call the Node.js helper script that uses the Switchboard SDK
        let output = Command::new("node")
            .arg("fetch-price.js")
            .arg(feed_id)
            .env("ANCHOR_WALLET", std::env::var("ANCHOR_WALLET").unwrap_or_default())
            .env("ANCHOR_PROVIDER_URL", std::env::var("ANCHOR_PROVIDER_URL").unwrap_or_default())
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .map_err(|e| SurgeError::ApiError(format!("Failed to execute helper script: {}", e)))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(SurgeError::ApiError(format!(
                "Helper script failed: {}",
                error_msg.trim()
            )));
        }

        // Parse the price from stdout
        let price_str = String::from_utf8_lossy(&output.stdout);
        let price = price_str.trim().parse::<f64>()
            .map_err(|e| SurgeError::ApiError(format!("Failed to parse price: {}", e)))?;

        Ok(price)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_surge_client() {
        // This test requires SURGE_API_KEY environment variable and feedIds.json
        if let Ok(api_key) = std::env::var("SURGE_API_KEY") {
            if let Ok(client) = SurgeClient::new(&api_key) {
                // Test that we can create a client
                assert!(client.has_symbol("BTC/USD"));
            }
        }
    }
}
