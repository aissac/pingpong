//! Hot Path - Zero-Allocation Market Data Processing
//!
//! Implements:
//! - Sub-microsecond price updates via crossbeam
//! - Edge detection (YES_ask + NO_ask < threshold)
//! - Bridge to async API client
//! - Integration with quote manager

use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::network::ws_engine::WsEvent;

/// Combined price threshold for arbitrage ($0.94 = 940,000 micro-USDC)
const EDGE_THRESHOLD_U64: u64 = 940_000;
/// Minimum valid combined price ($0.90 = 900,000 micro-USDC)
const MIN_VALID_COMBINED_U64: u64 = 900_000;
/// Maximum position size (5 USDC = 5,000,000 micro-USDC)
const MAX_POSITION_U64: u64 = 5_000_000;

/// Background task from hot path
#[derive(Debug, Clone)]
pub enum BackgroundTask {
    /// Edge detected - combined price below threshold
    EdgeDetected {
        yes_token_hash: u64,
        no_token_hash: u64,
        yes_token_id: String,
        no_token_id: String,
        condition_id: String,
        combined_ask: u64,
        yes_ask_price: u64,
        yes_ask_size: u64,
        no_ask_price: u64,
        no_ask_size: u64,
        timestamp_nanos: u64,
    },
    /// Latency statistics
    LatencyStats {
        min_ns: u64,
        max_ns: u64,
        avg_ns: u64,
        p99_ns: u64,
        sample_count: u64,
    },
    /// Order matched event
    OrderMatched {
        order_id: String,
        token_id: String,
        matched_size: u64,
        price: u64,
    },
}

/// Hot path state
pub struct HotPath {
    /// Token hash -> (bid_price, bid_size, ask_price, ask_size)
    orderbook: HashMap<u64, (u64, u64, u64, u64)>,
    /// YES token hash -> NO token hash mapping
    token_pairs: HashMap<u64, u64>,
    /// Hash -> Token ID mapping
    hash_to_id: HashMap<u64, String>,
    /// Token ID -> Condition ID mapping
    id_to_condition: HashMap<String, String>,
    /// Edge threshold
    edge_threshold: u64,
    /// Minimum valid combined
    min_valid_combined: u64,
    /// Max position
    max_position: u64,
    /// Warmup counter
    warmup_count: u8,
    /// Latency samples
    latency_samples: Vec<u64>,
    /// Last stats time
    last_stat_time: Instant,
    /// Message counter
    msg_counter: Arc<AtomicU64>,
    /// Edge counter
    edge_counter: Arc<AtomicU64>,
    /// Kill switch
    killswitch: Arc<AtomicBool>,
}

impl HotPath {
    /// Create a new hot path processor
    pub fn new(
        hash_to_id: HashMap<u64, String>,
        id_to_condition: HashMap<String, String>,
        token_pairs: HashMap<u64, u64>,
        killswitch: Arc<AtomicBool>,
    ) -> Self {
        Self {
            orderbook: HashMap::with_capacity(256),
            token_pairs,
            hash_to_id,
            id_to_condition,
            edge_threshold: EDGE_THRESHOLD_U64,
            min_valid_combined: MIN_VALID_COMBINED_U64,
            max_position: MAX_POSITION_U64,
            warmup_count: 0,
            latency_samples: Vec::with_capacity(8192),
            last_stat_time: Instant::now(),
            msg_counter: Arc::new(AtomicU64::new(0)),
            edge_counter: Arc::new(AtomicU64::new(0)),
            killswitch,
        }
    }

    /// Process WebSocket event from async engine
    pub fn process_ws_event(
        &mut self,
        event: WsEvent,
        task_tx: &Sender<BackgroundTask>,
    ) {
        match event {
            WsEvent::BookUpdate {
                token_hash,
                token_id: _,
                bid_price,
                bid_size,
                ask_price,
                ask_size,
                timestamp_nanos,
            } => {
                self.process_book_update(
                    token_hash,
                    bid_price,
                    bid_size,
                    ask_price,
                    ask_size,
                    timestamp_nanos,
                    task_tx,
                );
            }
            WsEvent::Trade {
                token_hash,
                token_id: _,
                price,
                size,
                side: _,
                timestamp_nanos,
            } => {
                self.process_trade(token_hash, price, size, timestamp_nanos);
            }
            WsEvent::Connected => {
                println!("[HOT_PATH] ✅ WebSocket connected");
            }
            WsEvent::Disconnected { reason } => {
                println!("[HOT_PATH] ⚠️ WebSocket disconnected: {}", reason);
            }
            WsEvent::OrderMatched {
                order_id,
                token_id,
                matched_size,
                price,
            } => {
                self.process_order_matched(order_id, token_id, matched_size, price, task_tx);
            }
        }
    }

    /// Process orderbook update
    fn process_book_update(
        &mut self,
        token_hash: u64,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64,
        timestamp_nanos: u64,
        task_tx: &Sender<BackgroundTask>,
    ) {
        let start = Instant::now();
        
        // Update orderbook
        self.orderbook.insert(token_hash, (bid_price, bid_size, ask_price, ask_size));
        
        // Increment message counter
        self.msg_counter.fetch_add(1, Ordering::Relaxed);
        
        // Skip edge detection during warmup
        if self.warmup_count < 10 {
            self.warmup_count += 1;
            return;
        }
        
        // Check for complement (YES/NO pair)
        if let Some(&complement_hash) = self.token_pairs.get(&token_hash) {
            if let Some(&(comp_bid, comp_bid_size, comp_ask, comp_ask_size)) = self.orderbook.get(&complement_hash) {
                // Calculate combined ask
                let combined_ask = ask_price.saturating_add(comp_ask);
                
                // Check for edge
                if combined_ask < self.edge_threshold && combined_ask > self.min_valid_combined {
                    // Edge detected!
                    self.edge_counter.fetch_add(1, Ordering::Relaxed);
                    
                    // Get token IDs
                    let yes_token_id = self.hash_to_id.get(&token_hash)
                        .or_else(|| self.hash_to_id.get(&complement_hash))
                        .cloned()
                        .unwrap_or_default();
                    let no_token_id = self.hash_to_id.get(&complement_hash)
                        .or_else(|| self.hash_to_id.get(&token_hash))
                        .cloned()
                        .unwrap_or_default();
                    
                    // Determine which is YES and which is NO
                    let (yes_hash, no_hash, yes_ask, yes_size, no_ask, no_size) = 
                        if ask_price <= comp_ask {
                            // This token is cheaper (likely YES)
                            (token_hash, complement_hash, ask_price, ask_size, comp_ask, comp_ask_size)
                        } else {
                            // Complement is cheaper (complement is YES)
                            (complement_hash, token_hash, comp_ask, comp_ask_size, ask_price, ask_size)
                        };
                    
                    // Get condition ID
                    let condition_id = self.id_to_condition.get(&yes_token_id)
                        .or_else(|| self.id_to_condition.get(&no_token_id))
                        .cloned()
                        .unwrap_or_default();
                    
                    // Send to background task
                    let task = BackgroundTask::EdgeDetected {
                        yes_token_hash: yes_hash,
                        no_token_hash: no_hash,
                        yes_token_id,
                        no_token_id,
                        condition_id,
                        combined_ask,
                        yes_ask_price: yes_ask,
                        yes_ask_size: yes_size,
                        no_ask_price: no_ask,
                        no_ask_size: no_size,
                        timestamp_nanos,
                    };
                    
                    if let Err(e) = task_tx.send(task) {
                        eprintln!("[HOT_PATH] Failed to send edge task: {:?}", e);
                    }
                }
            }
        }
        
        // Track latency
        let latency_ns = start.elapsed().as_nanos() as u64;
        self.latency_samples.push(latency_ns);
        
        // Emit stats periodically
        if self.last_stat_time.elapsed().as_secs() >= 1 {
            self.emit_latency_stats(task_tx);
            self.last_stat_time = Instant::now();
        }
    }

    /// Process trade event
    fn process_trade(&mut self, _token_hash: u64, _price: u64, _size: u64, _timestamp_nanos: u64) {
        // Trade events are informational - we primarily use book updates
        // Could be used for VWAP calculations or volume tracking
    }

    /// Process order matched event
    fn process_order_matched(
        &mut self,
        order_id: String,
        token_id: String,
        matched_size: u64,
        price: u64,
        task_tx: &Sender<BackgroundTask>,
    ) {
        let task = BackgroundTask::OrderMatched {
            order_id,
            token_id,
            matched_size,
            price,
        };
        
        if let Err(e) = task_tx.send(task) {
            eprintln!("[HOT_PATH] Failed to send order matched task: {:?}", e);
        }
    }

    /// Emit latency statistics
    fn emit_latency_stats(&mut self, task_tx: &Sender<BackgroundTask>) {
        if self.latency_samples.is_empty() {
            return;
        }
        
        // Calculate stats
        let mut sorted = self.latency_samples.clone();
        sorted.sort();
        
        let min_ns = sorted[0];
        let max_ns = sorted[sorted.len() - 1];
        let avg_ns = sorted.iter().sum::<u64>() / sorted.len() as u64;
        let p99_idx = (sorted.len() as f64 * 0.99) as usize;
        let p99_ns = sorted.get(p99_idx).copied().unwrap_or(max_ns);
        let sample_count = sorted.len() as u64;
        
        // Clear samples
        self.latency_samples.clear();
        
        // Send stats
        let task = BackgroundTask::LatencyStats {
            min_ns,
            max_ns,
            avg_ns,
            p99_ns,
            sample_count,
        };
        
        if let Err(e) = task_tx.send(task) {
            eprintln!("[HOT_PATH] Failed to send latency stats: {:?}", e);
        }
        
        // Print summary
        println!(
            "[HOT_PATH] 🔥 avg={:.2}µs min={:.2}µs max={:.2}µs p99={:.2}µs | {} msgs | {} edges",
            avg_ns as f64 / 1000.0,
            min_ns as f64 / 1000.0,
            max_ns as f64 / 1000.0,
            p99_ns as f64 / 1000.0,
            sample_count,
            self.edge_counter.load(Ordering::Relaxed),
        );
    }

    /// Run hot path with crossbeam receiver
    pub fn run(
        &mut self,
        ws_rx: Receiver<WsEvent>,
        task_tx: Sender<BackgroundTask>,
    ) {
        println!("[HOT_PATH] 🔥 Starting hot path processor");
        println!("[HOT_PATH] Token pairs: {}", self.token_pairs.len() / 2);
        println!("[HOT_PATH] Edge threshold: ${:.2}", self.edge_threshold as f64 / 1_000_000.0);
        
        loop {
            if self.killswitch.load(Ordering::Relaxed) {
                println!("[HOT_PATH] 🚨 Killswitch engaged - exiting");
                return;
            }
            
            // Receive from WebSocket engine
            match ws_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(event) => {
                    self.process_ws_event(event, &task_tx);
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    // Timeout - continue loop
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    println!("[HOT_PATH] ⚠️ Channel disconnected");
                    return;
                }
            }
        }
    }
}

/// Run hot path in a dedicated thread
pub fn spawn_hot_path(
    ws_rx: Receiver<WsEvent>,
    task_tx: Sender<BackgroundTask>,
    hash_to_id: HashMap<u64, String>,
    id_to_condition: HashMap<String, String>,
    token_pairs: HashMap<u64, u64>,
    killswitch: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut hot_path = HotPath::new(
            hash_to_id,
            id_to_condition,
            token_pairs,
            killswitch,
        );
        hot_path.run(ws_rx, task_tx);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_path_creation() {
        let mut hash_to_id = HashMap::new();
        hash_to_id.insert(12345, "token_yes".to_string());
        hash_to_id.insert(67890, "token_no".to_string());
        
        let mut id_to_condition = HashMap::new();
        id_to_condition.insert("token_yes".to_string(), "condition_1".to_string());
        id_to_condition.insert("token_no".to_string(), "condition_1".to_string());
        
        let mut token_pairs = HashMap::new();
        token_pairs.insert(12345, 67890);
        token_pairs.insert(67890, 12345);
        
        let killswitch = Arc::new(AtomicBool::new(false));
        
        let hot_path = HotPath::new(hash_to_id, id_to_condition, token_pairs, killswitch);
        assert_eq!(hot_path.edge_threshold, EDGE_THRESHOLD_U64);
    }
}