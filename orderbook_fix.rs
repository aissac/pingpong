//! OrderBook Tracker - FIXED: Replace instead of accumulate
//! 
//! CRITICAL FIXES:
//! 1. Replace depth, don't accumulate (fixes 739K depth bug)
//! 2. Validate tick size >= $0.01 (rejects $0.001 prices)
//! 3. Handle size=0 as deletion (removes stale quotes)

use dashmap::DashMap;
use std::sync::Arc;

/// Market price data with depth tracking
#[derive(Debug, Clone)]
pub struct MarketPrices {
    pub condition_id: String,
    pub yes_best_bid: Option<f64>,
    pub yes_best_ask: Option<f64>,
    pub no_best_bid: Option<f64>,
    pub no_best_ask: Option<f64>,
    pub yes_depth: f64,
    pub no_depth: f64,
    pub last_update: i64,
    pub queue_start_yes_depth: Option<f64>,
    pub queue_start_no_depth: Option<f64>,
}

impl MarketPrices {
    pub fn new(condition_id: String) -> Self {
        Self {
            condition_id,
            yes_best_bid: None,
            yes_best_ask: None,
            no_best_bid: None,
            no_best_ask: None,
            yes_depth: 0.0,
            no_depth: 0.0,
            last_update: 0,
            queue_start_yes_depth: None,
            queue_start_no_depth: None,
        }
    }

    pub fn combined_cost(&self) -> Option<f64> {
        match (self.yes_best_ask, self.no_best_ask) {
            (Some(yes), Some(no)) => Some(yes + no),
            _ => None,
        }
    }
    
    pub fn has_arbitrage(&self, target: f64) -> bool {
        self.combined_cost()
            .map(|c| c < target)
            .unwrap_or(false)
    }
}

/// Thread-safe orderbook tracker
pub struct OrderBookTracker {
    markets: Arc<DashMap<String, MarketPrices>>,
}

impl OrderBookTracker {
    pub fn new() -> Self {
        Self {
            markets: Arc::new(DashMap::new()),
        }
    }
    
    pub fn init_market(&self, condition_id: &str, yes_depth: f64, no_depth: f64) {
        let mut market = self.markets
            .entry(condition_id.to_string())
            .or_insert_with(|| MarketPrices::new(condition_id.to_string()));
        market.yes_depth = yes_depth;
        market.no_depth = no_depth;
    }
    
    /// Update prices for a market - CRITICAL FIX
    /// 
    /// RULES:
    /// 1. Replace depth, don't accumulate (fixes 739K depth bug)
    /// 2. Validate tick size >= $0.01 (rejects $0.001 prices)
    /// 3. Handle size=0 as deletion (removes stale quotes)
    pub fn update(&self, condition_id: &str, yes_ask: Option<f64>, no_ask: Option<f64>, yes_depth: f64, no_depth: f64) {
        // TICK SIZE VALIDATION: Reject prices below $0.01
        if let Some(yes) = yes_ask {
            if yes < 0.01 && yes > 0.0 {
                tracing::warn!("⛔ INVALID TICK: {} YES ask {:.6} < $0.01", &condition_id[..8], yes);
                return;
            }
        }
        if let Some(no) = no_ask {
            if no < 0.01 && no > 0.0 {
                tracing::warn!("⛔ INVALID TICK: {} NO ask {:.6} < $0.01", &condition_id[..8], no);
                return;
            }
        }
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        
        let mut market = self.markets
            .entry(condition_id.to_string())
            .or_insert_with(|| MarketPrices::new(condition_id.to_string()));
        
        // Update prices if provided
        if yes_ask.is_some() {
            market.yes_best_ask = yes_ask;
        }
        if no_ask.is_some() {
            market.no_best_ask = no_ask;
        }
        
        // CRITICAL FIX: REPLACE depth, don't accumulate
        // This fixes the 739,000 share bug
        if yes_depth > 0.0 {
            market.yes_depth = yes_depth;  // REPLACE, not +=
        } else if yes_ask.is_some() {
            // size=0 means quote was cancelled - clear depth
            market.yes_depth = 0.0;
        }
        
        if no_depth > 0.0 {
            market.no_depth = no_depth;  // REPLACE, not +=
        } else if no_ask.is_some() {
            // size=0 means quote was cancelled - clear depth
            market.no_depth = 0.0;
        }
        
        market.last_update = timestamp;
        
        // Log realistic depths (should be < 1000 shares for Up/Down markets)
        tracing::debug!(
            "📈 TRACKER UPDATE: {} | YES={:.2} NO={:.2}",
            &condition_id[..8.min(condition_id.len())],
            market.yes_depth, market.no_depth
        );
        
        // Warn if depth is unrealistic (> 10000 shares = > $10,000)
        if market.yes_depth > 10000.0 || market.no_depth > 10000.0 {
            tracing::warn!(
                "⚠️ UNREALISTIC DEPTH: {} | YES={:.0} NO={:.0} (> $10K)",
                &condition_id[..8.min(condition_id.len())],
                market.yes_depth, market.no_depth
            );
        }
    }

    pub fn get(&self, condition_id: &str) -> Option<MarketPrices> {
        self.markets.get(condition_id).map(|r| r.clone())
    }

    pub fn mark_queue_start(&self, condition_id: &str) {
        let mut market = self.markets
            .entry(condition_id.to_string())
            .or_insert_with(|| MarketPrices::new(condition_id.to_string()));
        market.queue_start_yes_depth = Some(market.yes_depth);
        market.queue_start_no_depth = Some(market.no_depth);
    }

    pub fn check_ghost_liquidity(&self, condition_id: &str) -> bool {
        self.markets
            .get(condition_id)
            .map(|r| r.liquidity_vanished_during_queue())
            .unwrap_or(false)
    }

    pub fn clear_queue_markers(&self, condition_id: &str) {
        if let Some(mut market) = self.markets.get_mut(condition_id) {
            market.queue_start_yes_depth = None;
            market.queue_start_no_depth = None;
        }
    }

    pub fn get_all_markets(&self) -> Vec<MarketPrices> {
        self.markets.iter().map(|r| r.clone()).collect()
    }

    pub fn market_count(&self) -> usize {
        self.markets.len()
    }
    
    /// Get best asks for a market (for adverse selection tracking)
    pub fn get_best_asks(&self, condition_id: &str) -> (f64, f64) {
        match self.markets.get(condition_id) {
            Some(market) => (market.yes_best_ask.unwrap_or(0.0), market.no_best_ask.unwrap_or(0.0)),
            None => (0.0, 0.0)
        }
    }
    
    /// Check if liquidity vanished during queue, accounting for own fills
    pub fn check_ghost_liquidity_with_fill(&self, condition_id: &str, filled_size: f64) -> bool {
        self.markets
            .get(condition_id)
            .map(|r| r.liquidity_vanished_during_queue_with_fill(filled_size))
            .unwrap_or(false)
    }
}

impl MarketPrices {
    /// Check if liquidity vanished during queue, accounting for own fills
    pub fn liquidity_vanished_during_queue_with_fill(&self, filled_size: f64) -> bool {
        match (self.queue_start_yes_depth, self.queue_start_no_depth) {
            (Some(start_yes), Some(start_no)) => {
                let yes_ghost_volume = start_yes - self.yes_depth - filled_size;
                let no_ghost_volume = start_no - self.no_depth - filled_size;
                
                let yes_ghost = yes_ghost_volume > start_yes * 0.5;
                let no_ghost = no_ghost_volume > start_no * 0.5;
                let ghost_detected = yes_ghost || no_ghost;
                
                if ghost_detected {
                    tracing::warn!(
                        "👻 GHOST DETECTED: {} | YES ghost: {:.2} | NO ghost: {:.2} | filled: {:.0}",
                        &self.condition_id[..8.min(self.condition_id.len())],
                        yes_ghost_volume, no_ghost_volume, filled_size
                    );
                }
                
                ghost_detected
            }
            _ => false
        }
    }
    
    /// Legacy method for backward compatibility
    pub fn liquidity_vanished_during_queue(&self) -> bool {
        match (self.queue_start_yes_depth, self.queue_start_no_depth) {
            (Some(start_yes), Some(start_no)) => {
                let yes_vanished = self.yes_depth < start_yes * 0.5;
                let no_vanished = self.no_depth < start_no * 0.5;
                yes_vanished || no_vanished
            }
            _ => false
        }
    }
    
    /// Record depth at start of 500ms queue
    pub fn mark_queue_start(&mut self) {
        self.queue_start_yes_depth = Some(self.yes_depth);
        self.queue_start_no_depth = Some(self.no_depth);
    }
    
    /// Clear queue markers after execution
    pub fn clear_queue_markers(&mut self) {
        self.queue_start_yes_depth = None;
        self.queue_start_no_depth = None;
    }
}

impl Default for OrderBookTracker {
    fn default() -> Self {
        Self::new()
    }
}