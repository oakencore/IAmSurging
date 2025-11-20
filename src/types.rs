use serde::{Deserialize, Serialize};

/// Represents a trading pair symbol (e.g., "BTC/USD")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Symbol {
    /// The base currency (e.g., "BTC")
    pub base: String,
    /// The quote currency (e.g., "USD")
    pub quote: String,
}

impl Symbol {
    /// Create a new symbol from base and quote
    pub fn new(base: impl Into<String>, quote: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            quote: quote.into(),
        }
    }

    /// Parse a symbol from string format "BASE/QUOTE"
    pub fn from_str(s: &str) -> crate::Result<Self> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(crate::error::SurgeError::InvalidSymbol(s.to_string()));
        }
        Ok(Self::new(parts[0], parts[1]))
    }

    /// Convert symbol to string format "BASE/QUOTE"
    pub fn to_string(&self) -> String {
        format!("{}/{}", self.base, self.quote)
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

/// Represents a price feed with its ID and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    /// The trading pair symbol
    pub symbol: Symbol,
    /// The feed ID (hex string)
    pub feed_id: String,
}

impl Feed {
    /// Create a new feed
    pub fn new(symbol: Symbol, feed_id: String) -> Self {
        Self { symbol, feed_id }
    }
}

/// Represents a price data point from a feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedPrice {
    /// The trading pair symbol
    pub symbol: String,
    /// The feed ID
    pub feed_id: String,
    /// The price value
    pub value: f64,
    /// Timestamp (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<i64>,
}

impl FeedPrice {
    /// Create a new feed price
    pub fn new(symbol: String, feed_id: String, value: f64) -> Self {
        Self {
            symbol,
            feed_id,
            value,
            timestamp: None,
        }
    }

    /// Set the timestamp
    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

impl std::fmt::Display for FeedPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ${:.6}", self.symbol, self.value)
    }
}

/// Response structure from Switchboard Crossbar simulate feed API
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct SimulateFeedResponse {
    pub results: Vec<f64>,
}

// ============================================================================
// Surge Streaming Types
// ============================================================================

/// Configuration for Surge streaming client
#[derive(Debug, Clone)]
pub struct SurgeConfig {
    /// API key for authentication
    pub api_key: String,
    /// WebSocket URL
    pub ws_url: String,
    /// REST API URL
    pub api_url: String,
    /// Enable auto-reconnection
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
    /// Initial reconnection delay in milliseconds
    pub initial_reconnect_delay_ms: u64,
}

impl Default for SurgeConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            ws_url: "wss://surge.switchboard.xyz/ws".to_string(),
            api_url: "https://surge.switchboard.xyz".to_string(),
            auto_reconnect: true,
            max_reconnect_attempts: 10,
            initial_reconnect_delay_ms: 1000,
        }
    }
}

/// Real-time price update from Surge WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeUpdate {
    /// The event type
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    /// Price update data
    pub data: SurgeUpdateData,
}

/// Price update data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeUpdateData {
    /// Trading pair symbol
    pub symbol: String,
    /// Current price
    pub price: f64,
    /// Source timestamp in milliseconds
    #[serde(rename = "source_ts_ms")]
    pub source_timestamp_ms: i64,
    /// Feed ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_id: Option<String>,
    /// Signature for on-chain verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Oracle public key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle_pubkey: Option<String>,
}

impl SurgeUpdate {
    /// Convert to on-chain Oracle Quote instruction data
    ///
    /// This is a placeholder for the actual implementation which would
    /// generate Solana transaction instructions for on-chain price updates.
    pub fn to_bundle_ix(&self) -> OracleQuoteIx {
        OracleQuoteIx {
            symbol: self.data.symbol.clone(),
            price: self.data.price,
            timestamp_ms: self.data.source_timestamp_ms,
            signature: self.data.signature.clone(),
        }
    }
}

/// Oracle Quote instruction data for on-chain execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleQuoteIx {
    /// Trading pair symbol
    pub symbol: String,
    /// Price value
    pub price: f64,
    /// Timestamp in milliseconds
    pub timestamp_ms: i64,
    /// Signature for verification
    pub signature: Option<String>,
}

/// Events emitted by the Surge streaming client
#[derive(Debug, Clone)]
pub enum SurgeEvent {
    /// Successfully connected to Surge
    Connected,
    /// Disconnected from Surge
    Disconnected,
    /// Received a price update
    PriceUpdate(SurgeUpdate),
    /// An error occurred
    Error(String),
    /// Attempting to reconnect
    Reconnecting {
        attempt: u32,
        delay_ms: u64,
    },
}

/// Request to subscribe/unsubscribe to symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    /// Action: "subscribe" or "unsubscribe"
    pub action: String,
    /// Symbols to subscribe/unsubscribe
    pub symbols: Vec<SymbolRequest>,
}

/// Symbol subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRequest {
    /// Trading pair symbol
    pub symbol: String,
}

/// Information about a Surge feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeFeedInfo {
    /// Trading pair symbol
    pub symbol: String,
    /// Feed ID
    #[serde(rename = "feedId")]
    pub feed_id: Option<String>,
    /// Whether the feed is active
    pub active: Option<bool>,
    /// Update frequency in milliseconds
    #[serde(rename = "updateFrequencyMs")]
    pub update_frequency_ms: Option<u64>,
}
