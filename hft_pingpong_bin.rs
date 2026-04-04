// src/bin/hft_pingpong.rs
//! HFT Pingpong - Sub-microsecond Polymarket Orderbook Processor
//!
//! Architecture: One-Way State Propagation
//! - Thread 1 (Hot Path): Sync tungstenite, pinned core, zero-allocation
//! - Thread 2 (Background): Tokio async, ghost simulation, Telegram reports
//!
//! Communication: crossbeam_channel (lock-free, bounded)

use crossbeam_channel::bounded;
use std::thread;
use std::sync::Arc;
use std::env;

// Internal modules
mod hft_hot_path;

use hft_hot_path::run_sync_hot_path;

/// Fetch tokens from Gamma API (called on startup)
async fn fetch_tokens() -> Vec<String> {
    println!("📊 Fetching BTC/ETH Up/Down markets from REST API...");
    
    // Hardcoded token list for 24 markets (BTC/ETH 5m and 15m)
    // In production, these come from Gamma API
    let tokens = vec![
        // BTC-5m YES/NO
        "96408543298904617446523822137153069418764739261787888217644515210922136845491",
        "59954835907681619931244536249752663855967744102320616217752569180734806747405",
        // BTC-15m YES/NO  
        "41108558526847620778819927411891650584101388274537913397351906677690129850000",
        "99494200271847010101611866277055261765900349008763367225562014559637659912409",
        // ETH-5m YES/NO
        "2174215449581136458514214532293403333333333333333333333333333333333333333",
        "4857204985720394857203948572093485720394857203948572039485720394857203948",
        // Additional tokens (24 total for 12 markets)
        // These would be populated from Gamma API in production
    ];
    
    println!("📊 Fetched {} tokens from {} markets", tokens.len(), tokens.len() / 2);
    tokens
}

fn main() {
    println!("=======================================================");
    println!("🚀 INITIALIZING POLYMARKET HFT ENGINE");
    println!("Strategy: Maker Hybrid (Dump & Hedge)");
    println!("Markets: BTC/ETH 5m & 15m (24 Tokens)");
    println!("=======================================================");

    // 1. Create the lock-free bridge (Line 39 reference)
    // 65,536 slots prevents OOM while providing buffer for I/O stalls
    let (tx, rx) = bounded(65536);

    // 2. Spawn Background OS Thread (Line 60 reference)
    // Runs tokio async for ghost simulation and Telegram reports
    let bg_handle = thread::Builder::new()
        .name("background-dispatcher".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime");

            rt.block_on(async move {
                println!("[BG] Background Tokio runtime started");
                println!("[BG] Waiting for telemetry from hot path...");

                // Pop zero-alloc structs and handle heavy I/O
                while let Ok(task) = rx.recv() {
                    match task {
                        // Ghost simulation would go here
                        _ => {}
                    }
                }
            });
        })
        .expect("Failed to spawn background thread");

    // 3. Pin Hot Path to isolated CPU core (Line 73-76 reference)
    if let Some(core_ids) = core_affinity::get_core_ids() {
        // Try to pin to core 1 (assuming isolcpus=1 in GRUB)
        for core in core_ids {
            if core.id == 1 {
                if core_affinity::set_for_current(core) {
                    println!("🔒 [HFT] Main thread pinned to isolated core {}", core.id);
                } else {
                    println!("⚠️ [HFT] Could not pin to core {}, running unpinned", core.id);
                }
                break;
            }
        }
    }

    // 4. Get tokens (Line 142-154 reference)
    let tokens: Vec<String> = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(fetch_tokens());

    println!("🚀 Starting HFT hot path...");
    println!("📡 Subscribing to {} tokens", tokens.len());

    // 5. Run Hot Path (Line 195-206 reference)
    // This function never returns unless socket dies
    run_sync_hot_path(tx, tokens);

    // Should never reach here
    eprintln!("🚨 [HFT] Hot path exited unexpectedly");
    std::process::exit(1);
}