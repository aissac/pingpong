//! HFT Hot Path - Event-Driven with Rate Limiting
//!
//! Only evaluates edges when PriceChange events arrive (not free-spinning loop)
//! Implements 100ms rate limiting per market to prevent spam

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, Duration};
use memchr::memchr;
use memchr::memmem;
use rustc_hash::FxHashMap; // Faster than HashMap for HFT
use crossbeam_channel::{Sender, Receiver, TryRecvError};

use crate::state::{TokenBookState, parse_fixed_6};
use crate::websocket_reader::WebSocketReader;
use crate::market_rollover::{build_subscribe_message, build_unsubscribe_message};

/// Hash token bytes consistently
#[inline]
fn fast_hash_token(bytes: &[u8]) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    let token_str = std::str::from_utf8(bytes).unwrap_or("");
    token_str.hash(&mut hasher);
    hasher.finish()
}

const EDGE_THRESHOLD_U64: u64 = 940_000;   // $0.94 in micro-dollars
const MIN_VALID_COMBINED_U64: u64 = 900_000; // $0.90 floor
const MAX_POSITION_U64: u64 = 5_000_000;
const TARGET_SHARES: u64 = 100;
const EVAL_RATE_LIMIT_MS: u64 = 100; // Only evaluate each market once per 100ms

/// Task sent to background execution thread
pub enum BackgroundTask {
    EdgeDetected {
        yes_token_hash: u64,
        no_token_hash: u64,
        yes_best_bid: u64,
        yes_best_ask: u64,
        yes_ask_size: u64,
        no_best_bid: u64,
        no_best_ask: u64,
        no_ask_size: u64,
        combined_ask: u64,
        timestamp_nanos: u64,
    },
    LatencyStats {
        min_ns: u64,
        max_ns: u64,
        avg_ns: u64,
        count: u64,
    },
}

/// Track evaluation times for rate limiting
struct EvalTracker {
    last_eval: Instant,
    eval_count: u64,
}

impl EvalTracker {
    fn new() -> Self {
        Self {
            last_eval: Instant::now() - Duration::from_secs(1), // Allow immediate first eval
            eval_count: 0,
        }
    }
    
    fn can_evaluate(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_eval) >= Duration::from_millis(EVAL_RATE_LIMIT_MS) {
            self.last_eval = now;
            self.eval_count += 1;
            true
        } else {
            false
        }
    }
}

/// Run event-driven hot path with rate limiting
pub fn run_sync_hot_path(
    mut ws_reader: WebSocketReader,
    token_pairs: Arc<HashMap<u64, u64>>,
    hash_to_id: Arc<HashMap<u64, String>>,
    id_to_condition: Arc<HashMap<String, String>>,
    killswitch: Arc<AtomicBool>,
    opportunity_tx: Sender<BackgroundTask>,
    rollover_rx: Receiver<String>,
) {
    println!("⚡ Event-Driven Hot Path Started (Rate Limit: {}ms)", EVAL_RATE_LIMIT_MS);
    println!("📊 Tracking {} token pairs", token_pairs.len());
    
    // Local orderbook state (zero-allocation, stack-allocated)
    let mut orderbook: FxHashMap<u64, TokenBookState> = FxHashMap::default();
    
    // Rate limiting tracker per token
    let mut eval_trackers: FxHashMap<u64, EvalTracker> = FxHashMap::default();
    
    // Metrics
    let start = Instant::now();
    let mut messages = 0;
    let mut total_evals = 0;
    let mut edges_found = 0;
    let mut last_report = Instant::now();
    let mut last_eval_count = 0;
    
    // Initialize rate limiters for all tokens
    for &token_hash in token_pairs.keys() {
        eval_trackers.insert(token_hash, EvalTracker::new());
    }
    
    println!("⚡ Hot Path Armed. Waiting for WebSocket events...");
    
    // Main event-driven loop
    loop {
        if killswitch.load(Ordering::Relaxed) {
            println!("⚡ Killswitch triggered, exiting hot path");
            break;
        }
        
        // Read WebSocket message (blocking, but that's OK - we want to wait for events)
        let mut buf = [0u8; 65536];
        match ws_reader.read(&mut buf) {
            Ok(0) => {
                println!("🚨 WebSocket closed, exiting hot path");
                break;
            }
            Ok(len) => {
                messages += 1;
                let bytes = &buf[..len];
                
                // Parse WebSocket message and update orderbook
                parse_and_update_orderbook(bytes, &mut orderbook, &token_pairs);
                
                // Extract token IDs that changed from the message
                let changed_tokens = extract_changed_tokens(bytes);
                
                // Only evaluate markets that actually changed
                for token_hash in changed_tokens {
                    // Check rate limit
                    if let Some(tracker) = eval_trackers.get_mut(&token_hash) {
                        if tracker.can_evaluate() {
                            total_evals += 1;
                            
                            // Evaluate this market for edges
                            if let Some(complement_hash) = token_pairs.get(&token_hash) {
                                if let Some(edge) = evaluate_edge(
                                    token_hash,
                                    *complement_hash,
                                    &orderbook,
                                ) {
                                    edges_found += 1;
                                    
                                    // Send edge to execution thread
                                    let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {
                                        yes_token_hash: token_hash,
                                        no_token_hash: *complement_hash,
                                        yes_best_bid: edge.0,
                                        yes_best_ask: edge.1,
                                        yes_ask_size: edge.2,
                                        no_best_bid: edge.3,
                                        no_best_ask: edge.4,
                                        no_ask_size: edge.5,
                                        combined_ask: edge.6,
                                        timestamp_nanos: 0,
                                    });
                                }
                            }
                        }
                    }
                }
                
                // Handle rollover commands
                if let Ok(cmd) = rollover_rx.try_recv() {
                    let _ = ws_reader.send(cmd);
                }
            }
            Err(e) => {
                eprintln!("🚨 WebSocket read error: {}", e);
                break;
            }
        }
        
        // Periodic metrics report (every 1 second)
        if last_report.elapsed() >= Duration::from_secs(1) {
            let evals_this_sec = total_evals - last_eval_count;
            println!("[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {}",
                start.elapsed().as_secs(),
                messages,
                total_evals,
                edges_found,
                evals_this_sec
            );
            last_eval_count = total_evals;
            last_report = Instant::now();
        }
    }
    
    let elapsed = start.elapsed();
    println!("[HFT] Processed {} messages in {:?}", messages, elapsed);
    println!("[HFT] Total evaluations: {} | Edges found: {}", total_evals, edges_found);
}

/// Parse WebSocket message and update orderbook
fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
) {
    // Simple JSON parsing to extract token updates
    // (Implementation depends on your actual WebSocket message format)
    
    // For now, this is a placeholder - you'll need to implement
    // the actual parsing based on Polymarket's WebSocket message format
}

/// Extract token IDs that changed from WebSocket message
fn extract_changed_tokens(bytes: &[u8]) -> Vec<u64> {
    // Extract token IDs from the message
    // Return list of tokens that had price/size changes
    
    // Placeholder implementation
    Vec::new()
}

/// Evaluate a single market for edge opportunities
fn evaluate_edge(
    token_hash: u64,
    complement_hash: u64,
    orderbook: &FxHashMap<u64, TokenBookState>,
) -> Option<(u64, u64, u64, u64, u64, u64, u64)> {
    if let (Some(yes_state), Some(no_state)) = 
        (orderbook.get(&token_hash), orderbook.get(&complement_hash)) {
        
        if let (Some((yes_ask_price, yes_ask_size)), 
                Some((no_ask_price, no_ask_size))) = 
            (yes_state.get_best_ask(), no_state.get_best_ask()) {
            
            // Sanity checks
            if yes_ask_price == 0 || yes_ask_price >= 100 || 
               no_ask_price == 0 || no_ask_price >= 100 {
                return None;
            }
            
            if yes_ask_size < TARGET_SHARES || no_ask_size < TARGET_SHARES {
                return None;
            }
            
            let combined_ask = yes_ask_price * 10_000 + no_ask_price * 10_000;
            
            if combined_ask <= EDGE_THRESHOLD_U64 && combined_ask >= MIN_VALID_COMBINED_U64 {
                return Some((
                    yes_state.get_best_bid().map(|(p, _)| p).unwrap_or(0),
                    yes_ask_price,
                    yes_ask_size,
                    no_state.get_best_bid().map(|(p, _)| p).unwrap_or(0),
                    no_ask_price,
                    no_ask_size,
                    combined_ask,
                ));
            }
        }
    }
    None
}
