//! WebSocket streaming client for Switchboard Surge
//!
//! Provides real-time price updates with sub-100ms latency through persistent WebSocket connections.

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::error::{Result, SurgeError};
use crate::types::{SurgeConfig, SurgeEvent, SurgeUpdate, SubscriptionRequest};

/// Default Surge WebSocket gateway URL
pub const DEFAULT_SURGE_WS_URL: &str = "wss://surge.switchboard.xyz/ws";

/// Default Surge REST API URL
pub const DEFAULT_SURGE_API_URL: &str = "https://surge.switchboard.xyz";

/// Surge streaming client for real-time price updates
pub struct Surge {
    config: SurgeConfig,
    event_tx: broadcast::Sender<SurgeEvent>,
    control_tx: Option<mpsc::Sender<ControlMessage>>,
    is_connected: Arc<RwLock<bool>>,
    subscriptions: Arc<RwLock<Vec<String>>>,
}

#[derive(Debug)]
enum ControlMessage {
    Subscribe(Vec<String>),
    Unsubscribe(Vec<String>),
    Disconnect,
}

impl Surge {
    /// Create a new Surge client with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            config: SurgeConfig {
                api_key: api_key.into(),
                ws_url: DEFAULT_SURGE_WS_URL.to_string(),
                api_url: DEFAULT_SURGE_API_URL.to_string(),
                auto_reconnect: true,
                max_reconnect_attempts: 10,
                initial_reconnect_delay_ms: 1000,
            },
            event_tx,
            control_tx: None,
            is_connected: Arc::new(RwLock::new(false)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a new Surge client with custom configuration
    pub fn with_config(config: SurgeConfig) -> Self {
        let (event_tx, _) = broadcast::channel(1000);

        Self {
            config,
            event_tx,
            control_tx: None,
            is_connected: Arc::new(RwLock::new(false)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Configure WebSocket URL
    pub fn ws_url(mut self, url: impl Into<String>) -> Self {
        self.config.ws_url = url.into();
        self
    }

    /// Configure auto-reconnection
    pub fn auto_reconnect(mut self, enabled: bool) -> Self {
        self.config.auto_reconnect = enabled;
        self
    }

    /// Get a receiver for Surge events
    pub fn subscribe_events(&self) -> broadcast::Receiver<SurgeEvent> {
        self.event_tx.subscribe()
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.is_connected.read().await
    }

    /// Get currently subscribed symbols
    pub async fn get_subscriptions(&self) -> Vec<String> {
        self.subscriptions.read().await.clone()
    }

    /// Connect to Surge and subscribe to symbols
    pub async fn connect_and_subscribe(&mut self, symbols: Vec<&str>) -> Result<()> {
        let symbols: Vec<String> = symbols.into_iter().map(|s| s.to_string()).collect();

        // Store subscriptions
        {
            let mut subs = self.subscriptions.write().await;
            *subs = symbols.clone();
        }

        // Create control channel
        let (control_tx, control_rx) = mpsc::channel(100);
        self.control_tx = Some(control_tx);

        // Clone what we need for the spawned task
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let is_connected = self.is_connected.clone();
        let subscriptions = self.subscriptions.clone();

        // Spawn connection task
        tokio::spawn(async move {
            connection_loop(
                config,
                symbols,
                event_tx,
                control_rx,
                is_connected,
                subscriptions,
            )
            .await;
        });

        // Wait briefly for connection
        sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Subscribe to additional symbols
    pub async fn subscribe(&self, symbols: Vec<&str>) -> Result<()> {
        let symbols: Vec<String> = symbols.into_iter().map(|s| s.to_string()).collect();

        if let Some(tx) = &self.control_tx {
            tx.send(ControlMessage::Subscribe(symbols))
                .await
                .map_err(|e| SurgeError::ApiError(format!("Failed to send subscribe: {}", e)))?;
        }

        Ok(())
    }

    /// Unsubscribe from symbols
    pub async fn unsubscribe(&self, symbols: Vec<&str>) -> Result<()> {
        let symbols: Vec<String> = symbols.into_iter().map(|s| s.to_string()).collect();

        if let Some(tx) = &self.control_tx {
            tx.send(ControlMessage::Unsubscribe(symbols))
                .await
                .map_err(|e| SurgeError::ApiError(format!("Failed to send unsubscribe: {}", e)))?;
        }

        Ok(())
    }

    /// Disconnect from Surge
    pub async fn disconnect(&self) -> Result<()> {
        if let Some(tx) = &self.control_tx {
            let _ = tx.send(ControlMessage::Disconnect).await;
        }

        *self.is_connected.write().await = false;
        Ok(())
    }

    /// Fetch available feeds from the Surge API
    pub async fn get_surge_feeds(&self) -> Result<Vec<crate::types::SurgeFeedInfo>> {
        let client = reqwest::Client::new();
        let url = format!("{}/feeds", self.config.api_url);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SurgeError::ApiError(format!(
                "Failed to fetch feeds: {}",
                response.status()
            )));
        }

        let feeds: Vec<crate::types::SurgeFeedInfo> = response.json().await?;
        Ok(feeds)
    }
}

/// Main connection loop with auto-reconnection
async fn connection_loop(
    config: SurgeConfig,
    _initial_symbols: Vec<String>,
    event_tx: broadcast::Sender<SurgeEvent>,
    mut control_rx: mpsc::Receiver<ControlMessage>,
    is_connected: Arc<RwLock<bool>>,
    subscriptions: Arc<RwLock<Vec<String>>>,
) {
    let mut reconnect_attempts = 0;
    let mut current_delay = config.initial_reconnect_delay_ms;

    loop {
        // Attempt connection
        let ws_url = format!(
            "{}?apiKey={}",
            config.ws_url, config.api_key
        );

        let url = match Url::parse(&ws_url) {
            Ok(u) => u,
            Err(e) => {
                let _ = event_tx.send(SurgeEvent::Error(format!("Invalid URL: {}", e)));
                return;
            }
        };

        match connect_async(url).await {
            Ok((ws_stream, _)) => {
                reconnect_attempts = 0;
                current_delay = config.initial_reconnect_delay_ms;

                *is_connected.write().await = true;
                let _ = event_tx.send(SurgeEvent::Connected);

                let (mut write, mut read) = ws_stream.split();

                // Subscribe to initial symbols
                let subs = subscriptions.read().await.clone();
                if !subs.is_empty() {
                    let subscribe_msg = SubscriptionRequest {
                        action: "subscribe".to_string(),
                        symbols: subs.iter().map(|s| crate::types::SymbolRequest { symbol: s.clone() }).collect(),
                    };

                    if let Ok(json) = serde_json::to_string(&subscribe_msg) {
                        let _ = write.send(Message::Text(json)).await;
                    }
                }

                // Handle messages
                loop {
                    tokio::select! {
                        // Handle incoming WebSocket messages
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(update) = serde_json::from_str::<SurgeUpdate>(&text) {
                                        let _ = event_tx.send(SurgeEvent::PriceUpdate(update));
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    let _ = event_tx.send(SurgeEvent::Disconnected);
                                    *is_connected.write().await = false;
                                    break;
                                }
                                Some(Err(e)) => {
                                    let _ = event_tx.send(SurgeEvent::Error(e.to_string()));
                                    *is_connected.write().await = false;
                                    break;
                                }
                                None => {
                                    *is_connected.write().await = false;
                                    break;
                                }
                                _ => {}
                            }
                        }

                        // Handle control messages
                        ctrl = control_rx.recv() => {
                            match ctrl {
                                Some(ControlMessage::Subscribe(symbols)) => {
                                    let mut subs = subscriptions.write().await;
                                    for s in &symbols {
                                        if !subs.contains(s) {
                                            subs.push(s.clone());
                                        }
                                    }

                                    let subscribe_msg = SubscriptionRequest {
                                        action: "subscribe".to_string(),
                                        symbols: symbols.iter().map(|s| crate::types::SymbolRequest { symbol: s.clone() }).collect(),
                                    };

                                    if let Ok(json) = serde_json::to_string(&subscribe_msg) {
                                        let _ = write.send(Message::Text(json)).await;
                                    }
                                }
                                Some(ControlMessage::Unsubscribe(symbols)) => {
                                    let mut subs = subscriptions.write().await;
                                    subs.retain(|s| !symbols.contains(s));

                                    let unsubscribe_msg = SubscriptionRequest {
                                        action: "unsubscribe".to_string(),
                                        symbols: symbols.iter().map(|s| crate::types::SymbolRequest { symbol: s.clone() }).collect(),
                                    };

                                    if let Ok(json) = serde_json::to_string(&unsubscribe_msg) {
                                        let _ = write.send(Message::Text(json)).await;
                                    }
                                }
                                Some(ControlMessage::Disconnect) | None => {
                                    let _ = write.send(Message::Close(None)).await;
                                    *is_connected.write().await = false;
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = event_tx.send(SurgeEvent::Error(format!("Connection failed: {}", e)));
            }
        }

        // Check if we should reconnect
        if !config.auto_reconnect || reconnect_attempts >= config.max_reconnect_attempts {
            let _ = event_tx.send(SurgeEvent::Error("Max reconnection attempts reached".to_string()));
            return;
        }

        // Exponential backoff
        let _ = event_tx.send(SurgeEvent::Reconnecting {
            attempt: reconnect_attempts + 1,
            delay_ms: current_delay,
        });

        sleep(Duration::from_millis(current_delay)).await;
        reconnect_attempts += 1;
        current_delay = (current_delay * 2).min(30000); // Cap at 30 seconds
    }
}
