//! HFT Hot Path - Proper Bid/Ask Tracking
//!
//! CRITICAL FIX: Track both sides of the orderbook
//! - best_bid: Highest BUY order price  
//! - best_ask: Lowest SELL order price
//! - Combined ASK = YES Ask + NO Ask (what we PAY to arbitrage)

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crossbeam_channel::Sender;
use memchr::memchr;
use memchr::memmem;
use std::time::Instant;

/// Token orderbook state - tracks both bid and ask
#[derive(Clone, Debug, Default)]
pub struct TokenBookState {
    pub best_bid_price: u64,  // Highest buy price (fixed-point * 1,000,000)
    pub best_bid_size: u64,   // Size at best bid
    pub best_ask_price: u64,   // Lowest sell price (fixed-point * 1,000,000)
    pub best_ask_size: u64,    // Size at best ask
}

/// Fast hash for token IDs
pub fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Parse fixed-point price (multiply by 1,000,000 for 6 decimal precision)
fn parse_fixed_6(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut decimal_seen = false;
    let mut decimal_places = 0u32;
    
    for &b in bytes {
        match b {
            b'.' => decimal_seen = true,
            b'0'..=b'9' => {
                result = result * 10 + (b - b'0') as u64;
                if decimal_seen { decimal_places += 1; }
            }
            _ => {}
        }
    }
    
    // Adjust to 6 decimal places
    while decimal_places < 6 {
        result *= 10;
        decimal_places += 1;
    }
    result
}

/// Edge threshold constants
const EDGE_THRESHOLD_U64: u64 = 980_000;   // $0.98 for DRY_RUN validation
const MIN_VALID_COMBINED_U64: u64 = 900_000; // $0.90 minimum (filter bad data)
const MAX_POSITION_U64: u64 = 5_000_000;    // $5 max position

/// Edge detection result
pub struct EdgeDetected {
    pub yes_token: u64,
    pub no_token: u64,
    pub yes_bid: u64,
    pub yes_ask: u64,
    pub no_bid: u64,
    pub no_ask: u64,
    pub combined_ask: u64,  // YES Ask + NO Ask (TRUE cost to arbitrage)
    pub yes_size: u64,
    pub no_size: u64,
}

/// Run the hot path - ZERO-ALLOCATION memchr scanner
pub fn run_sync_hot_path(
    ws_stream: Box<dyn std::io::Read>,
    opportunity_tx: Sender<EdgeDetected>,
    all_tokens: Vec<String>,
    killswitch: Arc<AtomicBool>,
    token_pairs: HashMap<u64, u64>,
    edge_counter: Arc<AtomicU64>,
) {
    // Orderbook: token_hash -> (bid_price, bid_size, ask_price, ask_size)
    let mut orderbook: HashMap<u64, (u64, u64, u64, u64)> = HashMap::new();
    
    // Pre-populate with known tokens
    for token in &all_tokens {
        orderbook.entry(fast_hash(token.as_bytes()))
            .or_insert((0, 0, u64::MAX, 0));
    }
    
    // Patterns for memchr scanning
    let asset_pattern = memmem::Finder::new(b"\"asset_id\":\"");
    let price_pattern = memmem::Finder::new(b"\"price\":\"");
    let size_pattern = memmem::Finder::new(b"\"size\":\"");
    let bids_start_pattern = memmem::Finder::new(b"\"bids\":");
    let asks_start_pattern = memmem::Finder::new(b"\"asks\":");
    let book_end_pattern = memmem::Finder::new(b"}]");
    
    let mut buffer = vec![0u8; 1024 * 1024];  // 1MB buffer
    let mut total_bytes = 0;
    let mut messages = 0u64;
    let start = Instant::now();
    
    println!("[HFT] 🔥 Starting MEMCHR hot path with BID/ASK tracking (target: <1µs)");
    println!("[HFT] Token pairs from Gamma API: {} pairs", token_pairs.len());
    
    // Simulated WebSocket read loop (actual impl reads from tungstenite)
    loop {
        if killswitch.load(Ordering::Relaxed) {
            println!("[HFT] Killswitch triggered, shutting down");
            break;
        }
        
        // Read message (simplified - actual code reads from WebSocket)
        let n = match ws_stream.read(&mut buffer[total_bytes..]) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        total_bytes += n;
        
        // Find the "bids" and "asks" section boundaries
        let bytes = &buffer[..total_bytes];
        let bids_section = bids_start_pattern.find(bytes);
        let asks_section = asks_start_pattern.find(bytes);
        
        // Determine if we're in bids or asks based on position
        // After "bids": and before "asks": → BID
        // After "asks": → ASK
        
        // Parse all tokens in this message
        let mut search_start = 0;
        let mut tokens_parsed = 0;
        let mut is_bid = false;
        
        while let Some(asset_idx) = asset_pattern.find(&bytes[search_start..]) {
            let token_start = search_start + asset_idx + 12;
            
            if let Some(token_end) = memchr(b'"', &bytes[token_start..]) {
                let token_bytes = &bytes[token_start..token_start + token_end];
                let token_hash = fast_hash(token_bytes);
                
                // Determine side based on position in message
                if let Some(bid_pos) = bids_section {
                    if let Some(ask_pos) = asks_section {
                        let current_pos = search_start + asset_idx;
                        is_bid = current_pos > bid_pos && current_pos < ask_pos;
                    }
                }
                
                // Find price
                let price_search_start = token_start + token_end + 1;
                if let Some(price_idx) = price_pattern.find(&bytes[price_search_start..]) {
                    let price_val_start = price_search_start + price_idx + 9;
                    
                    if let Some(price_end) = memchr(b'"', &bytes[price_val_start..]) {
                        let price = parse_fixed_6(&bytes[price_val_start..price_val_start + price_end]);
                        
                        // Find size
                        let size_search_start = price_val_start + price_end + 1;
                        if let Some(size_idx) = size_pattern.find(&bytes[size_search_start..]) {
                            let size_start = size_search_start + size_idx + 8;
                            
                            if let Some(size_end) = memchr(b'"', &bytes[size_start..]) {
                                let size = parse_fixed_6(&bytes[size_start..size_start + size_end]);
                                
                                // Update orderbook based on side
                                if size > 0 {
                                    orderbook.entry(token_hash)
                                        .and_modify(|(bid_p, bid_s, ask_p, ask_s)| {
                                            if is_bid {
                                                // BID: Track highest buy price
                                                if bid_p.is_zero() || price > *bid_p {
                                                    *bid_p = price;
                                                    *bid_s = size;
                                                }
                                            } else {
                                                // ASK: Track lowest sell price
                                                if ask_p.is_null() || price < *ask_p {
                                                    *ask_p = price;
                                                    *ask_s = size;
                                                }
                                            }
                                        })
                                        .or_insert(if is_bid {
                                            (price, size, u64::MAX, 0)
                                        } else {
                                            (0, 0, price, size)
                                        });
                                }
                                
                                tokens_parsed += 1;
                                
                                // Check for edge after updating orderbook
                                if let Some(&complement_hash) = token_pairs.get(&token_hash) {
                                    if let (Some((yes_bid, _, yes_ask, _)), Some((no_bid, _, no_ask, _))) =
                                        (orderbook.get(&token_hash), orderbook.get(&complement_hash)) {
                                        
                                        // Calculate TRUE Combined ASK (what we PAY)
                                        let combined_ask = yes_ask.saturating_add(*no_ask);
                                        
                                        // Edge detection: Combined ASK must be below threshold
                                        if combined_ask <= EDGE_THRESHOLD_U64 
                                            && combined_ask >= MIN_VALID_COMBINED_U64
                                            && !yes_ask.is_null() 
                                            && !no_ask.is_null() {
                                            
                                            let ec = edge_counter.fetch_add(1, Ordering::Relaxed);
                                            if ec < 10 || ec % 100 == 0 {
                                                println!("[EDGE] 🎯 FOUND! Combined ASK = ${:.4} (YES Ask=${:.4}, NO Ask=${:.4})", 
                                                    combined_ask as f64 / 1_000_000.0,
                                                    *yes_ask as f64 / 1_000_000.0,
                                                    *no_ask as f64 / 1_000_000.0);
                                            }
                                            
                                            let capped_yes = std::cmp::min(*yes_ask, MAX_POSITION_U64);
                                            let capped_no = std::cmp::min(*no_ask, MAX_POSITION_U64);
                                            
                                            let _ = opportunity_tx.send(EdgeDetected {
                                                yes_token: token_hash,
                                                no_token: complement_hash,
                                                yes_bid: *yes_bid,
                                                yes_ask: *yes_ask,
                                                no_bid: *no_bid,
                                                no_ask: *no_ask,
                                                combined_ask,
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
                
                search_start = token_start + 1;
            } else {
                break;
            }
        }
        
        messages += 1;
        total_bytes = 0;  // Reset for next message
        
        if messages == 50 && !debug_printed.swap(false, Ordering::Relaxed) {
            println!("[HFT] ✅ Warmed up after {} messages", messages);
        }
    }
    
    let elapsed = start.elapsed();
    println!("[HFT] Processed {} messages in {:?}", messages, elapsed);
}