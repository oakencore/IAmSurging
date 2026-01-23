use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

use crate::error::{Result, SurgeError};
use crate::types::{SurgeConfig, SurgeEvent, SurgeFeedInfo, SurgeUpdate, SubscriptionRequest, SymbolRequest};

/// Surge streaming client for real-time price updates
pub struct Surge {
    config: SurgeConfig,
    event_tx: broadcast::Sender<SurgeEvent>,
    control_tx: Option<mpsc::Sender<ControlMessage>>,
    is_connected: Arc<RwLock<bool>>,
    subscriptions: Arc<RwLock<Vec<String>>>,
}

enum ControlMessage {
    Disconnect,
}

impl Surge {
    pub fn new(api_key: impl Into<String>) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        let config = SurgeConfig {
            api_key: api_key.into(),
            ..SurgeConfig::default()
        };
        Self {
            config,
            event_tx,
            control_tx: None,
            is_connected: Arc::new(RwLock::new(false)),
            subscriptions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<SurgeEvent> {
        self.event_tx.subscribe()
    }

    pub async fn connect_and_subscribe(&mut self, symbols: Vec<&str>) -> Result<()> {
        let symbols: Vec<String> = symbols.iter().map(|&s| s.to_owned()).collect();
        *self.subscriptions.write().await = symbols;

        let (control_tx, control_rx) = mpsc::channel(100);
        self.control_tx = Some(control_tx);

        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let is_connected = self.is_connected.clone();
        let subscriptions = self.subscriptions.clone();

        tokio::spawn(async move {
            connection_loop(config, event_tx, control_rx, is_connected, subscriptions).await;
        });

        sleep(Duration::from_millis(100)).await;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        if let Some(tx) = &self.control_tx {
            let _ = tx.send(ControlMessage::Disconnect).await;
        }
        *self.is_connected.write().await = false;
        Ok(())
    }

    pub async fn get_surge_feeds(&self) -> Result<Vec<SurgeFeedInfo>> {
        let client = reqwest::Client::new();
        let url = format!("{}/feeds", self.config.api_url);
        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(SurgeError::ApiError(format!("Failed to fetch feeds: {}", response.status())));
        }
        Ok(response.json().await?)
    }
}

async fn connection_loop(
    config: SurgeConfig,
    event_tx: broadcast::Sender<SurgeEvent>,
    mut control_rx: mpsc::Receiver<ControlMessage>,
    is_connected: Arc<RwLock<bool>>,
    subscriptions: Arc<RwLock<Vec<String>>>,
) {
    let mut reconnect_attempts = 0;
    let mut delay = config.initial_reconnect_delay_ms;

    loop {
        let ws_url = format!("{}?apiKey={}", config.ws_url, config.api_key);
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
                delay = config.initial_reconnect_delay_ms;
                *is_connected.write().await = true;
                let _ = event_tx.send(SurgeEvent::Connected);

                let (mut write, mut read) = ws_stream.split();

                // Subscribe to symbols
                let subscription_msg = {
                    let current_subs = subscriptions.read().await;
                    if current_subs.is_empty() {
                        None
                    } else {
                        Some(SubscriptionRequest {
                            action: "subscribe".to_string(),
                            symbols: current_subs.iter().map(|s| SymbolRequest { symbol: s.clone() }).collect(),
                        })
                    }
                };
                if let Some(msg) = subscription_msg {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        let _ = write.send(Message::Text(json)).await;
                    }
                }

                loop {
                    tokio::select! {
                        msg = read.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(update) = serde_json::from_str::<SurgeUpdate>(&text) {
                                        let _ = event_tx.send(SurgeEvent::PriceUpdate(update));
                                    }
                                }
                                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                                    let _ = event_tx.send(SurgeEvent::Disconnected);
                                    *is_connected.write().await = false;
                                    break;
                                }
                                _ => {}
                            }
                        }
                        ctrl = control_rx.recv() => {
                            if matches!(ctrl, Some(ControlMessage::Disconnect) | None) {
                                let _ = write.send(Message::Close(None)).await;
                                *is_connected.write().await = false;
                                return;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = event_tx.send(SurgeEvent::Error(format!("Connection failed: {}", e)));
            }
        }

        if !config.auto_reconnect || reconnect_attempts >= config.max_reconnect_attempts {
            let _ = event_tx.send(SurgeEvent::Error("Max reconnection attempts reached".to_string()));
            return;
        }

        let _ = event_tx.send(SurgeEvent::Reconnecting { attempt: reconnect_attempts + 1, delay_ms: delay });
        sleep(Duration::from_millis(delay)).await;
        reconnect_attempts += 1;
        delay = (delay * 2).min(30000);
    }
}
