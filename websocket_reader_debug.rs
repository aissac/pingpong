//! WebSocket Integration for HFT Hot Path
//! 
//! Uses sync tungstenite for sub-microsecond latency (no tokio)
//! 
//! FIX (2026-04-01): CORRECT subscription format per NotebookLM
//! - Use "assets" not "assets_ids"
//! - Remove "operation" field (not in Polymarket schema)
//! - Use RAW 77-char clobTokenIds (NOT hashed)

use std::io::Read;
use tungstenite::{Message, client_tls};
use std::net::TcpStream;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;

/// Connect to Polymarket WebSocket and return a stream for the hot path
pub fn connect_to_polymarket(tokens: Vec<String>) -> WebSocketReader {
    let url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
    
    println!("Connecting to Polymarket WebSocket...");
    
    // OPTION C: Set TCP timeout BEFORE TLS handshake (NotebookLM recommendation)
    let tcp_stream = TcpStream::connect("ws-subscriptions-clob.polymarket.com:443")
        .expect("Failed to connect TCP stream");
    
    tcp_stream.set_read_timeout(Some(Duration::from_secs(30)))
        .expect("Failed to set read timeout");
    
    println!("TCP connected with 30s timeout");
    
    let (mut socket, _response) = client_tls(url, tcp_stream)
        .expect("Failed TLS handshake");
    
    println!("WebSocket connected");
    
    // FIX: CORRECT subscription format (NotebookLM)
    // Use "assets" field with RAW 77-char clobTokenIds
    let sub_payload = serde_json::json!({
        "assets": tokens.clone(),
        "type": "market"
    });
    
    // DEBUG: Log what we're sending
    println!("DEBUG: Sending {} tokens", tokens.len());
    if let Some(first) = tokens.first() {
        println!("DEBUG: First token: {}... (len={})", &first[..std::cmp::min(60, first.len())], first.len());
    }
    println!("DEBUG: Payload: {}", sub_payload);
    
    let msg = Message::Text(sub_payload.to_string());
    socket.send(msg).expect("Failed to subscribe");
    
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
                let bytes = text.as_bytes();
                let len = std::cmp::min(buf.len(), bytes.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            Ok(Message::Binary(data)) => {
                let len = std::cmp::min(buf.len(), data.len());
                buf[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            Ok(Message::Ping(_)) => Ok(0),
            Ok(Message::Pong(_)) => Ok(0),
            Ok(_) => Ok(0),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
        }
    }
}

impl WebSocketReader {
    pub fn send(&mut self, msg: Message) -> Result<(), tungstenite::Error> {
        self.socket.send(msg)
    }
}
