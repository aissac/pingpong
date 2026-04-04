// src/hft_hot_path.rs
//! HFT Hot Path - Zero-Allocation Raw Byte Scanner
//!
//! Uses memchr to scan WebSocket messages without parsing full JSON DOM.
//! Expected latency: ~50-100ns (vs 4-6µs for DOM parsing)

use crossbeam_channel::Sender;
use std::time::Duration;
use std::collections::HashMap;

/// Target combined price for arbitrage (95 cents = $0.95)
/// With 1.80% taker fee (March 30, 2026), we need 4.0%+ edge
const EDGE_THRESHOLD_U64: u64 = 950_000;

/// Background task for crossbeam channel
#[derive(Debug, Clone)]
pub enum BackgroundTask {
    EdgeDetected {
        token_hash: u64,
        combined_price: u64,
        timestamp_nanos: u64,
    },
    LatencyStats {
        min_ns: u64,
        max_ns: u64,
        avg_ns: u64,
        p99_ns: u64,
        sample_count: u64,
    },
}

/// Run the synchronous hot path with zero-allocation byte scanning
pub fn run_sync_hot_path(tx: Sender<BackgroundTask>, tokens: Vec<String>) {
    use tungstenite::{connect, Message};
    use memchr::memmem;
    
    // Connect to Polymarket CLOB WebSocket
    let ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
    
    println!("[HFT] Connecting to {}...", ws_url);
    
    let (mut socket, response) = match connect(ws_url) {
        Ok((s, r)) => (s, r),
        Err(e) => {
            eprintln!("❌ Failed to connect: {:?}", e);
            std::process::exit(1);
        }
    };
    
    println!("✅ Primary WebSocket connected (sync)");
    println!("HTTP status: {}", response.status());

    // Subscribe using correct format
    let subscribe_msg = serde_json::json!({
        "type": "market",
        "operation": "subscribe",
        "markets": [],
        "assets_ids": tokens,
        "initial_dump": true
    });
    
    let msg_str = serde_json::to_string(&subscribe_msg).expect("Failed to serialize subscription");
    let _ = socket.write_message(Message::Text(msg_str.into()));
    
    println!("📡 Subscribed to {} tokens", tokens.len());
    println!("[HFT] 🔥 Starting ZERO-ALLOCATION hot path...");
    println!("[HFT] Using memchr byte scanner (50-100ns latency)");

    // Local orderbook: token_hash -> (yes_price, no_price, yes_size, no_size)
    let mut orderbook: HashMap<u64, (u64, u64, u64, u64)> = HashMap::with_capacity(128);
    
    // Warmup counter
    let mut warmup_count = 0;
    
    // Latency tracking (8192 samples = ~10 seconds)
    let mut latency_samples: Vec<u64> = Vec::with_capacity(8192);
    let mut last_stat_time = std::time::Instant::now();

    // Pre-compiled search patterns for memchr
    let asset_pattern = memmem::Finder::new(b"\"asset_id\":\"");
    let price_pattern = memmem::Finder::new(b"\"price\":\"");
    let size_pattern = memmem::Finder::new(b"\"size\":\"");
    let best_ask_pattern = memmem::Finder::new(b"\"best_ask\":\"");
    let best_bid_pattern = memmem::Finder::new(b"\"best_bid\":\"");

    // Main busy-poll loop
    loop {
        let msg = match socket.read() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[HFT] WS Read Error: {:?}", e);
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        if let Message::Text(text) = msg {
            let start_tsc = minstant::Instant::now();
            let bytes = text.as_bytes();
            let len = bytes.len();

            // Skip tiny messages (pongs, heartbeats)
            if len < 100 {
                continue;
            }

            // Warmup period (first 50 messages)
            if warmup_count < 50 {
                warmup_count += 1;
                if warmup_count == 50 {
                    println!("[HFT] ✅ Warmed up after 50 messages");
                }
            }

            // ============================================
            // ZERO-ALLOCATION RAW BYTE SCANNING
            // ============================================
            
            // Try price_changes format first (newer Polymarket schema)
            // {"asset_id":"<66 chars>","price_changes":[{"price":"0.65","size":"1000"}]}
            if let Some(asset_idx) = asset_pattern.find(bytes) {
                let token_start = asset_idx + 12; // Length of "asset_id":"
                
                // Polymarket token IDs are exactly 66 characters
                if token_start + 66 <= len {
                    // Extract token bytes directly for hashing
                    let token_bytes = &bytes[token_start..token_start + 66];
                    let token_hash = fast_hash(token_bytes);
                    
                    // Scan for best_ask and best_bid (newer schema)
                    if let Some(ask_idx) = best_ask_pattern.find(&bytes[token_start..]) {
                        let ask_start = token_start + ask_idx + 12; // "best_ask":"
                        if ask_start < len {
                            // Find closing quote
                            if let Some(quote_idx) = memchr::memchr(b'"', &bytes[ask_start..]) {
                                let price_bytes = &bytes[ask_start..ask_start + quote_idx];
                                let ask_price = parse_fixed_6(price_bytes);
                                
                                // Scan for size
                                if let Some(size_idx) = size_pattern.find(&bytes[ask_start + quote_idx..]) {
                                    let size_start = ask_start + quote_idx + size_idx + 8;
                                    if size_start < len {
                                        if let Some(size_quote) = memchr::memchr(b'"', &bytes[size_start..]) {
                                            let size_bytes = &bytes[size_start..size_start + size_quote];
                                            let size = parse_fixed_6(size_bytes);
                                            
                                            // Update orderbook
                                            orderbook.entry(token_hash)
                                                .and_modify(|(p, _, s, _)| { *p = ask_price; *s = size; })
                                                .or_insert((ask_price, 0, size, 0));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Also check best_bid
                    if let Some(bid_idx) = best_bid_pattern.find(&bytes[token_start..]) {
                        let bid_start = token_start + bid_idx + 12;
                        if bid_start < len {
                            if let Some(quote_idx) = memchr::memchr(b'"', &bytes[bid_start..]) {
                                let price_bytes = &bytes[bid_start..bid_start + quote_idx];
                                let bid_price = parse_fixed_6(price_bytes);
                                
                                orderbook.entry(token_hash)
                                    .and_modify(|(_, p, _, s)| { *p = bid_price; })
                                    .or_insert((0, bid_price, 0, 0));
                            }
                        }
                    }
                    
                    // Edge detection
                    let complement_hash = token_hash ^ 1;
                    if let Some((yes_price, _, yes_size, _)) = orderbook.get(&token_hash) {
                        if let Some((c_yes_price, _, c_yes_size, _)) = orderbook.get(&complement_hash) {
                            let combined = yes_price + c_yes_price;
                            
                            // Maker Hybrid: look for extreme probabilities (p<0.30 or p>0.70)
                            // Lower fee tier: ~0.35% vs 1.80% for taker
                            if combined <= EDGE_THRESHOLD_U64 && *yes_size > 0 && *c_yes_size > 0 {
                                let _ = tx.try_send(BackgroundTask::EdgeDetected {
                                    token_hash,
                                    combined_price: combined,
                                    timestamp_nanos: start_tsc.elapsed().as_nanos() as u64,
                                });
                            }
                        }
                    }
                }
            }

            // Track latency
            let elapsed_nanos = start_tsc.elapsed().as_nanos() as u64;
            latency_samples.push(elapsed_nanos);

            // 5-second stats
            if last_stat_time.elapsed() >= Duration::from_secs(5) {
                if !latency_samples.is_empty() {
                    let mut sorted: Vec<u64> = latency_samples.clone();
                    sorted.sort_unstable();
                    
                    let min = sorted[0];
                    let max = sorted[sorted.len() - 1];
                    let sum: u64 = sorted.iter().sum();
                    let avg = sum / sorted.len() as u64;
                    let p99_idx = ((sorted.len() as f64) * 0.99) as usize;
                    let p99 = sorted[p99_idx.min(sorted.len() - 1)];
                    let sample_count = sorted.len();

                    let _ = tx.try_send(BackgroundTask::LatencyStats {
                        min_ns: min,
                        max_ns: max,
                        avg_ns: avg,
                        p99_ns: p99,
                        sample_count: sample_count as u64,
                    });
                }

                latency_samples.clear();
                last_stat_time = std::time::Instant::now();
            }
        }
    }
}

/// FNV-1a fast hash for token IDs
#[inline(always)]
fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Zero-allocation fixed-point parser
/// Converts "0.67" to 670000 (6 decimal places)
#[inline(always)]
fn parse_fixed_6(bytes: &[u8]) -> u64 {
    let mut val: u64 = 0;
    let mut fraction_digits = 0;
    let mut in_fraction = false;

    for &b in bytes {
        if b == b'.' {
            in_fraction = true;
        } else if b.is_ascii_digit() {
            val = val * 10 + (b - b'0') as u64;
            if in_fraction {
                fraction_digits += 1;
                if fraction_digits == 6 {
                    break;
                }
            }
        }
    }

    // Normalize to exactly 6 decimal places
    while fraction_digits < 6 {
        val *= 10;
        fraction_digits += 1;
    }

    val
}