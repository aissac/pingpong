// src/hft_hot_path.rs
//! HFT Hot Path - Zero-Allocation Raw Byte Scanner with memchr
//! CRITICAL FIX: Loop through price_changes array to parse ALL tokens

use crossbeam_channel::Sender;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

const EDGE_THRESHOLD_U64: u64 = 1_000_000;  // Temporarily $1.00 to see edges
const MAX_POSITION_U64: u64 = 5_000_000;

#[derive(Debug, Clone)]
pub enum BackgroundTask {
    EdgeDetected {
        token_hash: u64,
        combined_price: u64,
        timestamp_nanos: u64,
        yes_size: u64,
        no_size: u64,
    },
    LatencyStats {
        min_ns: u64,
        max_ns: u64,
        avg_ns: u64,
        p99_ns: u64,
        sample_count: u64,
    },
}

pub fn run_sync_hot_path(
    tx: Sender<BackgroundTask>,
    tokens: Vec<String>,
    killswitch: Arc<AtomicBool>,
    token_pairs: HashMap<u64, u64>,
) {
    use tungstenite::{connect, Message};
    
    println!("[HFT] Using memchr zero-allocation scanner (target: <1µs)");
    println!("[HFT] Token pairs loaded: {}", token_pairs.len());
    
    let ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
    
    let (mut socket, response) = match connect(ws_url) {
        Ok((s, r)) => (s, r),
        Err(e) => {
            eprintln!("❌ Failed to connect: {:?}", e);
            std::process::exit(1);
        }
    };
    
    println!("✅ Primary WebSocket connected (sync)");
    println!("HTTP status: {}", response.status());

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
    println!("[HFT] 🔥 Starting MEMCHR hot path (50-100ns target)...");

    let mut orderbook: HashMap<u64, (u64, u64, u64, u64)> = HashMap::with_capacity(128);
    let mut warmup_count = 0;
    let mut latency_samples: Vec<u64> = Vec::with_capacity(8192);
    let mut last_stat_time = std::time::Instant::now();
    
    static MSG_COUNTER: AtomicU64 = AtomicU64::new(0);
    static DEBUG_PRINTED: AtomicBool = AtomicBool::new(false);

    loop {
        if killswitch.load(Ordering::Relaxed) {
            println!("[HFT] 🚨 KILLSWITCH ENGAGED - exiting");
            return;
        }
        
        let msg = match socket.read() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[HFT] WS Read Error: {:?}", e);
                std::thread::sleep(Duration::from_secs(1));
                continue;
            }
        };

        match msg {
            Message::Text(text) => {
                let start_tsc = minstant::Instant::now();
                process_message(
                    text.as_bytes(),
                    &mut orderbook,
                    &tx,
                    &mut warmup_count,
                    &mut latency_samples,
                    &mut last_stat_time,
                    &MSG_COUNTER,
                    &DEBUG_PRINTED,
                    &token_pairs,
                );
            }
            Message::Binary(data) => {
                if let Ok(text) = std::str::from_utf8(&data) {
                    let start_tsc = minstant::Instant::now();
                    process_message(
                        text.as_bytes(),
                        &mut orderbook,
                        &tx,
                        &mut warmup_count,
                        &mut latency_samples,
                        &mut last_stat_time,
                        &MSG_COUNTER,
                        &DEBUG_PRINTED,
                        &token_pairs,
                    );
                }
            }
            Message::Ping(data) => { let _ = socket.write_message(Message::Pong(data)); }
            Message::Pong(_) => {}
            Message::Close(_) => { eprintln!("[HFT] Received Close frame"); return; }
            _ => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn process_message(
    bytes: &[u8],
    orderbook: &mut HashMap<u64, (u64, u64, u64, u64)>,
    tx: &Sender<BackgroundTask>,
    warmup_count: &mut u8,
    latency_samples: &mut Vec<u64>,
    last_stat_time: &mut std::time::Instant,
    msg_counter: &AtomicU64,
    debug_printed: &AtomicBool,
    token_pairs: &HashMap<u64, u64>,
) {
    use memchr::{memchr, memmem};
    
    let start_tsc = minstant::Instant::now();
    let len = bytes.len();

    if len < 100 { return; }

    if *warmup_count < 50 {
        *warmup_count += 1;
        if *warmup_count == 50 {
            println!("[HFT] ✅ Warmed up after 50 messages");
        }
    }
    
    let count = msg_counter.fetch_add(1, Ordering::Relaxed);
    if count < 2 && !debug_printed.load(Ordering::Relaxed) {
        let asset_pos = memmem::find(bytes, b"\"asset_id\":\"");
        let price_pos = memmem::find(bytes, b"\"price\":\"");
        println!("[DEBUG] Message {}: len={}, asset_id={:?}, price={:?}", count, len, asset_pos, price_pos);
        if count == 1 {
            debug_printed.store(true, Ordering::Relaxed);
        }
    }

    let asset_pattern = memmem::Finder::new(b"\"asset_id\":\"");
    let price_pattern = memmem::Finder::new(b"\"price\":\"");
    let size_pattern = memmem::Finder::new(b"\"size\":\"");
    
    let mut search_start = 0;
    let mut tokens_parsed = 0;
    
    // Loop through the entire byte array, finding EVERY asset_id
    while let Some(asset_idx) = asset_pattern.find(&bytes[search_start..]) {
        let token_start = search_start + asset_idx + 12;
        
        if token_start + 66 <= len {
            let token_bytes = &bytes[token_start..token_start + 66];
            let token_hash = fast_hash(token_bytes);
            
            let price_search_start = token_start;
            if let Some(price_idx) = price_pattern.find(&bytes[price_search_start..]) {
                let price_val_start = price_search_start + price_idx + 9;
                
                if price_val_start < len {
                    if let Some(price_end) = memchr(b'"', &bytes[price_val_start..]) {
                        let price_end_idx = price_val_start + price_end;
                        if price_end_idx <= len {
                            let price = parse_fixed_6(&bytes[price_val_start..price_end_idx]);
                            
                            let size_search_start = price_end_idx + 1;
                            if let Some(size_idx) = size_pattern.find(&bytes[size_search_start..]) {
                                let size_start = size_search_start + size_idx + 8;
                                
                                if size_start < len {
                                    if let Some(size_end) = memchr(b'"', &bytes[size_start..]) {
                                        let size_end_idx = size_start + size_end;
                                        if size_end_idx <= len {
                                            let size = parse_fixed_6(&bytes[size_start..size_end_idx]);
                                            
                                            orderbook.entry(token_hash)
                                                .and_modify(|(p, _, s, _)| { *p = price; *s = size; })
                                                .or_insert((price, 0, size, 0));
                                            
                                            tokens_parsed += 1;
                                            
                                            if let Some(&complement_hash) = token_pairs.get(&token_hash) {
                                                if let Some((c_yes_price, _, c_yes_size, _)) = orderbook.get(&complement_hash) {
                                                    if let Some((yes_price, _, yes_size, _)) = orderbook.get(&token_hash) {
                                                        let combined = yes_price + c_yes_price;
                                                        
                                                        static COMBINED_COUNTER: AtomicU64 = AtomicU64::new(0);
                                                        let cc = COMBINED_COUNTER.fetch_add(1, Ordering::Relaxed);
                                                        if cc % 100 == 0 {
                                                            println!("[DEBUG] Combined: ${:.4} (threshold: $1.00) | token_hash={:016x}", 
                                                                combined as f64 / 1_000_000.0, token_hash);
                                                        }
                                                        
                                                        if combined <= EDGE_THRESHOLD_U64 && *yes_size > 0 && *c_yes_size > 0 {
                                                            println!("[EDGE] 🔥 FOUND! combined=${:.4} yes_size={} no_size={}", 
                                                                combined as f64 / 1_000_000.0, yes_size, c_yes_size);
                                                            
                                                            let capped_yes = std::cmp::min(*yes_size, MAX_POSITION_U64);
                                                            let capped_no = std::cmp::min(*c_yes_size, MAX_POSITION_U64);
                                                            
                                                            let _ = tx.try_send(BackgroundTask::EdgeDetected {
                                                                token_hash,
                                                                combined_price: combined,
                                                                timestamp_nanos: start_tsc.elapsed().as_nanos() as u64,
                                                                yes_size: capped_yes,
                                                                no_size: capped_no,
                                                            });
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
            }
        }
        
        search_start = token_start + 66;
    }
    
    if tokens_parsed > 0 {
        static PARSE_COUNTER: AtomicU64 = AtomicU64::new(0);
        let pc = PARSE_COUNTER.fetch_add(1, Ordering::Relaxed);
        if pc % 50 == 0 {
            println!("[DEBUG] Parsed {} tokens from message (orderbook size: {})", tokens_parsed, orderbook.len());
        }
    }

    let elapsed_nanos = start_tsc.elapsed().as_nanos() as u64;
    latency_samples.push(elapsed_nanos);

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

            let _ = tx.try_send(BackgroundTask::LatencyStats {
                min_ns: min, max_ns: max, avg_ns: avg, p99_ns: p99, sample_count: sorted.len() as u64,
            });
        }
        
        // DIAGNOSTIC: Dump orderbook state every 5 seconds
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📊 ORDERBOOK DIAGNOSTIC ({} entries)", orderbook.len());
        println!("🔑 Token pairs map: {} entries", token_pairs.len());
        
        println!("📦 Orderbook entries (non-zero size):");
        let mut shown = 0;
        for (hash, (_, _, size, _)) in orderbook.iter() {
            if *size > 0 && shown < 15 {
                println!("  {:016x}: size={}", hash, *size);
                shown += 1;
            }
        }
        
        let mut populated_pairs = 0;
        let mut missing_yes = 0;
        let mut missing_no = 0;
        let mut empty_yes = 0;
        let mut empty_no = 0;
        
        for (yes_hash, no_hash) in token_pairs.iter() {
            let has_yes = orderbook.contains_key(yes_hash);
            let has_no = orderbook.contains_key(no_hash);
            let yes_size = orderbook.get(yes_hash).map(|(_, _, s, _)| *s).unwrap_or(0);
            let no_size = orderbook.get(no_hash).map(|(_, _, s, _)| *s).unwrap_or(0);
            
            if yes_size > 0 && no_size > 0 {
                populated_pairs += 1;
            } else {
                if !has_yes { missing_yes += 1; }
                else if yes_size == 0 { empty_yes += 1; }
                if !has_no { missing_no += 1; }
                else if no_size == 0 { empty_no += 1; }
            }
        }
        
        println!("📊 Pair status:");
        println!("  Fully populated: {}/12", populated_pairs);
        println!("  Missing YES entirely: {}", missing_yes);
        println!("  Missing NO entirely: {}", missing_no);
        println!("  YES exists but size=0: {}", empty_yes);
        println!("  NO exists but size=0: {}", empty_no);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        latency_samples.clear();
        *last_stat_time = std::time::Instant::now();
    }
}

#[inline(always)]
fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
    hash
}

#[inline(always)]
fn parse_fixed_6(bytes: &[u8]) -> u64 {
    let mut val: u64 = 0;
    let mut digits = 0;
    let mut in_frac = false;
    for &b in bytes {
        if b == b'.' { in_frac = true; }
        else if b.is_ascii_digit() {
            val = val * 10 + (b - b'0') as u64;
            if in_frac { digits += 1; if digits == 6 { break; } }
        }
    }
    while digits < 6 { val *= 10; digits += 1; }
    val
}