//! Pingpong - Main Entry Point
//! 
//! Phase 4.0: Gabagool Strategy - Expensive-side skew + directional betting
//! Based on RD Olivaw's proven PingPong strategy
//!
//! Supports two execution modes:
//! - HFT_SYNC: Pinned thread, sync WebSocket, thread-local orderbook (sub-1µs)
//! - LEGACY_ASYNC: Tokio async, DashMap, channels (default)

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;
use chrono::Local;

mod orderbook;
mod api;
mod strategy;
mod trading;
mod websocket;
mod hot_switchover;
mod pnl;
mod gabagool_strategy;
mod merger;
mod maker_hybrid;
mod ghost_simulator;
mod simd_parse;
mod simd_hot_path;
mod hft_hot_path;
mod execution_mode;

use api::{PolyClient, SimplifiedMarket};
use websocket::TokenData;
use orderbook::OrderBookTracker;
use strategy::{PingpongStrategy, StrategyEvent};
use trading::{TradingEngine, TradingConfig, start_trading_loop, ArbitrageSignal};
use maker_hybrid::{MakerSignal, MakerSide, InventoryTracker, evaluate_maker_opportunity};
use websocket::OrderBookUpdate;
use hot_switchover::{run_hot_switchover_manager, AppState};
use pnl::{PnlTracker, create_trade_result};
use gabagool_strategy::{GabagoolStrategy, GabagoolConfig, TradingSignal};
use merger::PolyMerger;
use execution_mode::ExecutionMode;

fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .finish();
    
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    info!("═══════════════════════════════════════════════════════════════");
    info!("   PINGPONG v0.4.0 - Gabagool Strategy");
    info!("═══════════════════════════════════════════════════════════════");
    
    // Parse command line args
    let args: Vec<String> = env::args().collect();
    let dry_run = !args.contains(&"--live".to_string());
    let use_websocket = args.contains(&"--ws".to_string());
    
    if dry_run {
        info!("⚠️  MODE: DRY RUN (paper trading, no real orders)");
    } else {
        info!("🔴  MODE: LIVE TRADING");
        info!("⚠️  Real money will be risked!");
    }
    
    if use_websocket {
        info!("📡 Using Hot Switchover WebSocket (primary + backup)");
    } else {
        info!("🌐 Using REST API (slower polling mode)");
    }
    
    // Get private key from env
    let private_key = env::var("POLYMARKET_PRIVATE_KEY")
        .unwrap_or_else(|_| {
            warn!("POLYMARKET_PRIVATE_KEY not set - running in read-only mode");
            String::new()
        });
    
    // Get tokens to subscribe to
    let tokens = vec![
        "0x7498409f0d52655f38f390b14b5c9030ff4d77b0".to_string(), // BTC 5m
        "0x81099561cd5b72d0040a9f15b74e181976a94e94".to_string(), // BTC 15m
        "0x5bfc6c71947590e41f27c14f6a5ea85a5e4f14b9".to_string(), // ETH 5m
        "0x6d0e14b42635d7e143dc7e50e27d6fa70b23e6f8".to_string(), // ETH 15m
    ];
    
    // Check execution mode
    let mode = ExecutionMode::from_env();
    let running = Arc::new(AtomicBool::new(true));
    
    match mode {
        ExecutionMode::HftSync => {
            // HFT SYNC MODE: Pinned thread, sync WebSocket
            info!("");
            info!("🔥 EXECUTION MODE: HFT SYNC (sub-microsecond)");
            info!("   - Sync tungstenite WebSocket");
            info!("   - Thread-local LocalOrderBook");
            info!("   - Busy-poll loop (no channels)");
            info!("   - CPU core pinning");
            info!("");
            
            // Start background tokio runtime for non-critical tasks
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?;
            
            rt.spawn(async {
                // Background tasks: metrics, REST API, etc.
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            });
            
            // Run HFT hot path on main thread (will be pinned)
            execution_mode::run_hft_sync(tokens, running)?;
        }
        
        ExecutionMode::LegacyAsync => {
            // LEGACY ASYNC MODE: Tokio async, DashMap
            info!("");
            info!("🚀 EXECUTION MODE: LEGACY ASYNC");
            info!("   - Async tokio-tungstenite WebSocket");
            info!("   - DashMap for concurrent orderbook");
            info!("   - Crossbeam channels");
            info!("");
            
            // Create AppState
            let state = Arc::new(AppState::new());
            
            // Run the async bot
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                if use_websocket {
                    run_hot_switchover_manager(tokens, state, running).await;
                } else {
                    warn!("REST API mode not yet implemented for async");
                }
            });
        }
    }
    
    info!("Bot shutdown complete");
    Ok(())
}