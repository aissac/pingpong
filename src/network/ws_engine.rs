//! WebSocket Engine - Async Polymarket WebSocket with WAF Bypass
//!
//! Implements:
//! - tokio-tungstenite async WebSocket
//! - WAF bypass headers (User-Agent, Origin)
//! - Market channel subscription
//! - Bridge to crossbeam for hot path
//! - Auto-reconnect with exponential backoff

use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio_tungstenite::tungstenite::handshake::client::Request;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crossbeam_channel::Sender;
use anyhow::{Result, Context, anyhow};
use serde_json::Value;

const WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
const RECONNECT_DELAY_SECS: u64 = 5;
const MAX_RECONNECT_DELAY_SECS: u64 = 60;
const PING_INTERVAL_SECS: u64 = 30;

/// WebSocket event types sent to hot path
#[derive(Debug, Clone)]
pub enum WsEvent {
    /// Orderbook update
    BookUpdate {
        token_hash: u64,
        token_id: String,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64,
        timestamp_nanos: u64,
    },
    /// Trade occurred
    Trade {
        token_hash: u64,
        token_id: String,
        price: u64,
        size: u64,
        side: TradeSide,
        timestamp_nanos: u64,
    },
    /// Connection status
    Connected,
    Disconnected { reason: String },
    /// Order matched (for user orders)
    OrderMatched {
        order_id: String,
        token_id: String,
        matched_size: u64,
        price: u64,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum TradeSide {
    Buy,
    Sell,
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// WebSocket Engine configuration
#[derive(Debug, Clone)]
pub struct WsConfig {
    /// WebSocket URL (defaults to Polymarket)
    pub url: String,
    /// User-Agent header for WAF bypass
    pub user_agent: String,
    /// Origin header for WAF bypass
    pub origin: String,
    /// Ping interval in seconds
    pub ping_interval_secs: u64,
    /// Reconnect delay in seconds
    pub reconnect_delay_secs: u64,
    /// Maximum reconnect delay
    pub max_reconnect_delay_secs: u64,
}

impl Default for WsConfig {
    fn default() -> Self {
        Self {
            url: WS_URL.to_string(),
            // Browser-like User-Agent for WAF bypass
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36".to_string(),
            // Polymarket origin for WAF bypass
            origin: "https://polymarket.com".to_string(),
            ping_interval_secs: PING_INTERVAL_SECS,
            reconnect_delay_secs: RECONNECT_DELAY_SECS,
            max_reconnect_delay_secs: MAX_RECONNECT_DELAY_SECS,
        }
    }
}

/// WebSocket Engine
/// 
/// Manages connection to Polymarket WebSocket and forwards
/// events to hot path via crossbeam channel
pub struct WsEngine {
    config: WsConfig,
    state: Arc<RwLock<ConnectionState>>,
    /// Token IDs to subscribe
    tokens: Arc<RwLock<Vec<String>>>,
    /// Event channel to hot path
    event_tx: Sender<WsEvent>,
    /// Last message timestamp (for health check)
    last_message: Arc<RwLock<Instant>>,
    /// Message counter for monitoring
    message_count: Arc<RwLock<u64>>,
}

impl WsEngine {
    /// Create a new WebSocket engine
    pub fn new(event_tx: Sender<WsEvent>) -> Self {
        Self {
            config: WsConfig::default(),
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            tokens: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            last_message: Arc::new(RwLock::new(Instant::now())),
            message_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Create with custom configuration
    pub fn with_config(event_tx: Sender<WsEvent>, config: WsConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            tokens: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            last_message: Arc::new(RwLock::new(Instant::now())),
            message_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Set tokens to subscribe
    pub async fn set_tokens(&self, tokens: Vec<String>) {
        *self.tokens.write().await = tokens;
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Get message count (for monitoring)
    pub async fn message_count(&self) -> u64 {
        *self.message_count.read().await
    }

    /// Connect and run the WebSocket loop
    /// 
    /// This is the main entry point - spawns background tasks
    /// and handles auto-reconnect
    pub async fn run(&self) -> Result<()> {
        let mut reconnect_delay = self.config.reconnect_delay_secs;
        
        loop {
            // Update state
            *self.state.write().await = ConnectionState::Connecting;
            
            println!("[WS] 🔌 Connecting to {}...", self.config.url);
            
            // Attempt connection
            match self.connect_and_run().await {
                Ok(_) => {
                    // Connection closed normally
                    println!("[WS] Connection closed");
                    *self.state.write().await = ConnectionState::Disconnected;
                    
                    // Reset reconnect delay on clean disconnect
                    reconnect_delay = self.config.reconnect_delay_secs;
                }
                Err(e) => {
                    eprintln!("[WS] ❌ Connection error: {:?}", e);
                    *self.state.write().await = ConnectionState::Reconnecting;
                    
                    // Exponential backoff
                    println!("[WS] Reconnecting in {}s...", reconnect_delay);
                    tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                    
                    // Increase delay for next attempt (max 60s)
                    reconnect_delay = (reconnect_delay * 2).min(self.config.max_reconnect_delay_secs);
                }
            }
            
            // Send disconnect event
            let _ = self.event_tx.send(WsEvent::Disconnected {
                reason: "Connection lost".to_string(),
            });
        }
    }

    /// Single connection attempt
    async fn connect_and_run(&self) -> Result<()> {
        // Build request with WAF bypass headers
        let request = Request::builder()
            .uri(&self.config.url)
            .header("User-Agent", &self.config.user_agent)
            .header("Origin", &self.config.origin)
            .header("Host", "ws-subscriptions-clob.polymarket.com")
            .header("Sec-WebSocket-Version", "13")
            .header("Sec-WebSocket-Key", generate_ws_key())
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .body(())
            .context("Failed to build WebSocket request")?;

        // Connect
        let (ws_stream, _) = connect_async(request)
            .await
            .context("Failed to connect to WebSocket")?;

        println!("[WS] ✅ Connected to Polymarket WebSocket");
        *self.state.write().await = ConnectionState::Connected;
        
        // Send connected event
        let _ = self.event_tx.send(WsEvent::Connected);

        // Split into sender/receiver
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Subscribe to tokens
        let tokens = self.tokens.read().await.clone();
        self.subscribe(&mut ws_sender, &tokens).await?;

        // Update last message time
        *self.last_message.write().await = Instant::now();

        // Update last message time
        *self.last_message.write().await = Instant::now();

        // Message receive loop
        loop {
            tokio::select! {
                msg = ws_receiver.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            *self.last_message.write().await = Instant::now();
                            *self.message_count.write().await += 1;
                            
                            // Parse and process message
                            if let Err(e) = self.process_message(&text).await {
                                eprintln!("[WS] ⚠️ Message parse error: {:?}", e);
                            }
                        }
                        Some(Ok(Message::Binary(data))) => {
                            *self.last_message.write().await = Instant::now();
                            *self.message_count.write().await += 1;
                            
                            // Try to parse as UTF-8 text
                            if let Ok(text) = std::str::from_utf8(&data) {
                                if let Err(e) = self.process_message(text).await {
                                    eprintln!("[WS] ⚠️ Binary message parse error: {:?}", e);
                                }
                            }
                        }
                        Some(Ok(Message::Ping(data))) => {
                            // Respond with pong
                            let _ = ws_sender.send(Message::Pong(data)).await;
                        }
                        Some(Ok(Message::Pong(_))) => {
                            // Pong received - connection alive
                        }
                        Some(Ok(Message::Close(_))) => {
                            println!("[WS] Close frame received");
                            break;
                        }
                        Some(Err(e)) => {
                            eprintln!("[WS] ❌ WebSocket error: {:?}", e);
                            break;
                        }
                        None => {
                            println!("[WS] Stream ended");
                            break;
                        }
                        _ => {}
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(PING_INTERVAL_SECS)) => {
                    // Send ping to keep connection alive
                    let _ = ws_sender.send(Message::Ping(vec![])).await;
                }
            }
        }

        Ok(())
    }

    /// Subscribe to token channels
    async fn subscribe(&self, sender: &mut futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>, tokens: &[String]) -> Result<()> {
        let subscribe_msg = serde_json::json!({
            "type": "market",
            "operation": "subscribe",
            "markets": [],
            "assets_ids": tokens,
            "initial_dump": true
        });

        let msg_str = serde_json::to_string(&subscribe_msg)?;
        sender.send(Message::Text(msg_str.into())).await?;
        
        println!("[WS] 📡 Subscribed to {} tokens", tokens.len());
        Ok(())
    }

    /// Process incoming WebSocket message
    async fn process_message(&self, text: &str) -> Result<()> {
        // Fast path: Check message type with memchr-style scan
        // Avoids full JSON parsing for hot path
        
        // Quick check for orderbook update
        if text.contains("\"event_type\":\"book\"") || text.contains("\"event\":\"book\"") {
            return self.process_book_update(text).await;
        }
        
        // Check for trade
        if text.contains("\"event_type\":\"trade\"") || text.contains("\"event\":\"trade\"") {
            return self.process_trade(text).await;
        }
        
        // Check for order matched (user orders)
        if text.contains("\"event_type\":\"matched\"") || text.contains("\"event\":\"matched\"") {
            return self.process_order_matched(text).await;
        }
        
        // Initial dump or other message
        if text.contains("\"initial_dump\"") || text.contains("\"type\":\"book\"") {
            // Process initial orderbook dump
            return self.process_book_update(text).await;
        }
        
        Ok(())
    }

    /// Process orderbook update
    async fn process_book_update(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)?;
        
        // Extract token ID
        let token_id = value["asset_id"]
            .as_str()
            .or_else(|| value["token_id"].as_str())
            .or_else(|| value["assetId"].as_str())
            .context("No token ID in book update")?;
        
        // Hash token ID for hot path
        let token_hash = fast_hash(token_id.as_bytes());
        
        // Extract price levels
        let market = &value["market"];
        
        // Try to get bid/ask from different formats
        let (bid_price, bid_size, ask_price, ask_size) = {
            // Format 1: nested market object
            if let Some(bids) = market["bids"].as_array() {
                let bid = bids.first().context("No bids")?;
                let bid_price = bid["price"].as_str()
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let bid_size = bid["size"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                    
                let asks = market["asks"].as_array().context("No asks")?;
                let ask = asks.first().context("No ask")?;
                let ask_price = ask["price"].as_str()
                    .and_then(|p| p.parse::<f64>().ok())
                    .unwrap_or(0.0);
                let ask_size = ask["size"].as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0);
                
                // Convert to micro-USDC (fixed-point)
                (
                    (bid_price * 1_000_000.0) as u64,
                    (bid_size * 1_000_000.0) as u64,
                    (ask_price * 1_000_000.0) as u64,
                    (ask_size * 1_000_000.0) as u64,
                )
            } else {
                // Format 2: direct price/size fields
                let bid_price = value["bid"].as_f64()
                    .or_else(|| value["bestBid"].as_f64())
                    .unwrap_or(0.0);
                let bid_size = value["bidSize"].as_f64()
                    .or_else(|| value["bid_vol"].as_f64())
                    .unwrap_or(0.0);
                let ask_price = value["ask"].as_f64()
                    .or_else(|| value["bestAsk"].as_f64())
                    .unwrap_or(0.0);
                let ask_size = value["askSize"].as_f64()
                    .or_else(|| value["ask_vol"].as_f64())
                    .unwrap_or(0.0);
                
                (
                    (bid_price * 1_000_000.0) as u64,
                    (bid_size * 1_000_000.0) as u64,
                    (ask_price * 1_000_000.0) as u64,
                    (ask_size * 1_000_000.0) as u64,
                )
            }
        };

        // Send to hot path
        let timestamp_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;
        
        let event = WsEvent::BookUpdate {
            token_hash,
            token_id: token_id.to_string(),
            bid_price,
            bid_size,
            ask_price,
            ask_size,
            timestamp_nanos,
        };

        let _ = self.event_tx.send(event);
        Ok(())
    }

    /// Process trade event
    async fn process_trade(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)?;
        
        let token_id = value["asset_id"]
            .as_str()
            .or_else(|| value["token_id"].as_str())
            .context("No token ID in trade")?;
        
        let token_hash = fast_hash(token_id.as_bytes());
        
        let price = value["price"]
            .as_str()
            .and_then(|p| p.parse::<f64>().ok())
            .unwrap_or(0.0);
        let size = value["size"]
            .as_str()
            .or_else(|| value["amount"].as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let side = if value["side"].as_str() == Some("BUY") || value["takerSide"].as_str() == Some("BUY") {
            TradeSide::Buy
        } else {
            TradeSide::Sell
        };

        let timestamp_nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let event = WsEvent::Trade {
            token_hash,
            token_id: token_id.to_string(),
            price: (price * 1_000_000.0) as u64,
            size: (size * 1_000_000.0) as u64,
            side,
            timestamp_nanos,
        };

        let _ = self.event_tx.send(event);
        Ok(())
    }

    /// Process order matched event (for user orders)
    async fn process_order_matched(&self, text: &str) -> Result<()> {
        let value: Value = serde_json::from_str(text)?;
        
        let order_id = value["order_id"]
            .as_str()
            .context("No order ID in matched event")?
            .to_string();
        let token_id = value["asset_id"]
            .as_str()
            .or_else(|| value["token_id"].as_str())
            .unwrap_or("")
            .to_string();
        let matched_size = value["matchedSize"]
            .as_str()
            .or_else(|| value["matched_size"].as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let price = value["price"]
            .as_str()
            .and_then(|p| p.parse::<f64>().ok())
            .unwrap_or(0.0);

        let event = WsEvent::OrderMatched {
            order_id,
            token_id,
            matched_size: (matched_size * 1_000_000.0) as u64,
            price: (price * 1_000_000.0) as u64,
        };

        let _ = self.event_tx.send(event);
        Ok(())
    }
}

/// Generate WebSocket key for handshake
fn generate_ws_key() -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let random_bytes: [u8; 16] = {
        let mut arr = [0u8; 16];
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        arr[..8].copy_from_slice(&timestamp.to_le_bytes());
        arr[8..].copy_from_slice(&(timestamp >> 8).to_le_bytes());
        arr
    };
    STANDARD.encode(&random_bytes)
}

/// Fast hash for token IDs (FNV-1a variant)
fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

/// Spawn WebSocket engine in background task
pub fn spawn_ws_engine(
    tokens: Vec<String>,
    event_tx: Sender<WsEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let engine = WsEngine::new(event_tx);
        engine.set_tokens(tokens).await;
        
        if let Err(e) = engine.run().await {
            eprintln!("[WS] ❌ WebSocket engine error: {:?}", e);
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fast_hash_consistency() {
        let token = "0x1234567890abcdef";
        let hash1 = fast_hash(token.as_bytes());
        let hash2 = fast_hash(token.as_bytes());
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_ws_config_defaults() {
        let config = WsConfig::default();
        assert!(config.url.contains("polymarket"));
        assert!(config.user_agent.contains("Chrome"));
    }
}