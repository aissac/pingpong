#!/usr/bin/env python3
"""Restore working hft_hot_path.rs with minimal debug"""

code = r'''//! HFT Hot Path - Rate Limited + WebSocket Reading + price_changes support

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Instant, Duration};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use crossbeam_channel::{Sender, Receiver};
use memchr::memchr;
use memchr::memmem;
use rustc_hash::FxHashMap;
use tungstenite::Message;
use crate::websocket_reader::WebSocketReader;

const EVAL_RATE_LIMIT_MS: u64 = 100;
const EDGE_THRESHOLD_U64: u64 = 980_000;
const MIN_VALID_COMBINED_U64: u64 = 900_000;
const TARGET_SHARES: u64 = 100;

pub enum RolloverCommand {
    AddPair(u64, u64),
    RemovePair(u64),
}

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
        p99_ns: u64,
        sample_count: u64,
    },
}

pub struct EvalTracker {
    last_eval: Instant,
}

impl EvalTracker {
    pub fn new() -> Self {
        Self {
            last_eval: Instant::now() - Duration::from_secs(1),
        }
    }

    pub fn can_evaluate(&mut self, now: Instant) -> bool {
        if now.duration_since(self.last_eval).as_millis() as u64 >= EVAL_RATE_LIMIT_MS {
            self.last_eval = now;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenBookState {
    pub best_bid_price: u64,
    pub best_bid_size: u64,
    pub best_ask_price: u64,
    pub best_ask_size: u64,
}

impl TokenBookState {
    pub fn new() -> Self {
        Self {
            best_bid_price: 0,
            best_bid_size: 0,
            best_ask_price: u64::MAX,
            best_ask_size: 0,
        }
    }

    pub fn update_bid(&mut self, price: u64, size: u64) {
        if price > self.best_bid_price {
            self.best_bid_price = price;
            self.best_bid_size = size;
        }
    }

    pub fn update_ask(&mut self, price: u64, size: u64) {
        if price < self.best_ask_price {
            self.best_ask_price = price;
            self.best_ask_size = size;
        }
    }

    pub fn get_best_bid(&self) -> Option<(u64, u64)> {
        if self.best_bid_price > 0 {
            Some((self.best_bid_price, self.best_bid_size))
        } else {
            None
        }
    }

    pub fn get_best_ask(&self) -> Option<(u64, u64)> {
        if self.best_ask_price < u64::MAX {
            Some((self.best_ask_price, self.best_ask_size))
        } else {
            None
        }
    }
}

pub fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

fn parse_fixed_6(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut decimal_seen = false;
    let mut decimal_places = 0;

    for &b in bytes {
        if b == b'.' {
            decimal_seen = true;
            continue;
        }
        if b >= b'0' && b <= b'9' {
            result = result * 10 + (b - b'0') as u64;
            if decimal_seen {
                decimal_places += 1;
            }
        }
    }

    while decimal_places < 6 {
        result *= 10;
        decimal_places += 1;
    }

    result
}

pub fn run_sync_hot_path(
    mut ws_stream: WebSocketReader,
    opportunity_tx: Sender<BackgroundTask>,
    all_tokens: Vec<String>,
    killswitch: Arc<AtomicBool>,
    mut token_pairs: HashMap<u64, u64>,
    edge_counter: Arc<AtomicU64>,
    rollover_rx: Receiver<RolloverCommand>,
) {
    println!("⚡ Rate-Limited Hot Path Started");
    println!("📊 Tracking {} token pairs", token_pairs.len());

    let mut orderbook: FxHashMap<u64, TokenBookState> = FxHashMap::default();
    let mut eval_trackers: FxHashMap<u64, EvalTracker> = FxHashMap::default();
    for &token_hash in token_pairs.keys() {
        eval_trackers.insert(token_hash, EvalTracker::new());
    }

    for token in &all_tokens {
        let hash = fast_hash(token.as_bytes());
        orderbook.entry(hash).or_insert_with(TokenBookState::new);
    }

    let mut messages = 0u64;
    let mut total_evals = 0u64;
    let mut edges_found = 0u64;
    let start = Instant::now();
    let mut last_report = Instant::now();
    let mut last_eval_count = 0u64;

    println!("⚡ Hot Path Armed. Waiting for WebSocket events...");

    loop {
        if killswitch.load(Ordering::Relaxed) {
            println!("⚡ Killswitch triggered, exiting hot path");
            break;
        }

        while let Ok(cmd) = rollover_rx.try_recv() {
            match cmd {
                RolloverCommand::AddPair(yes_hash, no_hash) => {
                    println!("[ROLLOVER] Adding pair: YES={} NO={}", yes_hash, no_hash);
                    token_pairs.insert(yes_hash, no_hash);
                    token_pairs.insert(no_hash, yes_hash);
                    eval_trackers.insert(yes_hash, EvalTracker::new());
                    eval_trackers.insert(no_hash, EvalTracker::new());
                    orderbook.entry(yes_hash).or_insert_with(TokenBookState::new);
                    orderbook.entry(no_hash).or_insert_with(TokenBookState::new);
                }
                RolloverCommand::RemovePair(yes_hash) => {
                    println!("[ROLLOVER] Removing pair: YES={}", yes_hash);
                    if let Some(no_hash) = token_pairs.remove(&yes_hash) {
                        token_pairs.remove(&no_hash);
                        eval_trackers.remove(&yes_hash);
                        eval_trackers.remove(&no_hash);
                    }
                }
            }
        }

        let msg = match ws_stream.socket.read() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("🚨 WebSocket read error: {}", e);
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };

        messages += 1;

        if let Message::Text(text) = msg {
            let bytes = text.as_bytes();
            parse_and_update_orderbook(bytes, &mut orderbook, &token_pairs, messages);

            let pairs: Vec<(u64, u64)> = token_pairs.iter()
                .map(|(&k, &v)| (k, v))
                .collect();
            
            for (token_hash, complement_hash) in pairs {
                let now = Instant::now();
                if let Some(tracker) = eval_trackers.get_mut(&token_hash) {
                    if !tracker.can_evaluate(now) {
                        continue;
                    }
                    total_evals += 1;
                }

                if let (Some(yes_state), Some(no_state)) = 
                    (orderbook.get(&token_hash), orderbook.get(&complement_hash)) {
                    
                    if let (Some((yes_ask_price, yes_ask_size)), 
                            Some((no_ask_price, no_ask_size))) = 
                        (yes_state.get_best_ask(), no_state.get_best_ask()) {
                        
                        if yes_ask_price == 0 || yes_ask_price >= 100_000_000 || 
                           no_ask_price == 0 || no_ask_price >= 100_000_000 {
                            continue;
                        }

                        if yes_ask_size < TARGET_SHARES || no_ask_size < TARGET_SHARES {
                            continue;
                        }

                        let combined_ask = yes_ask_price + no_ask_price;

                        if combined_ask <= EDGE_THRESHOLD_U64 && combined_ask >= MIN_VALID_COMBINED_U64 {
                            edges_found += 1;
                            edge_counter.fetch_add(1, Ordering::Relaxed);

                            println!("🎯 [EDGE] Combined ASK=${:.4} (YES=${} NO=${})", 
                                combined_ask as f64 / 1_000_000.0,
                                yes_ask_price as f64 / 1_000_000.0,
                                no_ask_price as f64 / 1_000_000.0);

                            let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {
                                yes_token_hash: token_hash,
                                no_token_hash: complement_hash,
                                yes_best_bid: yes_state.best_bid_price,
                                yes_best_ask: yes_ask_price,
                                yes_ask_size: yes_ask_size,
                                no_best_bid: no_state.best_bid_price,
                                no_best_ask: no_ask_price,
                                no_ask_size: no_ask_size,
                                combined_ask,
                                timestamp_nanos: 0,
                            });
                        }
                    }
                }
            }
        }

        if last_report.elapsed() >= Duration::from_secs(1) {
            let evals_this_sec = total_evals - last_eval_count;
            println!("[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {} | pairs:{}",
                start.elapsed().as_secs(),
                messages,
                total_evals,
                edges_found,
                evals_this_sec,
                token_pairs.len()
            );
            last_eval_count = total_evals;
            last_report = Instant::now();
        }
    }

    let elapsed = start.elapsed();
    println!("[HFT] Processed {} messages in {:?}", messages, elapsed);
    println!("[HFT] Total evaluations: {} | Edges found: {}", total_evals, edges_found);
}

fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
    messages: u64,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut pos = 0;

    while pos < bytes.len() {
        let remaining = &bytes[pos..];

        if remaining.starts_with(b"\"asset_id\":\"") {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'"', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }

        if remaining.starts_with(b"\"side\":\"BUY\"") {
            is_bid = true;
        } else if remaining.starts_with(b"\"side\":\"SELL\"") {
            is_bid = false;
        }

        if remaining.starts_with(b"\"price\":\"") {
            let price_start = pos + 9;
            if let Some(price_end) = memchr(b'"', &bytes[price_start..]) {
                let price = parse_fixed_6(&bytes[price_start..price_start + price_end]);
                
                if let Some(token_hash) = current_token_hash {
                    if let Some(state) = orderbook.get_mut(&token_hash) {
                        if is_bid {
                            state.update_bid(price, 100);
                        } else {
                            state.update_ask(price, 100);
                        }
                    }
                }
                pos = price_start + price_end + 1;
                continue;
            }
        }

        pos += 1;
    }
}
'''

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(code)

print('Restored clean working version')
