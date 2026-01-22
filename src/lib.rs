//! # I Am Surging
//!
//! The easiest way to get crypto prices from Switchboard Surge.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use i_am_surging::get_price;
//!
//! #[tokio::main]
//! async fn main() {
//!     let price = get_price("btc").await.unwrap();
//!     println!("Bitcoin: ${:.2}", price.value);
//! }
//! ```
//!
//! ## Multiple Prices
//!
//! ```rust,no_run
//! use i_am_surging::get_prices;
//!
//! #[tokio::main]
//! async fn main() {
//!     let prices = get_prices(&["btc", "eth", "sol"]).await.unwrap();
//!     for p in prices {
//!         println!("{}: ${:.2}", p.symbol, p.value);
//!     }
//! }
//! ```

pub mod client;
pub mod error;
pub mod feed_loader;
pub mod server;
pub mod streaming;
pub mod types;

pub use client::SurgeClient;
pub use error::{Result, SurgeError};
pub use feed_loader::FeedLoader;
pub use streaming::Surge;
pub use types::{FeedPrice, SurgeConfig, SurgeEvent, SurgeFeedInfo, SurgeUpdate, SurgeUpdateData};

/// Normalize symbol input: "btc" -> "BTC/USD", "eth/usdt" -> "ETH/USDT"
pub fn normalize_symbol(input: &str) -> String {
    let input = input.trim().to_uppercase();
    if input.contains('/') {
        input
    } else {
        format!("{}/USD", input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_lowercase_shortcut() {
        assert_eq!(normalize_symbol("btc"), "BTC/USD");
        assert_eq!(normalize_symbol("eth"), "ETH/USD");
        assert_eq!(normalize_symbol("sol"), "SOL/USD");
    }

    #[test]
    fn test_normalize_uppercase_shortcut() {
        assert_eq!(normalize_symbol("BTC"), "BTC/USD");
        assert_eq!(normalize_symbol("ETH"), "ETH/USD");
    }

    #[test]
    fn test_normalize_mixed_case_shortcut() {
        assert_eq!(normalize_symbol("Btc"), "BTC/USD");
        assert_eq!(normalize_symbol("eTh"), "ETH/USD");
    }

    #[test]
    fn test_normalize_full_symbol_lowercase() {
        assert_eq!(normalize_symbol("btc/usd"), "BTC/USD");
        assert_eq!(normalize_symbol("eth/usdt"), "ETH/USDT");
        assert_eq!(normalize_symbol("sol/usdc"), "SOL/USDC");
    }

    #[test]
    fn test_normalize_full_symbol_uppercase() {
        assert_eq!(normalize_symbol("BTC/USD"), "BTC/USD");
        assert_eq!(normalize_symbol("ETH/USDT"), "ETH/USDT");
    }

    #[test]
    fn test_normalize_with_whitespace() {
        assert_eq!(normalize_symbol("  btc  "), "BTC/USD");
        assert_eq!(normalize_symbol("\teth\n"), "ETH/USD");
        assert_eq!(normalize_symbol(" sol/usdt "), "SOL/USDT");
    }
}

/// Get a single price. Accepts shortcuts like "btc" for "BTC/USD".
///
/// # Example
/// ```rust,no_run
/// # async fn example() {
/// let price = i_am_surging::get_price("btc").await.unwrap();
/// println!("${:.2}", price.value);
/// # }
/// ```
pub async fn get_price(symbol: &str) -> Result<FeedPrice> {
    SurgeClient::new()?.get_price(symbol).await
}

/// Get multiple prices at once.
///
/// # Example
/// ```rust,no_run
/// # async fn example() {
/// let prices = i_am_surging::get_prices(&["btc", "eth", "sol"]).await.unwrap();
/// # }
/// ```
pub async fn get_prices(symbols: &[&str]) -> Result<Vec<FeedPrice>> {
    SurgeClient::new()?.get_multiple_prices(symbols).await
}

/// List all available symbols.
pub fn list_symbols() -> Result<Vec<String>> {
    Ok(FeedLoader::load_default()?.get_all_symbols())
}
