//! WebSocket handler for real-time price streaming

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::metrics::{ws_connection_closed, ws_connection_opened};
use super::routes::AppState;
use crate::{Surge, SurgeEvent};

/// Client message for WebSocket subscription
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum ClientMessage {
    Subscribe { symbols: Vec<String> },
    Unsubscribe { symbols: Vec<String> },
}

/// Server message for WebSocket responses
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ServerMessage {
    Price {
        symbol: String,
        price: f64,
        timestamp: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        feed_id: Option<String>,
    },
    Subscribed { symbols: Vec<String> },
    Unsubscribed { symbols: Vec<String> },
    Error { message: String },
}

/// WebSocket upgrade handler
/// WS /v1/stream
pub async fn ws_handler(ws: WebSocketUpgrade, State(_state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

/// Handle an individual WebSocket connection
async fn handle_socket(socket: WebSocket) {
    ws_connection_opened();
    tracing::info!("WebSocket connection established");

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(100);

    // Spawn task to send messages to the client
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    let surge: Arc<RwLock<Option<Surge>>> = Arc::new(RwLock::new(None));
    let subscribed_symbols: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

    // Task to relay upstream price updates
    let tx_relay = tx.clone();
    let surge_relay = surge.clone();
    let relay_task = tokio::spawn(async move {
        loop {
            let event_rx = {
                let guard = surge_relay.read().await;
                guard.as_ref().map(|s| s.subscribe_events())
            };

            if let Some(mut rx) = event_rx {
                match rx.recv().await {
                    Ok(SurgeEvent::PriceUpdate(update)) => {
                        let msg = ServerMessage::Price {
                            symbol: update.data.symbol,
                            price: update.data.price,
                            timestamp: update.data.source_timestamp_ms,
                            feed_id: update.data.feed_id,
                        };
                        if tx_relay.send(msg).await.is_err() {
                            break;
                        }
                    }
                    Ok(SurgeEvent::Error(e)) => {
                        let _ = tx_relay.send(ServerMessage::Error { message: e }).await;
                    }
                    Ok(_) => {}
                    Err(_) => tokio::time::sleep(tokio::time::Duration::from_millis(100)).await,
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });

    // Handle incoming client messages
    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::Subscribe { symbols }) => {
                    tracing::info!("Client subscribing to: {:?}", symbols);
                    {
                        let mut subs = subscribed_symbols.write().await;
                        subs.extend(symbols.clone());
                    }
                    reconnect_surge(&surge, &subscribed_symbols, &tx).await;
                    let _ = tx.send(ServerMessage::Subscribed { symbols }).await;
                }
                Ok(ClientMessage::Unsubscribe { symbols }) => {
                    tracing::info!("Client unsubscribing from: {:?}", symbols);
                    {
                        let mut subs = subscribed_symbols.write().await;
                        for sym in &symbols {
                            subs.remove(sym);
                        }
                    }
                    reconnect_surge(&surge, &subscribed_symbols, &tx).await;
                    let _ = tx.send(ServerMessage::Unsubscribed { symbols }).await;
                }
                Err(e) => {
                    let _ = tx.send(ServerMessage::Error { message: format!("Invalid message: {}", e) }).await;
                }
            },
            Ok(Message::Close(_)) => {
                tracing::info!("Client sent close frame");
                break;
            }
            Err(e) => {
                tracing::warn!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    relay_task.abort();
    send_task.abort();
    if let Some(s) = surge.write().await.take() {
        let _ = s.disconnect().await;
    }

    ws_connection_closed();
    tracing::info!("WebSocket connection closed");
}

async fn reconnect_surge(
    surge: &Arc<RwLock<Option<Surge>>>,
    subscribed_symbols: &Arc<RwLock<HashSet<String>>>,
    tx: &mpsc::Sender<ServerMessage>,
) {
    // Disconnect existing connection
    if let Some(old_surge) = surge.write().await.take() {
        let _ = old_surge.disconnect().await;
    }

    let symbols: Vec<String> = {
        let current_subs = subscribed_symbols.read().await;
        if current_subs.is_empty() {
            return;
        }
        current_subs.iter().cloned().collect()
    };

    let symbol_refs: Vec<&str> = symbols.iter().map(String::as_str).collect();
    let mut new_surge = Surge::new("");
    match new_surge.connect_and_subscribe(symbol_refs).await {
        Ok(()) => *surge.write().await = Some(new_surge),
        Err(e) => {
            let _ = tx.send(ServerMessage::Error { message: e.to_string() }).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === ClientMessage tests ===

    #[test]
    fn test_client_message_subscribe_deserialization() {
        let json = r#"{"action": "subscribe", "symbols": ["BTC/USD", "ETH/USD"]}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        match msg {
            ClientMessage::Subscribe { symbols } => {
                assert_eq!(symbols.len(), 2);
                assert!(symbols.contains(&"BTC/USD".to_string()));
                assert!(symbols.contains(&"ETH/USD".to_string()));
            }
            _ => panic!("Expected Subscribe variant"),
        }
    }

    #[test]
    fn test_client_message_unsubscribe_deserialization() {
        let json = r#"{"action": "unsubscribe", "symbols": ["BTC/USD"]}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        match msg {
            ClientMessage::Unsubscribe { symbols } => {
                assert_eq!(symbols.len(), 1);
                assert_eq!(symbols[0], "BTC/USD");
            }
            _ => panic!("Expected Unsubscribe variant"),
        }
    }

    #[test]
    fn test_client_message_subscribe_empty_symbols() {
        let json = r#"{"action": "subscribe", "symbols": []}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        match msg {
            ClientMessage::Subscribe { symbols } => {
                assert!(symbols.is_empty());
            }
            _ => panic!("Expected Subscribe variant"),
        }
    }

    #[test]
    fn test_client_message_invalid_action() {
        let json = r#"{"action": "invalid", "symbols": ["BTC/USD"]}"#;
        let result = serde_json::from_str::<ClientMessage>(json);
        assert!(result.is_err(), "Should fail for invalid action");
    }

    #[test]
    fn test_client_message_missing_symbols() {
        let json = r#"{"action": "subscribe"}"#;
        let result = serde_json::from_str::<ClientMessage>(json);
        assert!(result.is_err(), "Should fail without symbols field");
    }

    // === ServerMessage tests ===

    #[test]
    fn test_server_message_price_serialization() {
        let msg = ServerMessage::Price {
            symbol: "BTC/USD".to_string(),
            price: 89846.94,
            timestamp: 1705936800000,
            feed_id: Some("abc123".to_string()),
        };
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains(r#""type":"price""#));
        assert!(json.contains(r#""symbol":"BTC/USD""#));
        assert!(json.contains(r#""price":89846.94"#));
        assert!(json.contains(r#""timestamp":1705936800000"#));
        assert!(json.contains(r#""feed_id":"abc123""#));
    }

    #[test]
    fn test_server_message_price_without_feed_id() {
        let msg = ServerMessage::Price {
            symbol: "ETH/USD".to_string(),
            price: 3245.50,
            timestamp: 1705936800000,
            feed_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains(r#""type":"price""#));
        assert!(!json.contains("feed_id"), "feed_id should be omitted when None");
    }

    #[test]
    fn test_server_message_subscribed_serialization() {
        let msg = ServerMessage::Subscribed {
            symbols: vec!["BTC/USD".to_string(), "ETH/USD".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains(r#""type":"subscribed""#));
        assert!(json.contains("BTC/USD"));
        assert!(json.contains("ETH/USD"));
    }

    #[test]
    fn test_server_message_unsubscribed_serialization() {
        let msg = ServerMessage::Unsubscribed {
            symbols: vec!["BTC/USD".to_string()],
        };
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains(r#""type":"unsubscribed""#));
        assert!(json.contains("BTC/USD"));
    }

    #[test]
    fn test_server_message_error_serialization() {
        let msg = ServerMessage::Error {
            message: "Connection failed".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();

        assert!(json.contains(r#""type":"error""#));
        assert!(json.contains(r#""message":"Connection failed""#));
    }

    // === Message roundtrip tests ===

    #[test]
    fn test_server_message_json_roundtrip() {
        let original = ServerMessage::Price {
            symbol: "SOL/USD".to_string(),
            price: 148.25,
            timestamp: 1705936800000,
            feed_id: Some("xyz789".to_string()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["type"], "price");
        assert_eq!(parsed["symbol"], "SOL/USD");
        assert_eq!(parsed["price"], 148.25);
        assert_eq!(parsed["timestamp"], 1705936800000_i64);
        assert_eq!(parsed["feed_id"], "xyz789");
    }

    // === Subscription management tests ===

    #[test]
    fn test_add_unique_symbols_with_hashset() {
        let mut symbols: HashSet<String> = HashSet::new();
        symbols.insert("BTC/USD".to_string());

        // Adding duplicates should be a no-op
        symbols.insert("BTC/USD".to_string());
        symbols.insert("ETH/USD".to_string());

        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains("BTC/USD"));
        assert!(symbols.contains("ETH/USD"));
    }

    #[test]
    fn test_remove_symbols_from_hashset() {
        let mut symbols: HashSet<String> = HashSet::new();
        symbols.insert("BTC/USD".to_string());
        symbols.insert("ETH/USD".to_string());
        symbols.insert("SOL/USD".to_string());

        symbols.remove("ETH/USD");

        assert_eq!(symbols.len(), 2);
        assert!(symbols.contains("BTC/USD"));
        assert!(symbols.contains("SOL/USD"));
        assert!(!symbols.contains("ETH/USD"));
    }
}
