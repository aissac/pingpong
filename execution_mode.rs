//! Execution mode switch for HFT vs Legacy
//!
//! Allows runtime switching between:
//! - HFT_SYNC: Pinned thread, sync WebSocket, thread-local orderbook
//! - LEGACY_ASYNC: Tokio async, DashMap, channels

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Execution mode configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Sub-microsecond pinned thread (sync tungstenite)
    HftSync,
    /// Legacy async mode (tokio-tungstenite)
    LegacyAsync,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::LegacyAsync
    }
}

impl ExecutionMode {
    pub fn from_env() -> Self {
        match std::env::var("EXECUTION_MODE").as_deref() {
            Ok("HFT_SYNC") | Ok("hft_sync") | Ok("hft") => Self::HftSync,
            _ => Self::LegacyAsync,
        }
    }
}

/// Run the bot in HFT sync mode
pub fn run_hft_sync(
    tokens: Vec<String>,
    running: Arc<AtomicBool>,
) -> anyhow::Result<()> {
    tracing::info!("🔥 Starting HFT SYNC mode (pinned thread)");
    tracing::info!("   This mode uses:");
    tracing::info!("   - Sync tungstenite WebSocket");
    tracing::info!("   - Thread-local LocalOrderBook (no DashMap)");
    tracing::info!("   - Busy-poll loop (no channels)");
    tracing::info!("   - CPU core pinning");
    
    // Start the dual WebSocket hot path
    let handle = super::hft_hot_path::start_dual_hft_hot_path(
        "wss://ws-subscriptions-clob.polymarket.com/ws/market",
        "wss://ws-subscriptions-clob.polymarket.com/ws/market",
        tokens,
        running,
    );
    
    // Wait for completion
    handle.join().expect("HFT thread panicked");
    
    Ok(())
}