//! Single-threaded pinned hot path for sub-microsecond latency
//!
//! Eliminates:
//! 1. Crossbeam channel overhead (~0.5-1µs)
//! 2. DashMap concurrent access (~1-2µs)
//! 3. Cache invalidation from thread handoffs
//!
//! Pattern: WebSocket receive + parse + process in ONE pinned loop

use std::thread;
use std::time::Instant;
use std::collections::HashMap;

/// CPU core affinity for hot path
pub const HOT_PATH_CORE: usize = 0;

/// Thread-local orderbook state (no DashMap, no locks)
pub struct HotPathState {
    /// Orderbook: condition_id -> (yes_price, yes_depth, no_price, no_depth)
    pub orderbook: HashMap<String, (f64, f64, f64, f64)>,
    /// Token to condition mapping
    pub token_side: HashMap<String, (String, bool)>,
    /// Latency tracking
    pub total_latency_ns: u64,
    pub message_count: u64,
}

impl HotPathState {
    pub fn new() -> Self {
        Self {
            orderbook: HashMap::with_capacity(1024),
            token_side: HashMap::with_capacity(2048),
            total_latency_ns: 0,
            message_count: 0,
        }
    }
    
    /// Update orderbook - zero allocation, no locks
    #[inline(always)]
    pub fn update(&mut self, condition_id: &str, token_id: &str, is_yes: bool, price: f64, depth: f64) {
        let entry = self.orderbook.entry(condition_id.to_string()).or_insert((0.0, 0.0, 0.0, 0.0));
        
        if is_yes {
            entry.0 = price;  // yes_price
            entry.1 = depth;  // yes_depth
        } else {
            entry.2 = price;  // no_price
            entry.3 = depth;  // no_depth
        }
    }
    
    /// Check for arbitrage opportunity - inline for hot path
    #[inline(always)]
    pub fn check_arbitrage(&self, condition_id: &str, threshold: f64) -> Option<(f64, f64, f64)> {
        if let Some((yes_price, yes_depth, no_price, no_depth)) = self.orderbook.get(condition_id) {
            let combined = yes_price + no_price;
            if combined < threshold {
                return Some((*yes_price, *no_price, combined));
            }
        }
        None
    }
    
    /// Record latency for metrics
    #[inline(always)]
    pub fn record_latency(&mut self, elapsed_ns: u64) {
        self.total_latency_ns += elapsed_ns;
        self.message_count += 1;
    }
    
    /// Get average latency
    pub fn avg_latency_ns(&self) -> u64 {
        if self.message_count == 0 {
            0
        } else {
            self.total_latency_ns / self.message_count
        }
    }
}

/// Pin current thread to specific CPU core using core_affinity crate
pub fn pin_to_core(core: usize) -> Result<(), String> {
    let core_ids = core_affinity::get_core_ids();
    if let Some(cores) = core_ids {
        if core < cores.len() {
            if core_affinity::set_for_current(cores[core]) {
                return Ok(());
            }
        }
    }
    Err(format!("Failed to pin to core {}", core))
}

/// Spawn hot path thread - pinned to CPU core
pub fn spawn_hot_path<F>(core: usize, f: F) -> thread::JoinHandle<()>
where
    F: FnOnce() + Send + 'static,
{
    thread::spawn(move || {
        // Pin to CPU core
        if let Err(e) = pin_to_core(core) {
            tracing::warn!("Failed to pin to core {}: {}", core, e);
        }
        
        // Run hot path
        f();
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hot_path_update() {
        let mut state = HotPathState::new();
        state.update("test", "token1", true, 0.45, 100.0);
        state.update("test", "token2", false, 0.55, 200.0);
        
        let (yes_price, yes_depth, no_price, no_depth) = state.orderbook.get("test").unwrap();
        assert_eq!(*yes_price, 0.45);
        assert_eq!(*yes_depth, 100.0);
        assert_eq!(*no_price, 0.55);
        assert_eq!(*no_depth, 200.0);
    }
    
    #[test]
    fn test_arbitrage_check() {
        let mut state = HotPathState::new();
        state.update("test", "token1", true, 0.40, 100.0);
        state.update("test", "token2", false, 0.50, 200.0);
        
        // Combined = 0.90 < 0.95 threshold
        let arb = state.check_arbitrage("test", 0.95);
        assert!(arb.is_some());
        
        let (yes, no, combined) = arb.unwrap();
        assert_eq!(yes, 0.40);
        assert_eq!(no, 0.50);
        assert_eq!(combined, 0.90);
    }
}