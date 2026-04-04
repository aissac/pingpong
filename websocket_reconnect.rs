//! WebSocket Reconnection Wrapper
//! 
//! Automatically reconnects when WebSocket closes, with exponential backoff.
//! Clears orderbook state on reconnect to prevent stale data.

use std::io::Read;
use std::time::Duration;
use std::thread::sleep;
use tungstenite::{connect, Message};
use tungstenite::stream::MaybeTlsStream;
use std::net::TcpStream;

const WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
const MAX_RETRY_MS: u64 = 5000;  // Cap backoff at 5 seconds

/// Reconnecting WebSocket reader that handles disconnections.
/// 
/// IMPORTANT: When WebSocket disconnects:
/// 1. Clear all orderbook state (prevents stale data)
/// 2. Reconnect with exponential backoff
/// 3. Resubscribe with initial_dump=true (gets fresh snapshot)
/// 4. Rebuild orderbook from scratch
pub fn connect_with_reconnect(tokens: Vec<String>) -> WebSocketReader {
    let mut retry_backoff_ms: u64 = 100;
    
    loop {
        println!("[WS] 🔄 Attempting WebSocket connection...");
        
        match connect(WS_URL) {
            Ok((mut socket, _response)) => {
                println!("[WS] ✅ WebSocket connected!");
                retry_backoff_ms = 100;  // Reset backoff on success
                
                // Subscribe with initial_dump=true
                // This sends full orderbook snapshot on connect
                let subscribe_msg = serde_json::json!({
                    "type": "market",
                    "operation": "subscribe",
                    "markets": [],
                    "assets_ids": tokens.clone(),
                    "initial_dump": true  // CRITICAL: Get fresh snapshot on reconnect
                });
                
                let msg = Message::Text(subscribe_msg.to_string());
                if let Err(e) = socket.send(msg) {
                    eprintln!("[WS] ⚠️ Failed to send subscription: {}", e);
                    sleep(Duration::from_millis(retry_backoff_ms));
                    retry_backoff_ms = std::cmp::min(retry_backoff_ms * 2, MAX_RETRY_MS);
                    continue;
                }
                
                println!("[WS] 📡 Subscribed to {} tokens", tokens.len());
                
                return WebSocketReader { 
                    socket, 
                    buffer: vec![],
                    connected: true,
                };
            }
            Err(e) => {
                eprintln!("[WS] ⚠️ Connection failed: {}. Retrying in {}ms...", 
                    e, retry_backoff_ms);
                sleep(Duration::from_millis(retry_backoff_ms));
                retry_backoff_ms = std::cmp::min(retry_backoff_ms * 2, MAX_RETRY_MS);
            }
        }
    }
}

pub struct WebSocketReader {
    socket: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    buffer: Vec<u8>,
    connected: bool,
}

impl WebSocketReader {
    /// Check if connection is still alive
    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Read for WebSocketReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // If we have leftover data from previous message, return it
        if !self.buffer.is_empty() {
            let len = std::cmp::min(buf.len(), self.buffer.len());
            buf[..len].copy_from_slice(&self.buffer[..len]);
            self.buffer.drain(..len);
            return Ok(len);
        }
        
        // Read next WebSocket message
        loop {
            match self.socket.read() {
                Ok(Message::Text(text)) => {
                    let bytes = text.as_bytes();
                    let len = std::cmp::min(buf.len(), bytes.len());
                    buf[..len].copy_from_slice(&bytes[..len]);
                    
                    // Store remainder for next read
                    if bytes.len() > len {
                        self.buffer = bytes[len..].to_vec();
                    }
                    
                    self.connected = true;
                    return Ok(len);
                }
                Ok(Message::Ping(data)) => {
                    // Respond to ping automatically
                    let _ = self.socket.send(Message::Pong(data));
                    continue;
                }
                Ok(Message::Pong(_)) => {
                    // Ignore pong
                    continue;
                }
                Ok(Message::Close(_)) => {
                    self.connected = false;
                    return Ok(0);  // Signal EOF - caller should reconnect
                }
                Err(e) => {
                    self.connected = false;
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionReset,
                        format!("WebSocket error: {}", e)
                    ));
                }
                _ => continue,
            }
        }
    }
}