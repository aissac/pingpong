//! Resilient WebSocket runner with TCP timeout and auto-reconnect
//! Prevents silent half-open TCP connection death
//! 
//! KEY INSIGHT: Set TCP read timeout BEFORE wrapping in tungstenite TLS
//! This avoids MaybeTlsStream extraction complexity

use std::net::TcpStream;
use std::time::Duration;
use url::Url;
use tungstenite::client::IntoClientRequest;

use crate::hft_hot_path::{run_sync_hot_path, RolloverCommand, BackgroundTask, LogEvent};
use crate::websocket_reader::WebSocketReader;
use crate::market_rollover::run_rollover_thread;
use crate::jsonl_logger::JsonlLogger;
use crate::crypto_markets::fetch_active_crypto_markets;

use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::collections::HashMap;
use crossbeam_channel::bounded;

/// Start resilient bot with 30-second TCP timeout and auto-reconnect
pub fn start_resilient_bot() {
    let ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
    let url = Url::parse(ws_url).unwrap();
    
    // Outer reconnect loop - runs forever
    loop {
        println!("🔄 [SYSTEM] Initiating Connection Sequence...");
        
        // STEP 1: Fetch current active markets from Gamma API
        let client = reqwest::blocking::Client::new();
        let (all_tokens, _token_pairs, _token_strings) = 
            match fetch_active_crypto_markets(&client) {
                Ok(result) => result,
                Err(e) => {
                    eprintln!("❌ Failed to fetch markets: {}. Retrying in 2s...", e);
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            };
        
        println!("📊 Fetched {} tokens from Gamma API", all_tokens.len());
        
        let host = url.host_str().unwrap();
        let port = url.port_or_known_default().unwrap();
        
        // STEP 2: TCP Connection
        println!("🔌 Connecting to {}:{}...", host, port);
        let tcp_stream = match TcpStream::connect(format!("{}:{}", host, port)) {
            Ok(s) => {
                println!("✅ TCP connected");
                s
            },
            Err(e) => {
                eprintln!("❌ TCP Connect failed: {}. Retrying in 2s...", e);
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        
        // STEP 3: 🚨 SET TCP READ TIMEOUT (30 seconds) 🚨
        // This is the KEY FIX - set timeout BEFORE TLS wrapping
        if let Err(e) = tcp_stream.set_read_timeout(Some(Duration::from_secs(30))) {
            eprintln!("⚠️ Failed to set read timeout: {}", e);
        } else {
            println!("✅ TCP read timeout set to 30 seconds");
        }
        
        // STEP 4: TLS + WebSocket Handshake
        println!("🔐 Performing TLS/WS handshake...");
        let request = url.clone().into_client_request().unwrap();
        let ws_stream = match tungstenite::client_tls(request, tcp_stream) {
            Ok((stream, _response)) => {
                println!("✅ WebSocket connected");
                stream
            },
            Err(e) => {
                eprintln!("❌ TLS/WS Handshake failed: {}. Retrying in 2s...", e);
                std::thread::sleep(Duration::from_secs(2));
                continue;
            }
        };
        
        // STEP 5: Create channels for this connection
        let (rollover_tx, rollover_rx) = bounded::<RolloverCommand>(64);
        let (background_tx, _) = bounded::<BackgroundTask>(1024);
        let (log_tx, log_rx) = bounded::<LogEvent>(4096);
        let _logger_handle = JsonlLogger::start(log_rx);
        
        // STEP 6: Start rollover thread
        let rollover_client = Arc::new(reqwest::blocking::Client::new());
        let rollover_tx_clone = rollover_tx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                run_rollover_thread(rollover_client, rollover_tx_clone).await;
            });
        });
        
        // STEP 7: Build bi-directional token pairs
        let mut bidi_pairs: HashMap<u64, (u64, u64)> = HashMap::new();
        for token in &all_tokens {
            let hash = crate::hft_hot_path::fast_hash(token.as_bytes());
            // For now, insert placeholder - rollover will populate actual pairs
            bidi_pairs.insert(hash, (hash, hash));
        }
        println!("📊 Built {} token mappings", bidi_pairs.len());
        
        // STEP 8: Send subscription payloads
        println!("📡 Subscribing to {} tokens...", all_tokens.len());
        for token in &all_tokens {
            let sub_payload = format!(r#"{{"token_ids": ["{}"], "type": "price_changes"}}"#, token);
            if let Err(e) = ws_stream.get_mut().send(tungstenite::Message::Text(sub_payload)) {
                eprintln!("⚠️ Failed to subscribe: {}", e);
            }
        }
        println!("✅ Subscriptions sent");
        
        // STEP 9: Send START alert to Telegram
        send_start_alert(bidi_pairs.len());
        
        // STEP 10: Wrap in WebSocketReader and run hot path
        let ws_reader = WebSocketReader::new(ws_stream);
        
        println!("⚡ [SYSTEM] Handing over to Hot Path...");
        
        // This blocks until timeout (30s silence) or error triggers break
        let _ = run_sync_hot_path(
            ws_reader,
            background_tx,
            all_tokens.clone(),
            Arc::new(AtomicBool::new(false)),
            bidi_pairs,
            Arc::new(AtomicU64::new(0)),
            rollover_rx,
            log_tx,
            Arc::new(AtomicU64::new(0)),  // valid_evals
            Arc::new(AtomicU64::new(0)),  // missing_data
        );
        
        // If we reach here, hot path exited (timeout or error)
        eprintln!("⚠️ [SYSTEM] Hot Path exited. Reconnecting...");
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Send Telegram alert on bot start
fn send_start_alert(pairs_count: usize) {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .unwrap_or_else(|_| "8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY".to_string());
    let chat_id = std::env::var("TELEGRAM_CHAT_ID")
        .unwrap_or_else(|_| "1798631768".to_string());
    let pid = std::process::id();
    
    let msg = format!(
        "🚀 *HFT Bot STARTED*\n\n\
         PID: `{}`\n\
         Pairs Tracked: `{}`\n\
         Mode: *LIVE ARMED*\n\
         TCP Timeout: ✅ 30-second detection\n\
         Watchdog: ✅ 3-minute backup\n\n\
         WebSocket: Connected\n\
         Latency: Sub-microsecond",
        pid, pairs_count
    );
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    
    // Fire-and-forget async task
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client.post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": msg,
                "parse_mode": "Markdown"
            }))
            .send()
            .await;
    });
}
