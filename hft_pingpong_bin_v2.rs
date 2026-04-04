// src/bin/hft_pingpong.rs
//! HFT Pingpong - Unified Binary (Hot Path + Background Thread)

use crossbeam_channel::bounded;
use std::thread;
use chrono::{Utc, Timelike};

use pingpong::hft_hot_path::run_sync_hot_path;
use pingpong::hft_hot_path::BackgroundTask;

/// Get current 15-minute periods for market slugs
fn get_current_periods() -> Vec<i64> {
    let now = Utc::now();
    let minute = (now.minute() / 15) * 15;
    let period_start = now.with_minute(minute).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let base_ts = period_start.timestamp();
    vec![base_ts, base_ts - 900, base_ts + 900]  // current, prev 15m, next 15m
}

/// Fetch tokens from Gamma API (called on startup)
async fn fetch_tokens() -> Vec<String> {
    println!("📊 Fetching BTC/ETH Up/Down markets from REST API...");
    
    let client = reqwest::Client::new();
    let mut all_tokens: Vec<String> = Vec::new();
    
    let assets = ["btc", "eth"];
    let periods = get_current_periods();
    
    for asset in &assets {
        for &period_ts in &periods {
            // Try 15m market
            let slug_15m = format!("{}-updown-15m-{}", asset, period_ts);
            if let Ok(resp) = client
                .get(&format!("https://gamma-api.polymarket.com/markets?slug={}", slug_15m))
                .send()
                .await
            {
                if let Ok(text) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(markets) = json.as_array() {
                            for market in markets {
                                if let Some(tokens) = market.get("tokens") {
                                    if let Some(tokens_array) = tokens.as_array() {
                                        for token in tokens_array {
                                            if let Some(token_id) = token.as_str() {
                                                all_tokens.push(token_id.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Try 5m market
            let slug_5m = format!("{}-updown-5m-{}", asset, period_ts - 600);
            if let Ok(resp) = client
                .get(&format!("https://gamma-api.polymarket.com/markets?slug={}", slug_5m))
                .send()
                .await
            {
                if let Ok(text) = resp.text().await {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(markets) = json.as_array() {
                            for market in markets {
                                if let Some(tokens) = market.get("tokens") {
                                    if let Some(tokens_array) = tokens.as_array() {
                                        for token in tokens_array {
                                            if let Some(token_id) = token.as_str() {
                                                all_tokens.push(token_id.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Deduplicate
    all_tokens.sort();
    all_tokens.dedup();
    
    println!("📊 Fetched {} tokens from {} markets", all_tokens.len(), all_tokens.len() / 2);
    all_tokens
}

fn main() {
    println!("=======================================================");
    println!("🚀 INITIALIZING POLYMARKET HFT ENGINE (Unified)");
    println!("=======================================================");

    // 1. Create the lock-free bridge
    let (tx, rx) = bounded(65536);

    // 2. Spawn Background Thread (Ghost Simulation + Telegram)
    let _bg_handle = thread::Builder::new()
        .name("background-dispatcher".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime");

            rt.block_on(async move {
                println!("[BG] Background Tokio runtime started");

                while let Ok(task) = rx.recv() {
                    match task {
                        BackgroundTask::EdgeDetected { token_hash, combined_price, .. } => {
                            println!("[BG] Edge: hash={:016x} combined=${:.4}", 
                                token_hash, 
                                combined_price as f64 / 1_000_000.0
                            );
                        }
                        BackgroundTask::LatencyStats { min_ns, max_ns, avg_ns, p99_ns, sample_count } => {
                            println!(
                                "[HFT] 🔥 5s STATS | avg={:.2}µs min={:.2}µs max={:.2}µs p99={:.2}µs | {} samples",
                                avg_ns as f64 / 1000.0,
                                min_ns as f64 / 1000.0,
                                max_ns as f64 / 1000.0,
                                p99_ns as f64 / 1000.0,
                                sample_count
                            );
                        }
                    }
                }
            });
        })
        .expect("Failed to spawn background thread");

    // 3. Pin Hot Path to isolated CPU core
    if let Some(core_ids) = core_affinity::get_core_ids() {
        for core in core_ids {
            if core.id == 1 {
                if core_affinity::set_for_current(core) {
                    println!("🔒 [HFT] Pinned to core {}", core.id);
                }
                break;
            }
        }
    }

    // 4. Get tokens from Gamma API
    let tokens: Vec<String> = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(fetch_tokens());

    if tokens.is_empty() {
        eprintln!("❌ No tokens fetched, exiting");
        std::process::exit(1);
    }

    println!("🚀 Starting HFT hot path...");
    println!("📡 Subscribing to {} tokens", tokens.len());

    // 5. Run Hot Path
    run_sync_hot_path(tx, tokens);

    eprintln!("🚨 [HFT] Hot path exited");
    std::process::exit(1);
}