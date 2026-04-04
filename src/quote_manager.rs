//! Quote Manager - Skew-Aware Quote Generation for Market Maker
//!
//! Implements:
//! - Expensive-side skew (buy high, sell low on directional bias)
//! - Quote generation based on inventory position
//! - Spread management
//! - Position-based pricing

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::inventory_tracker::{InventoryTracker, Side};

/// Quote configuration
#[derive(Debug, Clone)]
pub struct QuoteConfig {
    /// Base spread in basis points (e.g., 100 = 1%)
    pub base_spread_bps: u64,
    /// Maximum additional spread from skew
    pub max_skew_spread_bps: u64,
    /// Inventory skew factor (how much position affects price)
    pub skew_factor: f64,
    /// Minimum price (don't quote below this)
    pub min_price: f64,
    /// Maximum price (don't quote above this)
    pub max_price: f64,
    /// Post-only (maker) - always true for rebates
    pub post_only: bool,
}

impl Default for QuoteConfig {
    fn default() -> Self {
        Self {
            base_spread_bps: 100,           // 1% base spread
            max_skew_spread_bps: 200,       // Max 2% additional from skew
            skew_factor: 0.5,               // Moderate skew impact
            min_price: 0.01,                // Don't quote below $0.01
            max_price: 0.99,                // Don't quote above $0.99
            post_only: true,                // Always post-only for maker rebate
        }
    }
}

/// Generated quote for a token
#[derive(Debug, Clone)]
pub struct Quote {
    pub token_id: String,
    pub condition_id: String,
    pub side: QuoteSide,
    pub price: u64,        // Micro-USDC
    pub size: u64,         // Micro-shares
    pub spread_bps: u64,
    pub skew_adjustment: f64,
    pub post_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteSide {
    Buy,  // We buy YES/NO
    Sell, // We sell YES/NO
}

/// Market state from orderbook
#[derive(Debug, Clone)]
pub struct MarketState {
    pub yes_best_bid: u64,
    pub yes_best_ask: u64,
    pub yes_bid_size: u64,
    pub yes_ask_size: u64,
    pub no_best_bid: u64,
    pub no_best_ask: u64,
    pub no_bid_size: u64,
    pub no_ask_size: u64,
    pub timestamp_nanos: u64,
}

impl Default for MarketState {
    fn default() -> Self {
        Self {
            yes_best_bid: 0,
            yes_best_ask: 0,
            yes_bid_size: 0,
            yes_ask_size: 0,
            no_best_bid: 0,
            no_best_ask: 0,
            no_bid_size: 0,
            no_ask_size: 0,
            timestamp_nanos: 0,
        }
    }
}

/// Quote Manager
/// 
/// Generates skew-aware quotes for market making
pub struct QuoteManager {
    config: QuoteConfig,
    inventory: Arc<InventoryTracker>,
    /// Cached market states
    market_states: Arc<RwLock<HashMap<String, MarketState>>>,
    /// Maximum inventory we want to hold (micro-USDC)
    max_inventory: u64,
}

impl QuoteManager {
    /// Create a new quote manager
    pub fn new(config: QuoteConfig, inventory: Arc<InventoryTracker>, max_inventory: u64) -> Self {
        Self {
            config,
            inventory,
            market_states: Arc::new(RwLock::new(HashMap::new())),
            max_inventory,
        }
    }

    /// Update market state from WebSocket
    pub async fn update_market_state(&self, condition_id: &str, state: MarketState) {
        let mut states = self.market_states.write().await;
        states.insert(condition_id.to_string(), state);
    }

    /// Generate quotes for a market (YES and NO)
    /// 
    /// Returns (yes_quote, no_quote) based on:
    /// - Current market prices
    /// - Inventory position
    /// - Skew adjustment
    pub async fn generate_quotes(&self, condition_id: &str, yes_token_id: &str, no_token_id: &str) -> Option<(Quote, Quote)> {
        // Get current market state
        let states = self.market_states.read().await;
        let market = states.get(condition_id)?;
        
        // Get current inventory skew
        let skew = self.inventory.get_skew(condition_id).await;
        
        // Calculate spread based on skew
        let (yes_spread, no_spread) = self.calculate_skewed_spreads(skew);
        
        // Calculate prices with skew
        let (yes_price, yes_side) = self.calculate_quote_price(
            market.yes_best_bid,
            market.yes_best_ask,
            skew,
            true, // YES token
        ).await;
        
        let (no_price, no_side) = self.calculate_quote_price(
            market.no_best_bid,
            market.no_best_ask,
            -skew, // NO skew is inverse of YES
            false, // NO token
        ).await;
        
        // Calculate sizes based on available liquidity and position limits
        let yes_size = self.calculate_quote_size(yes_token_id, market.yes_ask_size).await;
        let no_size = self.calculate_quote_size(no_token_id, market.no_ask_size).await;
        
        drop(states);
        
        let yes_quote = Quote {
            token_id: yes_token_id.to_string(),
            condition_id: condition_id.to_string(),
            side: yes_side,
            price: yes_price,
            size: yes_size,
            spread_bps: yes_spread,
            skew_adjustment: skew,
            post_only: self.config.post_only,
        };
        
        let no_quote = Quote {
            token_id: no_token_id.to_string(),
            condition_id: condition_id.to_string(),
            side: no_side,
            price: no_price,
            size: no_size,
            spread_bps: no_spread,
            skew_adjustment: -skew,
            post_only: self.config.post_only,
        };
        
        Some((yes_quote, no_quote))
    }

    /// Calculate spreads adjusted for skew
    /// 
    /// When we're long YES (positive skew):
    /// - Quote YES higher (wider spread on sell side)
    /// - Quote NO lower (wider spread on buy side)
    fn calculate_skewed_spreads(&self, skew: f64) -> (u64, u64) {
        let base = self.config.base_spread_bps;
        let max_extra = self.config.max_skew_spread_bps;
        
        // Adjust spread based on skew
        // Positive skew = long YES = want to sell YES higher, buy NO lower
        let skew_adj = (skew.abs() * max_extra as f64) as u64;
        
        if skew > 0.0 {
            // Long YES: quote YES wider (want to sell), NO narrower (want to buy)
            (base + skew_adj, base)
        } else {
            // Long NO: quote YES narrower (want to buy), NO wider (want to sell)
            (base, base + skew_adj)
        }
    }

    /// Calculate quote price based on market and skew
    async fn calculate_quote_price(&self, best_bid: u64, best_ask: u64, skew: f64, is_yes: bool) -> (u64, QuoteSide) {
        let spread_bps = self.config.base_spread_bps;
        
        // Mid price
        let mid = if best_bid > 0 && best_ask > 0 {
            (best_bid + best_ask) / 2
        } else if best_ask > 0 {
            best_ask
        } else if best_bid > 0 {
            best_bid
        } else {
            return (0, QuoteSide::Buy); // No market
        };
        
        // Apply skew to price
        // Positive skew = we're long, want to sell higher
        // Negative skew = we're short, want to buy lower
        let skew_price_adj = skew * self.config.skew_factor * mid as f64;
        
        // Base spread adjustment
        let spread_adj = (spread_bps as f64 / 10000.0) * mid as f64;
        
        // Determine which side to quote
        // We want to be a maker on the expensive side
        let (price, side) = if skew > 0.0 {
            // We're long - want to sell (quote ask side)
            let ask_price = (mid as f64 + spread_adj / 2.0 + skew_price_adj) as u64;
            let ask_price = ask_price.max(self.config.min_price as u64 * 1_000_000);
            let ask_price = ask_price.min(self.config.max_price as u64 * 1_000_000);
            (ask_price, QuoteSide::Sell)
        } else {
            // We're short or neutral - want to buy (quote bid side)
            let bid_price = (mid as f64 - spread_adj / 2.0 + skew_price_adj) as u64;
            let bid_price = bid_price.max(self.config.min_price as u64 * 1_000_000);
            let bid_price = bid_price.min(self.config.max_price as u64 * 1_000_000);
            (bid_price, QuoteSide::Buy)
        };
        
        (price, side)
    }

    /// Calculate appropriate quote size
    async fn calculate_quote_size(&self, token_id: &str, available_size: u64) -> u64 {
        // Check position limits
        let position = self.inventory.get_position(token_id).await;
        let current_size = position.map(|p| p.net_size.abs() as u64).unwrap_or(0);
        
        // Don't exceed max inventory
        let remaining_capacity = self.max_inventory.saturating_sub(current_size);
        
        // Quote size is min of:
        // - Available size in market
        // - Remaining capacity
        // - A reasonable portion of capacity (10%)
        let max_quote_size = remaining_capacity / 10;
        
        available_size.min(remaining_capacity).min(max_quote_size.max(100_000)) // At least 0.1 USDC
    }

    /// Check if a combined price opportunity is worth taking
    /// 
    /// Returns true if YES_ask + NO_ask < threshold
    pub async fn should_take_arbitrage(&self, condition_id: &str, yes_ask: u64, no_ask: u64, threshold: u64) -> bool {
        let combined = yes_ask.saturating_add(no_ask);
        
        if combined >= threshold {
            return false;
        }
        
        // Check inventory limits
        let yes_within_limits = self.inventory.is_within_limits(&format!("{}_yes", condition_id)).await;
        let no_within_limits = self.inventory.is_within_limits(&format!("{}_no", condition_id)).await;
        
        yes_within_limits && no_within_limits
    }

    /// Get current market state
    pub async fn get_market_state(&self, condition_id: &str) -> Option<MarketState> {
        let states = self.market_states.read().await;
        states.get(condition_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quote_generation() {
        let inventory = Arc::new(InventoryTracker::new(1_000_000));
        let manager = QuoteManager::new(QuoteConfig::default(), inventory, 1_000_000);
        
        // Set up market state
        let state = MarketState {
            yes_best_bid: 480_000,  // $0.48
            yes_best_ask: 520_000,  // $0.52
            yes_bid_size: 1_000_000,
            yes_ask_size: 1_000_000,
            no_best_bid: 480_000,
            no_best_ask: 520_000,
            no_bid_size: 1_000_000,
            no_ask_size: 1_000_000,
            timestamp_nanos: 0,
        };
        
        manager.update_market_state("test_cond", state).await;
        
        // Generate quotes
        let quotes = manager.generate_quotes("test_cond", "yes_token", "no_token").await;
        assert!(quotes.is_some());
    }

    #[tokio::test]
    async fn test_skewed_spreads() {
        let inventory = Arc::new(InventoryTracker::new(100_000));
        let config = QuoteConfig {
            base_spread_bps: 100,
            max_skew_spread_bps: 200,
            ..Default::default()
        };
        let manager = QuoteManager::new(config, inventory, 100_000);
        
        // Positive skew = long YES
        let (yes_spread, no_spread) = manager.calculate_skewed_spreads(0.5);
        assert!(yes_spread > no_spread); // YES spread wider (want to sell)
        
        // Negative skew = long NO
        let (yes_spread, no_spread) = manager.calculate_skewed_spreads(-0.5);
        assert!(no_spread > yes_spread); // NO spread wider (want to sell)
    }

    #[tokio::test]
    async fn test_arbitrage_check() {
        let inventory = Arc::new(InventoryTracker::new(1_000_000));
        let manager = QuoteManager::new(QuoteConfig::default(), inventory, 1_000_000);
        
        // Combined < threshold
        let should_take = manager.should_take_arbitrage("cond", 480_000, 460_000, 950_000).await;
        assert!(should_take); // 480k + 460k = 940k < 950k
        
        // Combined > threshold
        let should_take = manager.should_take_arbitrage("cond", 480_000, 480_000, 950_000).await;
        assert!(!should_take); // 480k + 480k = 960k > 950k
    }
}