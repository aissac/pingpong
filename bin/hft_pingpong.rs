//! HFT Pingpong - Sub-microsecond synchronous WebSocket hot path
//!
//! This is a SEPARATE binary for pure sync HFT execution.
//! Run with: cargo run --bin hft_pingpong --release
//!
//! Architecture:
//! - NO tokio runtime (pure std::thread)
//! - NO async (sync tungstenite)
//! - NO DashMap (thread-local LocalOrderBook)
//! - NO channels (busy-poll loop)
//! - CPU core pinning for zero OS interrupts

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tracing::{info, warn};
use tracing_subscriber::FmtSubscriber;

// Import from the library
use pingpong::hft_hot_path::start_dual_hft_hot_path;

/// DRY RUN mode - no real orders
const DRY_RUN: bool = true;

/// WebSocket URLs
const WS_URL: &str = "wss://ws-subscriptions-clob.polymarket.com/ws/market";

/// Fetch token IDs from Polymarket REST API (same as async bot)
/// Only fetch BTC/ETH 5m and 15m Up/Down markets
fn get_tokens_from_api() -> Vec<String> {
    use chrono::{Utc, Timelike};
    
    info!("📡 Fetching BTC/ETH Up/Down markets from REST API...");
    
    // Calculate current periods (same as async bot)
    let now = Utc::now();
    let minute = (now.minute() / 15) * 15;
    let period_start = now.with_minute(minute).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let base_ts = period_start.timestamp();
    let periods = vec![base_ts, base_ts - 900, base_ts + 900]; // current, prev, next
    
    let mut tokens = Vec::new();
    
    // Fetch specific markets: BTC/ETH 5m and 15m
    let assets = ["btc", "eth"];
    let gamma_url = "https://gamma-api.polymarket.com";
    
    for asset in &assets {
        for &period_ts in &periods {
            // Try 15m market
            let slug_15m = format!("{}-updown-15m-{}", asset, period_ts);
            if let Some(market_tokens) = fetch_market_tokens(gamma_url, &slug_15m) {
                tokens.extend(market_tokens);
                info!("✅ Found 15m market: {}", slug_15m);
            }
            
            // Try 5m market (use period_ts - 600 for 5m offset)
            let slug_5m = format!("{}-updown-5m-{}", asset, period_ts - 600);
            if let Some(market_tokens) = fetch_market_tokens(gamma_url, &slug_5m) {
                tokens.extend(market_tokens);
                info!("✅ Found 5m market: {}", slug_5m);
            }
        }
    }
    
    if tokens.is_empty() {
        warn!("No markets found, using defaults");
        get_default_tokens()
    } else {
        info!("📊 Fetched {} tokens from {} markets", tokens.len(), tokens.len() / 2);
        tokens
    }
}

/// Fetch tokens for a specific market slug using Gamma API
fn fetch_market_tokens(gamma_url: &str, slug: &str) -> Option<Vec<String>> {
    let url = format!("{}/markets?slug={}", gamma_url, slug);
    
    match minreq::get(&url).send() {
        Ok(response) => {
            let body = response.as_str().unwrap_or("[]");
            
            // Parse as array of markets
            match serde_json::from_str::<Vec<serde_json::Value>>(body) {
                Ok(markets) => {
                    if markets.is_empty() {
                        return None;
                    }
                    
                    let market = &markets[0];
                    
                    // Get clobTokenIds - it's a JSON string that needs to be parsed
                    if let Some(token_ids_str) = market.get("clobTokenIds").and_then(|t| t.as_str()) {
                        if let Ok(token_ids) = serde_json::from_str::<Vec<String>>(token_ids_str) {
                            if token_ids.len() >= 2 && !token_ids[0].is_empty() {
                                return Some(token_ids.into_iter().take(2).collect());
                            }
                        }
                    }
                    None
                }
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

fn get_default_tokens() -> Vec<String> {
    // These are the hardcoded token IDs from the async bot
    vec![
        // BTC 5m YES
        "0x7498409f0d52655f38f390b14b5c9030ff4d77b0".to_string(),
        // BTC 15m YES
        "0x81099561cd5b72d0040a9f15b74e181976a94e94".to_string(),
        // ETH 5m YES
        "0x5bfc6c71947590e41f27c14f6a5ea85a5e4f14b9".to_string(),
        // ETH 15m YES
        "0x6d0e14b42635d7e143dc7e50e27d6fa70b23e6f8".to_string(),
    ]
}

fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    info!("═══════════════════════════════════════════════════════════════");
    info!("   PINGPONG HFT - Sub-Microsecond Sync Mode");
    info!("═══════════════════════════════════════════════════════════════");
    info!("");
    info!("🔥 ARCHITECTURE:");
    info!("   - Sync tungstenite WebSocket (NO tokio)");
    info!("   - Thread-local LocalOrderBook (NO DashMap)");
    info!("   - Busy-poll loop (NO channels)");
    info!("   - CPU core pinning (isolated)");
    info!("");
    
    if DRY_RUN {
        info!("⚠️  MODE: DRY RUN (paper trading, no real orders)");
    } else {
        info!("🔴  MODE: LIVE TRADING");
        info!("⚠️  Real money will be risked!");
    }
    info!("");
    
    // Shutdown flag
    let running = Arc::new(AtomicBool::new(true));
    
    // Setup Ctrl+C handler
    {
        let running = running.clone();
        ctrlc::set_handler(move || {
            info!("Received Ctrl+C, shutting down...");
            running.store(false, Ordering::Relaxed);
        }).expect("Failed to set Ctrl+C handler");
    }
    
    // Fetch tokens from API
    let tokens = get_tokens_from_api();
    info!("📊 Subscribing to {} tokens", tokens.len());
    
    // Start HFT hot path
    info!("🚀 Starting HFT hot path...");
    let handle = start_dual_hft_hot_path(
        WS_URL,
        WS_URL, // Same URL for primary and backup
        tokens,
        running,
    );
    
    // Wait for completion
    handle.join().expect("HFT thread panicked");
    
    info!("Bot shutdown complete");
}
