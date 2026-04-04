//! Fee-Aware Edge Detection for Polymarket
//!
//! CRITICAL: Polymarket charges dynamic fees that MUST be factored into arb calculations.
//! As of March 30, 2026: Taker fee = 1.80% for crypto markets
//!
//! The dump-and-hedge pattern:
//! 1. Detect 15%+ price drop within 3 seconds (THE DUMP)
//! 2. Fire Taker BUY immediately on dumped side
//! 3. Wait for opposite ask to drop (THE HEDGE)
//! 4. When combined cost < threshold, fire second BUY
//! 5. If hedge doesn't fill in 5 min, stop-loss (sell at market)

use std::time::Instant;

/// Polymarket V2 Fee Structure (March 2026)
/// Taker fee: 1.80% on crypto markets
/// Maker rebate: 0.36% (20% of taker fee)
pub const TAKER_FEE_PCT: f64 = 0.018;  // 1.80%
pub const MAKER_REBATE_PCT: f64 = 0.0036; // 0.36%

/// Minimum combined cost AFTER fees for viable arb
/// If combined < $0.96, after 1.8% * 2 = ~$0.98 real cost, profit ~$0.02
pub const FEE_ADJUSTED_THRESHOLD_CENTS: u64 = 96;

/// Dump detection threshold (15% drop in 3 seconds)
pub const DUMP_THRESHOLD_PCT: f64 = 0.15;
pub const DUMP_WINDOW_SECS: u64 = 3;

/// Hedge timeout (5 minutes max wait for second leg)
pub const HEDGE_TIMEOUT_SECS: u64 = 300;

/// Stop-loss: Sell at market if hedge unfilled after timeout
pub const STOP_LOSS_AFTER_TIMEOUT: bool = true;

#[derive(Debug, Clone)]
pub struct FeeAdjustedOpportunity {
    pub condition_id: String,
    pub yes_token: String,
    pub no_token: String,
    /// Side that was dumped (BUY this first)
    pub dump_side: String,
    /// Raw ask prices (cents)
    pub yes_ask_cents: u64,
    pub no_ask_cents: u64,
    /// Combined cost before fees (cents)
    pub combined_raw_cents: u64,
    /// Combined cost after taker fees (cents)
    pub combined_fee_adjusted_cents: f64,
    /// Effective edge after fees (0.028 = 2.8%)
    pub edge_after_fees_pct: f64,
    /// Fee cost in cents
    pub fee_cost_cents: f64,
    /// Whether this is DUMP phase or HEDGE phase
    pub phase: ArbPhase,
    /// Timestamp when dump was detected
    pub dump_detected_at: Option<Instant>,
    /// How long since dump (for hedge timeout)
    pub seconds_since_dump: Option<u64>,
    pub market_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArbPhase {
    /// Detected dump, ready to fire first leg
    Dump,
    /// First leg filled, waiting for hedge
    HedgePending,
    /// Both legs filled, arb complete
    Complete,
    /// Hedge timeout, stop-loss triggered
    StopLoss,
    /// No opportunity
    None,
}

#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub price_cents: u64,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Default)]
pub struct DumpDetector {
    /// Price history for YES token
    pub yes_history: Vec<PriceHistory>,
    /// Price history for NO token
    pub no_history: Vec<PriceHistory>,
    /// Maximum history window
    max_history_secs: u64,
}

impl DumpDetector {
    pub fn new() -> Self {
        Self {
            yes_history: Vec::with_capacity(100),
            no_history: Vec::with_capacity(100),
            max_history_secs: 30, // Keep 30 seconds of history
        }
    }

    /// Record price update and check for dump
    pub fn update_price(&mut self, is_yes: bool, price_cents: u64) -> Option<DumpInfo> {
        let now = Instant::now();
        let history = if is_yes { &mut self.yes_history } else { &mut self.no_history };
        
        // Add new price
        history.push(PriceHistory { price_cents, timestamp: now });
        
        // Prune old entries
        history.retain(|p| now.duration_since(p.timestamp).as_secs() < self.max_history_secs);
        
        // Check for dump (15% drop in last 3 seconds)
        self.detect_dump(is_yes, price_cents)
    }

    fn detect_dump(&self, is_yes: bool, current_price: u64) -> Option<DumpInfo> {
        let history = if is_yes { &self.yes_history } else { &self.no_history };
        let now = Instant::now();
        
        // Find price 3 seconds ago
        let price_3s_ago = history.iter()
            .filter(|p| now.duration_since(p.timestamp).as_secs() >= DUMP_WINDOW_SECS)
            .map(|p| p.price_cents)
            .last();
        
        if let Some(old_price) = price_3s_ago {
            if old_price > 0 {
                let drop_pct = (old_price as f64 - current_price as f64) / old_price as f64;
                
                if drop_pct >= DUMP_THRESHOLD_PCT {
                    return Some(DumpInfo {
                        side: if is_yes { "YES" } else { "NO" },
                        old_price_cents: old_price,
                        new_price_cents: current_price,
                        drop_pct,
                        detected_at: now,
                    });
                }
            }
        }
        
        None
    }

    /// Clear history (call on market rollover)
    pub fn clear(&mut self) {
        self.yes_history.clear();
        self.no_history.clear();
    }
}

#[derive(Debug, Clone)]
pub struct DumpInfo {
    pub side: &'static str,
    pub old_price_cents: u64,
    pub new_price_cents: u64,
    pub drop_pct: f64,
    pub detected_at: Instant,
}

/// Calculate the ACTUAL cost after Polymarket fees
/// 
/// Fee calculation:
/// - Taker fee: 1.80% on each buy order
/// - Total fee = (yes_cost * 0.018) + (no_cost * 0.018)
/// - Effective combined = (yes + no) * 1.018
pub fn calculate_fee_adjusted_cost(yes_ask_cents: u64, no_ask_cents: u64, size_shares: u64) -> f64 {
    let yes_cost = yes_ask_cents as f64 * size_shares as f64 / 100.0;
    let no_cost = no_ask_cents as f64 * size_shares as f64 / 100.0;
    let combined = yes_cost + no_cost;
    
    // Apply taker fee to both legs
    combined * (1.0 + TAKER_FEE_PCT)
}

/// Calculate edge after fees
/// 
/// Edge = (Payout - Cost_with_fees) / Payout
/// Payout is always $1.00 per share at resolution
pub fn calculate_edge_after_fees(yes_ask_cents: u64, no_ask_cents: u64) -> f64 {
    let combined_raw = (yes_ask_cents + no_ask_cents) as f64 / 100.0;
    let combined_with_fees = combined_raw * (1.0 + TAKER_FEE_PCT);
    
    let payout = 1.0; // $1.00 guaranteed
    let edge = payout - combined_with_fees;
    
    edge / payout
}

/// Check if combined cost is viable for arb AFTER fees
pub fn is_viable_after_fees(yes_ask_cents: u64, no_ask_cents: u64) -> bool {
    let edge = calculate_edge_after_fees(yes_ask_cents, no_ask_cents);
    edge > 0.02 // Minimum 2% edge after fees
}

/// Fee-aware edge detector
pub struct FeeAwareEdgeDetector {
    pub dump_detector: DumpDetector,
    /// Minimum edge after fees
    min_edge_pct: f64,
    /// Pending hedge (first leg filled, waiting for second)
    pending_hedge: Option<PendingHedge>,
}

#[derive(Debug, Clone)]
pub struct PendingHedge {
    pub condition_id: String,
    pub first_side: String,
    pub first_token: String,
    pub first_price_cents: u64,
    pub first_size_shares: u64,
    pub first_filled_at: Instant,
    pub hedge_token: String,
    pub target_hedge_price_cents: u64,
}

impl FeeAwareEdgeDetector {
    pub fn new(min_edge_pct: f64) -> Self {
        Self {
            dump_detector: DumpDetector::new(),
            min_edge_pct,
            pending_hedge: None,
        }
    }

    /// Process orderbook update and check for dump or hedge opportunity
    pub fn process_update(
        &mut self,
        yes_ask: u64,
        no_ask: u64,
        condition_id: &str,
        yes_token: &str,
        no_token: &str,
        market_name: Option<&str>,
    ) -> Option<FeeAdjustedOpportunity> {
        // Update dump detector
        let yes_dump = self.dump_detector.update_price(true, yes_ask);
        let no_dump = self.dump_detector.update_price(false, no_ask);

        // Check for pending hedge timeout
        if let Some(ref hedge) = self.pending_hedge {
            let elapsed = hedge.first_filled_at.elapsed().as_secs();
            if elapsed > HEDGE_TIMEOUT_SECS {
                // Stop-loss required
                return Some(FeeAdjustedOpportunity {
                    condition_id: condition_id.to_string(),
                    yes_token: yes_token.to_string(),
                    no_token: no_token.to_string(),
                    dump_side: hedge.first_side.clone(),
                    yes_ask_cents: yes_ask,
                    no_ask_cents: no_ask,
                    combined_raw_cents: yes_ask + no_ask,
                    combined_fee_adjusted_cents: calculate_fee_adjusted_cost(yes_ask, no_ask, 1),
                    edge_after_fees_pct: calculate_edge_after_fees(yes_ask, no_ask),
                    fee_cost_cents: (yes_ask + no_ask) as f64 * TAKER_FEE_PCT,
                    phase: ArbPhase::StopLoss,
                    dump_detected_at: Some(hedge.first_filled_at),
                    seconds_since_dump: Some(elapsed),
                    market_name: market_name.map(|s| s.to_string()),
                });
            }
        }

        // Check for dump opportunity
        if let Some(dump) = yes_dump {
            // YES dropped 15%+, BUY YES immediately
            return Some(FeeAdjustedOpportunity {
                condition_id: condition_id.to_string(),
                yes_token: yes_token.to_string(),
                no_token: no_token.to_string(),
                dump_side: "YES".to_string(),
                yes_ask_cents: yes_ask,
                no_ask_cents: no_ask,
                combined_raw_cents: yes_ask + no_ask,
                combined_fee_adjusted_cents: calculate_fee_adjusted_cost(yes_ask, no_ask, 1),
                edge_after_fees_pct: calculate_edge_after_fees(yes_ask, no_ask),
                fee_cost_cents: (yes_ask + no_ask) as f64 * TAKER_FEE_PCT,
                phase: ArbPhase::Dump,
                dump_detected_at: Some(dump.detected_at),
                seconds_since_dump: None,
                market_name: market_name.map(|s| s.to_string()),
            });
        }

        if let Some(dump) = no_dump {
            // NO dropped 15%+, BUY NO immediately
            return Some(FeeAdjustedOpportunity {
                condition_id: condition_id.to_string(),
                yes_token: yes_token.to_string(),
                no_token: no_token.to_string(),
                dump_side: "NO".to_string(),
                yes_ask_cents: yes_ask,
                no_ask_cents: no_ask,
                combined_raw_cents: yes_ask + no_ask,
                combined_fee_adjusted_cents: calculate_fee_adjusted_cost(yes_ask, no_ask, 1),
                edge_after_fees_pct: calculate_edge_after_fees(yes_ask, no_ask),
                fee_cost_cents: (yes_ask + no_ask) as f64 * TAKER_FEE_PCT,
                phase: ArbPhase::Dump,
                dump_detected_at: Some(dump.detected_at),
                seconds_since_dump: None,
                market_name: market_name.map(|s| s.to_string()),
            });
        }

        // Check for simultaneous arb opportunity (both sides already cheap)
        let edge = calculate_edge_after_fees(yes_ask, no_ask);
        if edge >= self.min_edge_pct && yes_ask + no_ask < FEE_ADJUSTED_THRESHOLD_CENTS {
            // Simultaneous arb - fire both at once
            return Some(FeeAdjustedOpportunity {
                condition_id: condition_id.to_string(),
                yes_token: yes_token.to_string(),
                no_token: no_token.to_string(),
                dump_side: "BOTH".to_string(), // Fire both simultaneously
                yes_ask_cents: yes_ask,
                no_ask_cents: no_ask,
                combined_raw_cents: yes_ask + no_ask,
                combined_fee_adjusted_cents: calculate_fee_adjusted_cost(yes_ask, no_ask, 1),
                edge_after_fees_pct: edge,
                fee_cost_cents: (yes_ask + no_ask) as f64 * TAKER_FEE_PCT,
                phase: ArbPhase::Dump, // Treat as immediate dump
                dump_detected_at: None,
                seconds_since_dump: None,
                market_name: market_name.map(|s| s.to_string()),
            });
        }

        // Check for hedge opportunity (if we have pending first leg)
        if let Some(ref hedge) = self.pending_hedge {
            let hedge_price = if hedge.first_side == "YES" { no_ask } else { yes_ask };
            let combined = hedge.first_price_cents + hedge_price;
            
            if is_viable_after_fees(hedge.first_price_cents, hedge_price) {
                return Some(FeeAdjustedOpportunity {
                    condition_id: condition_id.to_string(),
                    yes_token: yes_token.to_string(),
                    no_token: no_token.to_string(),
                    dump_side: hedge.first_side.clone(),
                    yes_ask_cents: yes_ask,
                    no_ask_cents: no_ask,
                    combined_raw_cents: combined,
                    combined_fee_adjusted_cents: calculate_fee_adjusted_cost(hedge.first_price_cents, hedge_price, 1),
                    edge_after_fees_pct: calculate_edge_after_fees(hedge.first_price_cents, hedge_price),
                    fee_cost_cents: combined as f64 * TAKER_FEE_PCT,
                    phase: ArbPhase::HedgePending,
                    dump_detected_at: Some(hedge.first_filled_at),
                    seconds_since_dump: Some(hedge.first_filled_at.elapsed().as_secs()),
                    market_name: market_name.map(|s| s.to_string()),
                });
            }
        }

        None
    }

    /// Record that first leg was filled (start hedge timer)
    pub fn record_first_leg(&mut self, hedge: PendingHedge) {
        self.pending_hedge = Some(hedge);
    }

    /// Clear pending hedge (both legs complete)
    pub fn clear_pending_hedge(&mut self) {
        self.pending_hedge = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        // YES=$0.48, NO=$0.48 -> combined $0.96
        // After 1.8% fee on each: $0.96 * 1.018 = $0.977
        let cost = calculate_fee_adjusted_cost(48, 48, 1);
        assert!((cost - 0.977).abs() < 0.01);
    }

    #[test]
    fn test_edge_after_fees() {
        // Combined $0.97, after fees ~$0.989
        // Edge = $1.00 - $0.989 = $0.011 = 1.1%
        let edge = calculate_edge_after_fees(48, 49); // 48 + 49 = 97 cents
        assert!(edge > 0.01 && edge < 0.02);
    }

    #[test]
    fn test_viable_after_fees() {
        // Combined $0.96 with fees ~$0.977 -> edge ~2.3%
        assert!(is_viable_after_fees(48, 48));
        
        // Combined $0.99 with fees ~$1.008 -> negative edge
        assert!(!is_viable_after_fees(50, 49));
    }

    #[test]
    fn test_dump_detection() {
        let mut detector = DumpDetector::new();
        
        // Start at 50 cents
        detector.update_price(true, 50);
        
        // Small changes - no dump
        assert!(detector.update_price(true, 48).is_none());
        
        // Need time to pass for dump detection
        // This test would need mocked time in real implementation
    }
}