//! Inventory Tracker - Position Management for Market Maker
//!
//! Implements:
//! - Real-time inventory tracking per token
//! - Position limits and skew awareness
//! - PnL tracking
//! - Ghost detection (stale positions)

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Token position
#[derive(Debug, Clone, Copy)]
pub struct Position {
    /// Net position (positive = long, negative = short)
    pub net_size: i64,
    /// Average entry price (micro-USDC)
    pub avg_entry_price: u64,
    /// Realized PnL (micro-USDC)
    pub realized_pnl: i64,
    /// Unrealized PnL (micro-USDC)
    pub unrealized_pnl: i64,
    /// Last update timestamp
    pub last_update: DateTime<Utc>,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            net_size: 0,
            avg_entry_price: 0,
            realized_pnl: 0,
            unrealized_pnl: 0,
            last_update: Utc::now(),
        }
    }
}

/// Market with YES/NO positions
#[derive(Debug, Clone)]
pub struct MarketPosition {
    pub condition_id: String,
    pub yes_position: Position,
    pub no_position: Position,
    /// Combined value (YES + NO should equal ~$1 per share)
    pub combined_value: u64,
}

/// Inventory Tracker
/// 
/// Thread-safe position tracking for all markets
pub struct InventoryTracker {
    /// Positions by token ID
    positions: Arc<RwLock<HashMap<String, Position>>>,
    /// Market positions by condition ID
    markets: Arc<RwLock<HashMap<String, MarketPosition>>>,
    /// Total USDC balance
    usdc_balance: Arc<RwLock<u64>>,
    /// Maximum position size per token (micro-USDC)
    max_position: u64,
    /// Ghost detection threshold (seconds)
    ghost_threshold_secs: u64,
}

impl InventoryTracker {
    /// Create a new inventory tracker
    pub fn new(max_position: u64) -> Self {
        Self {
            positions: Arc::new(RwLock::new(HashMap::new())),
            markets: Arc::new(RwLock::new(HashMap::new())),
            usdc_balance: Arc::new(RwLock::new(0)),
            max_position,
            ghost_threshold_secs: 300, // 5 minutes
        }
    }

    /// Create with custom ghost threshold
    pub fn with_ghost_threshold(max_position: u64, ghost_threshold_secs: u64) -> Self {
        Self {
            positions: Arc::new(RwLock::new(HashMap::new())),
            markets: Arc::new(RwLock::new(HashMap::new())),
            usdc_balance: Arc::new(RwLock::new(0)),
            max_position,
            ghost_threshold_secs,
        }
    }

    /// Record a fill (trade execution)
    pub async fn record_fill(
        &self,
        token_id: &str,
        condition_id: &str,
        side: Side,
        size: u64,
        price: u64, // micro-USDC
    ) -> Result<(), String> {
        let mut positions = self.positions.write().await;
        let mut markets = self.markets.write().await;

        // Update token position
        let position = positions.entry(token_id.to_string()).or_default();
        
        match side {
            Side::Buy => {
                // Buying increases position
                let new_size = position.net_size + size as i64;
                // Update average entry price
                if new_size > 0 && position.net_size >= 0 {
                    // Adding to long position
                    let total_cost = position.avg_entry_price * position.net_size as u64 + price * size;
                    position.avg_entry_price = total_cost / new_size as u64;
                }
                position.net_size = new_size;
            }
            Side::Sell => {
                // Selling decreases position
                let new_size = position.net_size - size as i64;
                if position.net_size > 0 && new_size < position.net_size {
                    // Closing long - realize PnL
                    let closed_size = (position.net_size - new_size) as u64;
                    let pnl = (closed_size as i64) * (price as i64 - position.avg_entry_price as i64);
                    position.realized_pnl += pnl;
                }
                position.net_size = new_size;
            }
        }
        
        position.last_update = Utc::now();

        // Update market position
        let market = markets.entry(condition_id.to_string()).or_insert(MarketPosition {
            condition_id: condition_id.to_string(),
            yes_position: Position::default(),
            no_position: Position::default(),
            combined_value: 0,
        });

        // Determine if this is YES or NO token (caller should know)
        // For now, we'll track by token_id
        if token_id == condition_id {
            // This is the YES token
            market.yes_position = position.clone();
        } else {
            // Assume this is the NO token
            market.no_position = position.clone();
        }

        // Update combined value
        market.combined_value = (market.yes_position.net_size.abs() + market.no_position.net_size.abs()) as u64 / 2;

        Ok(())
    }

    /// Get position for a token
    pub async fn get_position(&self, token_id: &str) -> Option<Position> {
        let positions = self.positions.read().await;
        positions.get(token_id).copied()
    }

    /// Get market position
    pub async fn get_market_position(&self, condition_id: &str) -> Option<MarketPosition> {
        let markets = self.markets.read().await;
        markets.get(condition_id).cloned()
    }

    /// Check if position is within limits
    pub async fn is_within_limits(&self, token_id: &str) -> bool {
        let positions = self.positions.read().await;
        if let Some(position) = positions.get(token_id) {
            position.net_size.abs() as u64 <= self.max_position
        } else {
            true // No position = within limits
        }
    }

    /// Get current skew for a market (positive = long YES, negative = long NO)
    /// 
    /// Returns skew as percentage of max position:
    /// - 1.0 = max long YES
    /// - -1.0 = max long NO
    /// - 0.0 = balanced
    pub async fn get_skew(&self, condition_id: &str) -> f64 {
        let markets = self.markets.read().await;
        if let Some(market) = markets.get(condition_id) {
            let yes_size = market.yes_position.net_size;
            let no_size = market.no_position.net_size;
            
            // Skew = (YES - NO) / max_position
            // Positive = we own more YES than NO
            // Negative = we own more NO than YES
            let skew = (yes_size - no_size) as f64 / self.max_position as f64;
            skew.clamp(-1.0, 1.0)
        } else {
            0.0
        }
    }

    /// Detect ghost positions (stale data)
    pub async fn detect_ghosts(&self) -> Vec<String> {
        let positions = self.positions.read().await;
        let now = Utc::now();
        
        positions
            .iter()
            .filter(|(_, pos)| {
                let age = (now - pos.last_update).num_seconds();
                age > self.ghost_threshold_secs as i64 && pos.net_size != 0
            })
            .map(|(token_id, _)| token_id.clone())
            .collect()
    }

    /// Update USDC balance
    pub async fn set_usdc_balance(&self, balance: u64) {
        *self.usdc_balance.write().await = balance;
    }

    /// Get USDC balance
    pub async fn get_usdc_balance(&self) -> u64 {
        *self.usdc_balance.read().await
    }

    /// Calculate unrealized PnL for a position at current market price
    pub async fn calculate_unrealized_pnl(&self, token_id: &str, current_price: u64) -> i64 {
        let positions = self.positions.read().await;
        if let Some(position) = positions.get(token_id) {
            position.net_size as i64 * (current_price as i64 - position.avg_entry_price as i64)
        } else {
            0
        }
    }

    /// Get total inventory summary
    pub async fn get_summary(&self) -> InventorySummary {
        let positions = self.positions.read().await;
        let usdc_balance = self.usdc_balance.read().await;
        
        let mut total_long = 0i64;
        let mut total_short = 0i64;
        let mut total_realized = 0i64;
        let mut position_count = 0;

        for position in positions.values() {
            if position.net_size > 0 {
                total_long += position.net_size;
            } else if position.net_size < 0 {
                total_short += position.net_size.abs();
            }
            total_realized += position.realized_pnl;
            position_count += 1;
        }

        InventorySummary {
            total_long,
            total_short,
            net_position: total_long - total_short,
            usdc_balance: *usdc_balance,
            total_realized_pnl: total_realized,
            position_count,
        }
    }

    /// Clear all positions (reset)
    pub async fn clear(&self) {
        let mut positions = self.positions.write().await;
        let mut markets = self.markets.write().await;
        positions.clear();
        markets.clear();
    }
}

/// Trade side
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

/// Inventory summary
#[derive(Debug, Clone)]
pub struct InventorySummary {
    pub total_long: i64,
    pub total_short: i64,
    pub net_position: i64,
    pub usdc_balance: u64,
    pub total_realized_pnl: i64,
    pub position_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_inventory_tracking() {
        let tracker = InventoryTracker::new(1_000_000); // Max 1 USDC
        
        // Buy YES
        tracker.record_fill("yes_token", "condition_1", Side::Buy, 100_000, 500_000).await.unwrap();
        
        let position = tracker.get_position("yes_token").await.unwrap();
        assert_eq!(position.net_size, 100_000);
        
        // Check skew
        let skew = tracker.get_skew("condition_1").await;
        assert!(skew > 0.0); // Long YES = positive skew
    }

    #[tokio::test]
    async fn test_position_limits() {
        let tracker = InventoryTracker::new(100_000); // Max 0.1 USDC
        
        // Should be within limits initially
        assert!(tracker.is_within_limits("new_token").await);
        
        // Add position
        tracker.record_fill("new_token", "cond", Side::Buy, 50_000, 500_000).await.unwrap();
        assert!(tracker.is_within_limits("new_token").await);
        
        // Add more - should exceed limits
        tracker.record_fill("new_token", "cond", Side::Buy, 100_000, 500_000).await.unwrap();
        assert!(!tracker.is_within_limits("new_token").await);
    }

    #[tokio::test]
    async fn test_skew_calculation() {
        let tracker = InventoryTracker::new(100_000);
        
        // Long YES only
        tracker.record_fill("yes_token", "cond", Side::Buy, 100_000, 500_000).await.unwrap();
        let skew = tracker.get_skew("cond").await;
        assert!(skew > 0.9); // Strong positive skew
        
        // Now also short NO (balanced)
        tracker.record_fill("no_token", "cond", Side::Sell, 100_000, 500_000).await.unwrap();
        // Skew should adjust
    }
}