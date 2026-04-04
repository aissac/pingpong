//! Market State - Zero-Allocation Price Tracking
//!
//! Groups YES/NO prices under condition_id using BTreeMap
//! Uses fixed-point integers (micro-USDC) for branchless arithmetic
//!
//! CRITICAL: This module is SYNCHRONOUS - no async/await
//! All updates come from hot_path via crossbeam channels

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

/// Fixed-point price (micro-USDC: $0.49 = 490,000)
pub type MicroUsdc = u64;

/// Market prices grouped by condition_id
/// 
/// Stores YES and NO prices together for edge detection
#[derive(Debug, Clone)]
pub struct MarketPrices {
    /// YES token ID
    pub yes_token_id: String,
    /// NO token ID
    pub no_token_id: String,
    /// YES best ask (for edge calculation)
    pub yes_ask: Option<MicroUsdc>,
    /// NO best ask (for edge calculation)
    pub no_ask: Option<MicroUsdc>,
    /// YES best bid
    pub yes_bid: Option<MicroUsdc>,
    /// NO best bid
    pub no_bid: Option<MicroUsdc>,
    /// Timestamp of last update (nanos)
    pub last_update_ns: u64,
}

impl MarketPrices {
    pub fn new(yes_token_id: String, no_token_id: String) -> Self {
        Self {
            yes_token_id,
            no_token_id,
            yes_ask: None,
            no_ask: None,
            yes_bid: None,
            no_bid: None,
            last_update_ns: 0,
        }
    }
    
    /// Calculate combined ask (YES_ask + NO_ask)
    /// Returns None if either price is missing
    #[inline]
    pub fn combined_ask(&self) -> Option<MicroUsdc> {
        match (self.yes_ask, self.no_ask) {
            (Some(yes), Some(no)) => Some(yes.saturating_add(no)),
            _ => None,
        }
    }
    
    /// Check for arbitrage opportunity
    /// Returns true if combined_ask < threshold
    /// Standard threshold: 950,000 (0.95)
    #[inline]
    pub fn is_edge(&self, threshold: MicroUsdc) -> Option<bool> {
        self.combined_ask().map(|combined| combined < threshold)
    }
    
    /// Reset all prices (called on reconnection)
    #[inline]
    pub fn reset(&mut self) {
        self.yes_ask = None;
        self.no_ask = None;
        self.yes_bid = None;
        self.no_bid = None;
        self.last_update_ns = 0;
    }
}

/// Synchronous market state manager for hot path
/// 
/// NO ASYNC - this is CPU-bound decision logic
/// All updates are synchronous via &mut references
pub struct MarketState {
    /// condition_id -> MarketPrices
    markets: BTreeMap<String, MarketPrices>,
    /// token_id -> condition_id mapping
    token_to_condition: BTreeMap<String, String>,
    /// Edge detection threshold (micro-USDC)
    /// Standard: 950,000 = $0.95 for dump-and-hedge arbitrage
    edge_threshold: MicroUsdc,
}

impl MarketState {
    pub fn new(edge_threshold: MicroUsdc) -> Self {
        Self {
            markets: BTreeMap::new(),
            token_to_condition: BTreeMap::new(),
            edge_threshold,
        }
    }
    
    /// Register a new market with YES/NO token pair
    pub fn register_market(
        &mut self,
        condition_id: String,
        yes_token_id: String,
        no_token_id: String,
    ) {
        self.markets.insert(
            condition_id.clone(),
            MarketPrices::new(yes_token_id.clone(), no_token_id.clone()),
        );
        
        self.token_to_condition.insert(yes_token_id, condition_id.clone());
        self.token_to_condition.insert(no_token_id, condition_id);
    }
    
    /// Update price for a token (SYNCHRONOUS)
    /// Returns (condition_id, combined_ask) if market has both YES and NO prices
    #[inline]
    pub fn update_price(
        &mut self,
        token_id: &str,
        bid: MicroUsdc,
        ask: MicroUsdc,
        timestamp_ns: u64,
    ) -> Option<(String, MicroUsdc)> {
        let condition_id = self.token_to_condition.get(token_id)?;
        let market = self.markets.get_mut(condition_id)?;
        
        // Determine if this is YES or NO token
        let is_yes = token_id == market.yes_token_id;
        
        if is_yes {
            market.yes_bid = Some(bid);
            market.yes_ask = Some(ask);
        } else {
            market.no_bid = Some(bid);
            market.no_ask = Some(ask);
        }
        
        market.last_update_ns = timestamp_ns;
        
        // Return condition_id and combined_ask if we have both prices
        market.combined_ask().map(|combined| {
            (condition_id.clone(), combined)
        })
    }
    
    /// Clear all prices (call on WebSocket disconnect)
    pub fn clear_all(&mut self) {
        for market in self.markets.values_mut() {
            market.reset();
        }
    }
    
    /// Get market state
    pub fn get_market(&self, condition_id: &str) -> Option<&MarketPrices> {
        self.markets.get(condition_id)
    }
    
    /// Find all arbitrage opportunities
    /// Returns Vec<(condition_id, yes_ask, no_ask, combined_ask)>
    pub fn find_edges(&self) -> Vec<(String, MicroUsdc, MicroUsdc, MicroUsdc)> {
        self.markets
            .iter()
            .filter_map(|(condition_id, market)| {
                let yes_ask = market.yes_ask?;
                let no_ask = market.no_ask?;
                let combined = yes_ask.saturating_add(no_ask);
                
                if combined < self.edge_threshold {
                    Some((condition_id.clone(), yes_ask, no_ask, combined))
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Check if a specific market is an edge
    #[inline]
    pub fn is_edge(&self, condition_id: &str) -> Option<bool> {
        self.markets.get(condition_id)?.is_edge(self.edge_threshold)
    }
    
    /// Get edge threshold
    pub fn threshold(&self) -> MicroUsdc {
        self.edge_threshold
    }
}

/// Thread-safe wrapper using Arc<Atomic> for shared counters
pub struct MarketStats {
    /// Total updates processed
    pub updates: AtomicU64,
    /// Edges detected
    pub edges: AtomicU64,
    /// Last edge timestamp
    pub last_edge_ns: AtomicU64,
}

impl MarketStats {
    pub fn new() -> Self {
        Self {
            updates: AtomicU64::new(0),
            edges: AtomicU64::new(0),
            last_edge_ns: AtomicU64::new(0),
        }
    }
    
    pub fn increment_updates(&self) {
        self.updates.fetch_add(1, AtomicOrdering::Relaxed);
    }
    
    pub fn increment_edges(&self, timestamp_ns: u64) {
        self.edges.fetch_add(1, AtomicOrdering::Relaxed);
        self.last_edge_ns.store(timestamp_ns, AtomicOrdering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_prices() {
        let mut market = MarketPrices::new(
            "yes_token".to_string(),
            "no_token".to_string(),
        );
        
        // Initially no prices
        assert!(market.combined_ask().is_none());
        assert!(market.is_edge(950_000).is_none());
        
        // Set YES price
        market.yes_ask = Some(480_000);
        assert!(market.combined_ask().is_none()); // Still missing NO
        
        // Set NO price
        market.no_ask = Some(460_000);
        assert_eq!(market.combined_ask(), Some(940_000));
        
        // Check edge detection with threshold 0.95
        assert_eq!(market.is_edge(950_000), Some(true));  // 940k < 950k
        assert_eq!(market.is_edge(930_000), Some(false)); // 940k > 930k
        
        // Reset
        market.reset();
        assert!(market.yes_ask.is_none());
        assert!(market.no_ask.is_none());
    }

    #[test]
    fn test_market_state() {
        let mut state = MarketState::new(950_000);
        
        state.register_market(
            "condition_1".to_string(),
            "yes_token".to_string(),
            "no_token".to_string(),
        );
        
        // Update YES price - should return None (missing NO)
        let result = state.update_price("yes_token", 470_000, 490_000, 1000);
        assert!(result.is_none());
        
        // Update NO price - should return combined
        let result = state.update_price("no_token", 450_000, 470_000, 1001);
        assert!(result.is_some());
        let (cond_id, combined) = result.unwrap();
        assert_eq!(cond_id, "condition_1");
        assert_eq!(combined, 960_000); // 490k + 470k
        
        // Find edges (should find none because 960k > 950k threshold)
        let edges = state.find_edges();
        assert!(edges.is_empty());
        
        // Update with better prices
        state.update_price("yes_token", 470_000, 480_000, 1002);
        state.update_price("no_token", 450_000, 460_000, 1003);
        
        let edges = state.find_edges();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].3, 940_000); // combined = 480k + 460k
    }

    #[test]
    fn test_clear_on_reconnect() {
        let mut state = MarketState::new(950_000);
        
        state.register_market("c1".to_string(), "yes".to_string(), "no".to_string());
        state.update_price("yes", 470_000, 490_000, 1000);
        state.update_price("no", 450_000, 470_000, 1001);
        
        // Verify we have prices
        assert!(state.get_market("c1").unwrap().yes_ask.is_some());
        
        // Clear on reconnect
        state.clear_all();
        
        // Verify prices are gone
        assert!(state.get_market("c1").unwrap().yes_ask.is_none());
        assert!(state.get_market("c1").unwrap().no_ask.is_none());
    }
}