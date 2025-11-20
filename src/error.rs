use thiserror::Error;

/// Custom error type for Surge client operations
#[derive(Debug, Error)]
pub enum SurgeError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// IO error (e.g., reading feedIds.json)
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Feed not found in feedIds.json
    #[error("Feed not found for symbol: {0}")]
    FeedNotFound(String),

    /// Invalid feed ID format
    #[error("Invalid feed ID: {0}")]
    InvalidFeedId(String),

    /// API returned an error
    #[error("API error: {0}")]
    ApiError(String),

    /// No price data returned
    #[error("No price data returned for feed: {0}")]
    NoPriceData(String),

    /// Invalid symbol format
    #[error("Invalid symbol format: {0}. Expected format: BASE/QUOTE (e.g., BTC/USD)")]
    InvalidSymbol(String),

    /// Subscription error
    #[error("Subscription error: {0}")]
    SubscriptionError(String),
}

/// Convenience Result type for Surge operations
pub type Result<T> = std::result::Result<T, SurgeError>;
