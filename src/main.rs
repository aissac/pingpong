//! Market Maker - Phase 3 Integration
//!
//! Wires together:
//! - Phase 1: Inventory tracking (inventory_tracker.rs)
//! - Phase 2: Skew-aware quotes (quote_manager.rs, hot_path.rs)
//! - Phase 3: WebSocket + CLOB API (ws_engine.rs, api_client.rs, rate_limiter.rs)
//!
//! Architecture:
//! - WebSocket engine: tokio async, bridged to crossbeam for hot path
//! - Hot path: sync, zero-allocation, processes events from crossbeam
//! - API client: async, rate-limited, handles EIP-712 signing
//! - Quote manager: generates quotes based on inventory skew

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use crossbeam_channel::{bounded, Receiver, Sender};

mod rate_limiter;
mod api_client;
mod inventory_tracker;
mod quote_manager;
mod hot_path;

// Network module
mod network {
    pub mod ws_engine;
}

use rate_limiter::RateLimiter;
use api_client::{ClobClient, Side, OrderStatus};
use inventory_tracker::InventoryTracker;
use quote_manager::{QuoteManager, QuoteConfig};
use hot_path::{BackgroundTask, spawn_hot_path};
use network::ws_engine::{WsEvent, WsEngine, spawn_ws_engine};

/// Configuration
const EDGE_THRESHOLD_U64: u64 = 940_000; // $0.94
const MAX_POSITION_U64: u64 = 5_000_000; // $5 USDC
const RATE_LIMIT_BURST: f64 = 300.0;
const RATE_LIMIT_PER_SEC: f64 = 50.0;

#[derive(Debug, Clone)]
struct Config {
    /// Run in demo mode (no real orders)
    dry_run: bool,
    /// Edge threshold (micro-USDC)
    edge_threshold: u64,
    /// Maximum position per token (micro-USDC)
    max_position: u64,
    /// Polymarket API key
    api_key: String,
    /// Polymarket API secret
    api_secret: String,
    /// Polymarket API passphrase
    api_passphrase: String,
    /// Gnosis Safe signer address
    signer_address: String,
    /// Private key (for EIP-712 signing)
    private_key: String,
    /// Token IDs to subscribe
    tokens: Vec<String>,
    /// YES/NO token pairs (hash -> hash)
    token_pairs: HashMap<u64, u64>,
    /// Hash -> Token ID mapping
    hash_to_id: HashMap<u64, String>,
    /// Token ID -> Condition ID mapping
    id_to_condition: HashMap<String, String>,
}

impl Config {
    fn from_env() -> Self {
        dotenvy::dotenv().ok();
        
        let dry_run = std::env::var("DRY_RUN")
            .unwrap_or_else(|_| "true".to_string())
            .parse()
            .unwrap_or(true);
        
        let api_key = std::env::var("POLYMARKET_API_KEY")
            .unwrap_or_else(|_| "demo_key".to_string());
        let api_secret = std::env::var("POLYMARKET_API_SECRET")
            .unwrap_or_else(|_| "demo_secret".to_string());
        let api_passphrase = std::env::var("POLYMARKET_PASSPHRASE")
            .unwrap_or_else(|_| "demo_passphrase".to_string());
        let signer_address = std::env::var("POLYMARKET_SAFE_ADDRESS")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
        let private_key = std::env::var("POLYMARKET_PRIVATE_KEY")
            .unwrap_or_else(|_| "0000000000000000000000000000000000000000000000000000000000000000".to_string());
        
        Self {
            dry_run,
            edge_threshold: EDGE_THRESHOLD_U64,
            max_position: MAX_POSITION_U64,
            api_key,
            api_secret,
            api_passphrase,
            signer_address,
            private_key,
            tokens: Vec::new(),
            token_pairs: HashMap::new(),
            hash_to_id: HashMap::new(),
            id_to_condition: HashMap::new(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   POLYMARKET MARKET MAKER - Phase 3 Integration");
    println!("═══════════════════════════════════════════════════════════════");
    
    // Load configuration
    let config = Config::from_env();
    
    println!("📋 Config: DRY_RUN={}, EDGE=${:.2}, MAX_POS=${:.2}",
        config.dry_run,
        config.edge_threshold as f64 / 1_000_000.0,
        config.max_position as f64 / 1_000_000.0,
    );
    
    // Create killswitch
    let killswitch = Arc::new(AtomicBool::new(false));
    let killswitch_ws = Arc::clone(&killswitch);
    
    // Create channels
    // WebSocket -> Hot Path
    let (ws_tx, ws_rx): (Sender<WsEvent>, Receiver<WsEvent>) = bounded(4096);
    // Hot Path -> Background Executor
    let (task_tx, task_rx): (Sender<BackgroundTask>, Receiver<BackgroundTask>) = bounded(1024);
    
    println!("✅ Channels created (WS→HP: 4096, HP→BG: 1024)");
    
    // Phase 1: Initialize inventory tracker
    let inventory = Arc::new(InventoryTracker::new(config.max_position));
    println!("✅ Inventory tracker initialized (max: ${:.2})", config.max_position as f64 / 1_000_000.0);
    
    // Phase 2: Initialize quote manager
    let quote_config = QuoteConfig {
        base_spread_bps: 100,     // 1% spread
        max_skew_spread_bps: 200, // Max 2% additional from skew
        skew_factor: 0.5,         // Moderate skew impact
        min_price: 0.01,          // Floor at $0.01
        max_price: 0.99,          // Cap at $0.99
        post_only: true,          // Always maker for rebate
    };
    let quote_manager = Arc::new(QuoteManager::new(quote_config, Arc::clone(&inventory), config.max_position));
    println!("✅ Quote manager initialized (spread: {}bps, skew_factor: {})", 
        quote_config.base_spread_bps, quote_config.skew_factor);
    
    // Phase 3: Initialize rate limiter
    let rate_limiter = Arc::new(RateLimiter::with_config(RATE_LIMIT_BURST, RATE_LIMIT_PER_SEC));
    println!("✅ Rate limiter initialized (burst: {}, rate: {}/s)", 
        RATE_LIMIT_BURST, RATE_LIMIT_PER_SEC);
    
    // Phase 3: Initialize API client
    let api_client = Arc::new(ClobClient::with_rate_limiter(
        config.api_key.clone(),
        config.api_secret.clone(),
        config.api_passphrase.clone(),
        config.signer_address.clone(),
        config.private_key.clone(),
        Arc::clone(&rate_limiter),
    ));
    println!("✅ CLOB API client initialized (signer: {}...)", &config.signer_address[..10]);
    
    // Fetch initial fee rate
    if !config.dry_run {
        match api_client.fetch_fee_rate().await {
            Ok(fee_bps) => println!("✅ Fee rate fetched: {}bps ({:.2}%)", fee_bps, fee_bps as f64 / 100.0),
            Err(e) => println!("⚠️ Failed to fetch fee rate: {} (using default 2%)", e),
        }
    }
    
    // TODO: Fetch token maps from Gamma API
    // For now, use placeholders
    let hash_to_id: HashMap<u64, String> = HashMap::new();
    let id_to_condition: HashMap<String, String> = HashMap::new();
    let token_pairs: HashMap<u64, u64> = HashMap::new();
    let tokens: Vec<String> = Vec::new();
    
    println!("📊 Token maps: {} hashes, {} conditions, {} pairs", 
        hash_to_id.len(), id_to_condition.len(), token_pairs.len() / 2);
    
    // Spawn hot path thread
    let hot_path_handle = spawn_hot_path(
        ws_rx,
        task_tx,
        hash_to_id.clone(),
        id_to_condition.clone(),
        token_pairs.clone(),
        Arc::clone(&killswitch),
    );
    println!("✅ Hot path thread spawned");
    
    // Spawn WebSocket engine
    let ws_handle = spawn_ws_engine(tokens.clone(), ws_tx);
    println!("✅ WebSocket engine spawned");
    
    // Spawn background executor
    let bg_handle = tokio::spawn(async move {
        println!("[BG] Background executor started (DRY_RUN={})", config.dry_run);
        
        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::EdgeDetected {
                    yes_token_hash,
                    no_token_hash,
                    yes_token_id,
                    no_token_id,
                    condition_id,
                    combined_ask,
                    yes_ask_price,
                    yes_ask_size,
                    no_ask_price,
                    no_ask_size,
                    timestamp_nanos,
                } => {
                    println!("[BG] 🎯 Edge detected: combined=${:.4} YES=${:.4} NO=${:.4}",
                        combined_ask as f64 / 1_000_000.0,
                        yes_ask_price as f64 / 1_000_000.0,
                        no_ask_price as f64 / 1_000_000.0,
                    );
                    println!("[BG]   Condition: {}", condition_id);
                    println!("[BG]   Sizes: YES={} NO={}", yes_ask_size, no_ask_size);
                    println!("[BG]   Hashes: YES={:016x} NO={:016x}", yes_token_hash, no_token_hash);
                    
                    if config.dry_run {
                        println!("[BG] [DRY_RUN] Would place orders:");
                        println!("[BG]   Maker: BUY YES {} @ ${:.4}", yes_ask_size, yes_ask_price as f64 / 1_000_000.0);
                        println!("[BG]   Taker: BUY NO {} @ ${:.4}", no_ask_size, no_ask_price as f64 / 1_000_000.0);
                        println!("[BG]   Expected profit: ${:.4}", (1_000_000 - combined_ask) as f64 / 1_000_000.0);
                    } else {
                        // TODO: Real order execution
                        // 1. Check inventory limits
                        // 2. Get current skew
                        // 3. Generate quotes via quote_manager
                        // 4. Place orders via api_client
                        println!("[BG] ⚠️ Live trading not yet implemented");
                    }
                }
                BackgroundTask::LatencyStats {
                    min_ns,
                    max_ns,
                    avg_ns,
                    p99_ns,
                    sample_count,
                } => {
                    println!("[BG] 📊 Latency: avg={:.2}µs min={:.2}µs max={:.2}µs p99={:.2}µs | {} samples",
                        avg_ns as f64 / 1000.0,
                        min_ns as f64 / 1000.0,
                        max_ns as f64 / 1000.0,
                        p99_ns as f64 / 1000.0,
                        sample_count,
                    );
                }
                BackgroundTask::OrderMatched {
                    order_id,
                    token_id,
                    matched_size,
                    price,
                } => {
                    println!("[BG] ✅ Order matched: {} @ ${:.4} (size: {})",
                        order_id,
                        price as f64 / 1_000_000.0,
                        matched_size as f64 / 1_000_000.0,
                    );
                    
                    // Update inventory
                    // TODO: Determine if this is YES or NO token
                    let _ = inventory.record_fill(
                        &token_id,
                        "unknown",
                        Side::Buy, // TODO: Determine actual side
                        matched_size,
                        price,
                    ).await;
                }
            }
        }
        
        println!("[BG] Background executor stopped");
    });
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("🚀 Market maker running!");
    println!("   Press Ctrl+C to stop");
    println!("═══════════════════════════════════════════════════════════════");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Shutdown signal received");
    
    // Set killswitch
    killswitch.store(true, Ordering::SeqCst);
    
    // Wait for threads
    println!("⏳ Waiting for threads to stop...");
    
    // Cancel WebSocket
    ws_handle.abort();
    
    // Wait for hot path
    hot_path_handle.join().unwrap_or_else(|_| println!("Hot path thread stopped"));
    
    // Wait for background executor
    bg_handle.abort();
    
    println!("✅ All threads stopped");
    println!("👋 Goodbye!");
    
    Ok(())
}

/// Fetch token maps from Gamma API
/// 
/// TODO: Implement actual Gamma API call
async fn fetch_token_maps(_client: &reqwest::Client) -> anyhow::Result<(
    HashMap<u64, String>,  // hash -> token_id
    HashMap<String, String>, // token_id -> condition_id
    HashMap<u64, u64>,      // YES_hash -> NO_hash
    Vec<String>,            // all token_ids
)> {
    // Placeholder - in production, call Gamma API
    Ok((HashMap::new(), HashMap::new(), HashMap::new(), Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        std::env::set_var("DRY_RUN", "true");
        let config = Config::from_env();
        assert!(config.dry_run);
    }
}