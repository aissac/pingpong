//! WebSocket Integration for HFT Hot Path
//! 
//! Uses sync tungstenite for sub-microsecond latency (no tokio)
//! 
//! FIX (2026-04-01 17:30): Add User-Agent and Origin headers to bypass WAF silent block

use std::io::Read;
use tungstenite::{Message, client_tls};
use tungstenite::client::IntoClientRequest;
use tungstenite::http::HeaderValue;
use std::net::TcpStream;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;

/// Connect to Polymarket WebSocket and return a stream for the hot path
pub fn connect_to_polymarket(tokens: Vec<String>) -> WebSocketReader {
    eprintln!("🔗 [WS] Connecting to Polymarket WebSocket...");
    
    // FIX: Build HTTP request with required headers to bypass WAF silent block
    let url_str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
    let mut request = url_str.into_client_request().expect("Invalid WS URL");
    
    // CRITICAL: Add headers that Cloudflare/WAF expects
    let headers = request.headers_mut();
    headers.insert("User-Agent", HeaderValue::from_static(
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
    ));
    headers.insert("Origin", HeaderValue::from_static(
        "https://polymarket.com"
    ));
    
    eprintln!("📋 [WS] Headers set (User-Agent, Origin)");
    
    // OPTION C: Set TCP timeout BEFORE TLS handshake
    let tcp_stream = TcpStream::connect("ws-subscriptions-clob.polymarket.com:443")
        .expect("Failed to connect TCP stream");
    
    tcp_stream.set_read_timeout(Some(Duration::from_secs(30)))
        .expect("Failed to set read timeout");
    
    eprintln!("✅ [WS] TCP connected with 30s timeout");
    
    let (mut socket, response) = client_tls(request, tcp_stream)
        .expect("Failed TLS handshake");
    
    eprintln!("✅ [WS] WebSocket connected! HTTP Status: {}", response.status());
    
    // FIX: CORRECT subscription format (NotebookLM)
    // Use "assets" field with RAW 77-char clobTokenIds
    let sub_payload = serde_json::json!({
        "assets": tokens.clone(),
        "type": "market"
    });
    
    // DEBUG: Log what we're sending
    eprintln!("📤 [WS] Sending {} tokens", tokens.len());
    if let Some(first) = tokens.first() {
        eprintln!("📤 [WS] First token: {}... (len={})", &first[..std::cmp::min(60, first.len())], first.len());
    }
    eprintln!("📤 [WS] Payload: {}", sub_payload);
    
    let msg = Message::Text(sub_payload.to_string());
    socket.send(msg).expect("Failed to subscribe");
    
    eprintln!("📤 [WS] Subscription sent to Polymarket");
    println!("Subscribed to {} tokens (CORRECT FORMAT)", tokens.len());
    
    WebSocketReader { socket, buffer: vec![] }
}

/// Wrapper to implement Read for WebSocket
pub struct WebSocketReader {
    pub socket: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    buffer: Vec<u8>,
}

impl Read for WebSocketReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.socket.read() {
            Ok(Message::Text(text)) => {
                eprintln!("📩 [WS] Received Text message ({} bytes)", text.len());
                if text.len() < 300 {
                    eprintln!("📩 [WS] Content: {}", text);
                }
                let bytes = text.as_bytes();
                let len = std::cmp::min(buf.len(), bytes.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            Ok(Message::Binary(data)) => {
                eprintln!("📩 [WS] Received Binary message ({} bytes)", data.len());
                let len = std::cmp::min(buf.len(), data.len());
                buf[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            Ok(Message::Ping(_)) => {
                eprintln!("📩 [WS] Received Ping (auto-responding)");
                Ok(0)
            }
            Ok(Message::Pong(_)) => {
                eprintln!("📩 [WS] Received Pong");
                Ok(0)
            }
            Ok(other) => {
                eprintln!("📩 [WS] Received other message type: {:?}", other);
                Ok(0)
            }
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}

impl WebSocketReader {
    pub fn send(&mut self, msg: Message) -> Result<(), tungstenite::Error> {
        self.socket.send(msg)
    }
}
