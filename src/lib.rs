//! # I Am Surging
//!
//! A production-ready Rust client for Switchboard Surge with real-time WebSocket streaming.
//!
//! ## Features
//!
//! - **Real-time streaming** - Sub-100ms latency WebSocket price updates
//! - **Event-driven** - Receive price updates as they happen
//! - **Auto-reconnection** - Exponential backoff for connection recovery
//! - **On-chain ready** - Convert updates to Oracle Quote instructions
//! - **2,266+ trading pairs** - Comprehensive cryptocurrency coverage
//! - **Type-safe** - Full Rust type safety with proper error handling
//!
//! ## Quick Start - Streaming (Recommended)
//!
//! ```rust,no_run
//! use i_am_surging::{Surge, SurgeEvent, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let mut surge = Surge::new("YOUR_API_KEY");
//!     let mut rx = surge.subscribe_events();
//!
//!     // Connect and subscribe to price feeds
//!     surge.connect_and_subscribe(vec!["BTC/USD", "ETH/USD", "SOL/USD"]).await?;
//!
//!     // Handle real-time price updates
//!     while let Ok(event) = rx.recv().await {
//!         match event {
//!             SurgeEvent::PriceUpdate(update) => {
//!                 println!("{}: ${:.2}", update.data.symbol, update.data.price);
//!
//!                 // Convert to on-chain instruction
//!                 let _ix = update.to_bundle_ix();
//!             }
//!             SurgeEvent::Connected => println!("Connected!"),
//!             SurgeEvent::Error(e) => eprintln!("Error: {}", e),
//!             _ => {}
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## SurgeClient - REST API
//!
//! For one-off price fetches using the REST API:
//!
//! ```rust,no_run
//! use i_am_surging::SurgeClient;
//!
//! #[tokio::main]
//! async fn main() -> i_am_surging::Result<()> {
//!     // api_key is your Solana wallet address
//!     let client = SurgeClient::new("YourSolanaWalletAddress")?;
//!
//!     // Single price fetch
//!     let price = client.get_price("SOL/USD").await?;
//!     println!("SOL: ${:.2}", price.value);
//!
//!     // Batch fetch multiple prices
//!     let prices = client.get_multiple_prices(&["BTC/USD", "ETH/USD"]).await?;
//!     for p in prices {
//!         println!("{}: ${:.2}", p.symbol, p.value);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### SurgeClient API
//!
//! **Constructor:**
//! - `SurgeClient::new(api_key: impl Into<String>) -> Result<Self>`
//!   - `api_key`: Solana wallet address used for API authentication
//!   - Returns error if `feedIds.json` is not found
//!
//! **Methods:**
//! - `async fn get_price(&self, symbol: &str) -> Result<FeedPrice>`
//!   - `symbol`: Trading pair format e.g., "BTC/USD", "ETH/USDT", "SOL/USDC"
//!   - Returns `FeedPrice` with `value` and `symbol` fields
//!
//! - `async fn get_multiple_prices(&self, symbols: &[&str]) -> Result<Vec<FeedPrice>>`
//!   - Batch fetch multiple prices in one call
//!   - `symbols`: Array of trading pair strings
//!
//! ### FeedPrice Struct
//!
//! ```rust,ignore
//! pub struct FeedPrice {
//!     pub value: f64,      // The price value
//!     pub symbol: String,  // The trading pair (e.g., "BTC/USD")
//!     pub feed_id: String, // The feed identifier
//! }
//! ```
//!
//! ## Requirements
//!
//! 1. `feedIds.json` must be in the working directory (CARGO_MANIFEST_DIR)
//! 2. Node.js 18+ must be installed
//! 3. npm dependencies must be installed (`npm install`)
//! 4. `fetch-price.js` helper script must be present
//!
//! ## Environment Variables
//!
//! - `SURGE_API_KEY` - Your Switchboard Surge API key (required)
//!
//! ## CLI Usage
//!
//! ```bash
//! # Stream real-time prices
//! surge-cli stream BTC/USD ETH/USD SOL/USD
//!
//! # Get single price
//! surge-cli get BTC/USD
//!
//! # List available symbols
//! surge-cli list --filter USD
//! ```
//!
//! ## Available Symbols
//!
//! The client supports 2,266+ trading pairs including:
//! - Major cryptocurrencies: BTC/USD, ETH/USD, SOL/USD
//! - Stablecoins: USDC/USD, USDT/USD
//! - Altcoins: TRX/USD, DOGE/USD, ADA/USD, and many more
//!
//! Use `client.get_all_symbols()` to see all available pairs.

// Module declarations
pub mod client;
pub mod error;
pub mod feed_loader;
pub mod streaming;
pub mod types;

// Re-exports for convenience
pub use client::SurgeClient;
pub use error::{Result, SurgeError};
pub use feed_loader::FeedLoader;
pub use streaming::Surge;
pub use types::{
    Feed, FeedPrice, OracleQuoteIx, SurgeConfig, SurgeEvent, SurgeFeedInfo, SurgeUpdate,
    SurgeUpdateData, Symbol,
};

/// Get the latest price for a symbol using a one-off client
///
/// This is a convenience function that creates a client, fetches the price, and returns it.
/// For multiple requests, consider creating a `SurgeClient` instance instead.
///
/// # Example
///
/// ```rust,no_run
/// use i_am_surging::get_price;
///
/// #[tokio::main]
/// async fn main() {
///     let price = get_price("BTC/USD", "YOUR_API_KEY").await.unwrap();
///     println!("BTC: ${}", price.value);
/// }
/// ```
pub async fn get_price(symbol: &str, api_key: &str) -> Result<FeedPrice> {
    let client = SurgeClient::new(api_key)?;
    client.get_price(symbol).await
}

/// Get prices for multiple symbols using a one-off client
///
/// This is a convenience function that creates a client and fetches multiple prices.
/// For repeated requests, consider creating a `SurgeClient` instance instead.
///
/// # Example
///
/// ```rust,no_run
/// use i_am_surging::get_multiple_prices;
///
/// #[tokio::main]
/// async fn main() {
///     let symbols = vec!["BTC/USD", "ETH/USD", "SOL/USD"];
///     let prices = get_multiple_prices(&symbols, "YOUR_API_KEY").await.unwrap();
///     for price in prices {
///         println!("{}", price);
///     }
/// }
/// ```
pub async fn get_multiple_prices(symbols: &[&str], api_key: &str) -> Result<Vec<FeedPrice>> {
    let client = SurgeClient::new(api_key)?;
    client.get_multiple_prices(symbols).await
}
