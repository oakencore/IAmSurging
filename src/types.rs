use serde::{Deserialize, Serialize};

/// Price data from a feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedPrice {
    pub symbol: String,
    pub feed_id: String,
    pub value: f64,
}

impl std::fmt::Display for FeedPrice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ${:.6}", self.symbol, self.value)
    }
}

/// Configuration for Surge streaming client
#[derive(Debug, Clone)]
pub struct SurgeConfig {
    pub api_key: String,
    pub ws_url: String,
    pub api_url: String,
    pub auto_reconnect: bool,
    pub max_reconnect_attempts: u32,
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
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub data: SurgeUpdateData,
}

/// Price update data payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeUpdateData {
    pub symbol: String,
    pub price: f64,
    #[serde(rename = "source_ts_ms")]
    pub source_timestamp_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feed_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Events emitted by the Surge streaming client
#[derive(Debug, Clone)]
pub enum SurgeEvent {
    Connected,
    Disconnected,
    PriceUpdate(SurgeUpdate),
    Error(String),
    Reconnecting { attempt: u32, delay_ms: u64 },
}

/// Request to subscribe/unsubscribe to symbols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionRequest {
    pub action: String,
    pub symbols: Vec<SymbolRequest>,
}

/// Symbol subscription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolRequest {
    pub symbol: String,
}

/// Information about a Surge feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurgeFeedInfo {
    pub symbol: String,
    #[serde(rename = "feedId")]
    pub feed_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // === FeedPrice tests ===

    #[test]
    fn test_feed_price_display() {
        let price = FeedPrice {
            symbol: "BTC/USD".to_string(),
            feed_id: "abc123".to_string(),
            value: 50000.123456,
        };
        assert_eq!(format!("{}", price), "BTC/USD: $50000.123456");
    }

    #[test]
    fn test_feed_price_display_small_value() {
        let price = FeedPrice {
            symbol: "SHIB/USD".to_string(),
            feed_id: "abc123".to_string(),
            value: 0.000012,
        };
        assert_eq!(format!("{}", price), "SHIB/USD: $0.000012");
    }

    #[test]
    fn test_feed_price_serialization() {
        let price = FeedPrice {
            symbol: "ETH/USD".to_string(),
            feed_id: "def456".to_string(),
            value: 3000.50,
        };
        let json = serde_json::to_string(&price).unwrap();
        assert!(json.contains("\"symbol\":\"ETH/USD\""));
        assert!(json.contains("\"feed_id\":\"def456\""));
        assert!(json.contains("\"value\":3000.5"));
    }

    #[test]
    fn test_feed_price_deserialization() {
        let json = r#"{"symbol":"SOL/USD","feed_id":"xyz789","value":150.25}"#;
        let price: FeedPrice = serde_json::from_str(json).unwrap();
        assert_eq!(price.symbol, "SOL/USD");
        assert_eq!(price.feed_id, "xyz789");
        assert_eq!(price.value, 150.25);
    }

    #[test]
    fn test_feed_price_clone() {
        let price = FeedPrice {
            symbol: "BTC/USD".to_string(),
            feed_id: "abc".to_string(),
            value: 50000.0,
        };
        let cloned = price.clone();
        assert_eq!(price.symbol, cloned.symbol);
        assert_eq!(price.value, cloned.value);
    }

    // === SurgeConfig tests ===

    #[test]
    fn test_surge_config_default() {
        let config = SurgeConfig::default();
        assert_eq!(config.ws_url, "wss://surge.switchboard.xyz/ws");
        assert_eq!(config.api_url, "https://surge.switchboard.xyz");
        assert!(config.auto_reconnect);
        assert_eq!(config.max_reconnect_attempts, 10);
        assert_eq!(config.initial_reconnect_delay_ms, 1000);
        assert!(config.api_key.is_empty());
    }

    // === SurgeUpdate tests ===

    #[test]
    fn test_surge_update_deserialization() {
        let json = r#"{
            "type": "price",
            "data": {
                "symbol": "BTC/USD",
                "price": 50000.0,
                "source_ts_ms": 1234567890
            }
        }"#;
        let update: SurgeUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.event_type, Some("price".to_string()));
        assert_eq!(update.data.symbol, "BTC/USD");
        assert_eq!(update.data.price, 50000.0);
        assert_eq!(update.data.source_timestamp_ms, 1234567890);
    }

    #[test]
    fn test_surge_update_with_optional_fields() {
        let json = r#"{
            "type": "price",
            "data": {
                "symbol": "ETH/USD",
                "price": 3000.0,
                "source_ts_ms": 1234567890,
                "feed_id": "abc123",
                "signature": "sig456"
            }
        }"#;
        let update: SurgeUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.data.feed_id, Some("abc123".to_string()));
        assert_eq!(update.data.signature, Some("sig456".to_string()));
    }

    #[test]
    fn test_surge_update_without_type() {
        let json = r#"{
            "data": {
                "symbol": "SOL/USD",
                "price": 150.0,
                "source_ts_ms": 1234567890
            }
        }"#;
        let update: SurgeUpdate = serde_json::from_str(json).unwrap();
        assert_eq!(update.event_type, None);
        assert_eq!(update.data.symbol, "SOL/USD");
    }

    // === SubscriptionRequest tests ===

    #[test]
    fn test_subscription_request_serialization() {
        let req = SubscriptionRequest {
            action: "subscribe".to_string(),
            symbols: vec![SymbolRequest { symbol: "BTC/USD".to_string() }],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"action\":\"subscribe\""));
        assert!(json.contains("\"symbol\":\"BTC/USD\""));
    }

    #[test]
    fn test_subscription_request_multiple_symbols() {
        let req = SubscriptionRequest {
            action: "subscribe".to_string(),
            symbols: vec![
                SymbolRequest { symbol: "BTC/USD".to_string() },
                SymbolRequest { symbol: "ETH/USD".to_string() },
                SymbolRequest { symbol: "SOL/USD".to_string() },
            ],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("BTC/USD"));
        assert!(json.contains("ETH/USD"));
        assert!(json.contains("SOL/USD"));
    }

    #[test]
    fn test_unsubscribe_request() {
        let req = SubscriptionRequest {
            action: "unsubscribe".to_string(),
            symbols: vec![SymbolRequest { symbol: "BTC/USD".to_string() }],
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"action\":\"unsubscribe\""));
    }

    // === SurgeFeedInfo tests ===

    #[test]
    fn test_surge_feed_info_deserialization() {
        let json = r#"{"symbol":"BTC/USD","feedId":"abc123"}"#;
        let info: SurgeFeedInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.symbol, "BTC/USD");
        assert_eq!(info.feed_id, Some("abc123".to_string()));
    }

    #[test]
    fn test_surge_feed_info_without_feed_id() {
        let json = r#"{"symbol":"NEW/COIN"}"#;
        let info: SurgeFeedInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.symbol, "NEW/COIN");
        assert_eq!(info.feed_id, None);
    }

    // === SurgeEvent tests ===

    #[test]
    fn test_surge_event_debug() {
        let event = SurgeEvent::Connected;
        assert_eq!(format!("{:?}", event), "Connected");

        let event = SurgeEvent::Error("test error".to_string());
        assert!(format!("{:?}", event).contains("test error"));
    }

    #[test]
    fn test_surge_event_reconnecting() {
        let event = SurgeEvent::Reconnecting { attempt: 3, delay_ms: 5000 };
        let debug = format!("{:?}", event);
        assert!(debug.contains("3"));
        assert!(debug.contains("5000"));
    }
}
