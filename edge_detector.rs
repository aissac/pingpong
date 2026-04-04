//! Arbitrage Edge Detector - Detects risk-free arb opportunities
//! 
//! Core Strategy: When YES_ask + NO_ask < $1.00, buy both for guaranteed payout
//! Edge = 100 - (yes_ask + no_ask) cents

use serde::{Serialize, Deserialize};

/// Minimum combined cost threshold (cents) to trigger arb
const EDGE_THRESHOLD_CENTS: u64 = 97;

/// Minimum edge percentage to execute
const MIN_EDGE_PCT: f64 = 0.02;

/// Maximum spread between bid and ask (in basis points)
const MAX_SPREAD_BPS: u64 = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbOpportunity {
    pub condition_id: String,
    pub yes_token: String,
    pub no_token: String,
    pub yes_ask_cents: u64,
    pub no_ask_cents: u64,
    pub combined_cost_cents: u64,
    pub edge_pct: f64,
    pub edge_cents: u64,
    pub timestamp_ms: u64,
    pub market_name: Option<String>,
}

impl ArbOpportunity {
    pub fn new(
        condition_id: String,
        yes_token: String,
        no_token: String,
        yes_ask_cents: u64,
        no_ask_cents: u64,
        combined_cost_cents: u64,
        edge_pct: f64,
        edge_cents: u64,
        market_name: Option<String>,
    ) -> Self {
        Self {
            condition_id,
            yes_token,
            no_token,
            yes_ask_cents,
            no_ask_cents,
            combined_cost_cents,
            edge_pct,
            edge_cents,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            market_name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderbookSnapshot {
    pub yes_ask: Option<u64>,
    pub yes_bid: Option<u64>,
    pub no_ask: Option<u64>,
    pub no_bid: Option<u64>,
    pub yes_liquidity: u64,
    pub no_liquidity: u64,
}

impl Default for OrderbookSnapshot {
    fn default() -> Self {
        Self {
            yes_ask: None,
            yes_bid: None,
            no_ask: None,
            no_bid: None,
            yes_liquidity: 0,
            no_liquidity: 0,
        }
    }
}

impl OrderbookSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_ask(&mut self, is_yes: bool, ask_cents: u64, liquidity: u64) {
        if is_yes {
            self.yes_ask = Some(ask_cents);
            self.yes_liquidity = liquidity;
        } else {
            self.no_ask = Some(ask_cents);
            self.no_liquidity = liquidity;
        }
    }

    pub fn update_bid(&mut self, is_yes: bool, bid_cents: u64) {
        if is_yes {
            self.yes_bid = Some(bid_cents);
        } else {
            self.no_bid = Some(bid_cents);
        }
    }

    pub fn has_both_asks(&self) -> bool {
        self.yes_ask.is_some() && self.no_ask.is_some()
    }

    pub fn combined_ask(&self) -> Option<u64> {
        match (self.yes_ask, self.no_ask) {
            (Some(yes), Some(no)) => Some(yes + no),
            _ => None,
        }
    }

    pub fn yes_spread_bps(&self) -> Option<u64> {
        match (self.yes_bid, self.yes_ask) {
            (Some(bid), Some(ask)) if ask > 0 && bid > 0 => {
                let spread = ask.abs_diff(bid);
                Some((spread * 10000) / ((bid + ask) / 2))
            }
            _ => None,
        }
    }

    pub fn no_spread_bps(&self) -> Option<u64> {
        match (self.no_bid, self.no_ask) {
            (Some(bid), Some(ask)) if ask > 0 && bid > 0 => {
                let spread = ask.abs_diff(bid);
                Some((spread * 10000) / ((bid + ask) / 2))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EdgeDetectorConfig {
    pub threshold_cents: u64,
    pub min_edge_pct: f64,
    pub max_spread_bps: u64,
    pub dry_run: bool,
}

impl Default for EdgeDetectorConfig {
    fn default() -> Self {
        Self {
            threshold_cents: EDGE_THRESHOLD_CENTS,
            min_edge_pct: MIN_EDGE_PCT,
            max_spread_bps: MAX_SPREAD_BPS,
            dry_run: true,
        }
    }
}

pub struct EdgeDetector {
    config: EdgeDetectorConfig,
    opportunities_detected: u64,
    opportunities_executed: u64,
    total_edge_captured_cents: u64,
}

impl EdgeDetector {
    pub fn new(config: EdgeDetectorConfig) -> Self {
        Self {
            config,
            opportunities_detected: 0,
            opportunities_executed: 0,
            total_edge_captured_cents: 0,
        }
    }

    pub fn detect_edge(
        &mut self,
        snapshot: &OrderbookSnapshot,
        condition_id: &str,
        yes_token: &str,
        no_token: &str,
        market_name: Option<&str>,
    ) -> Option<ArbOpportunity> {
        let yes_ask = snapshot.yes_ask?;
        let no_ask = snapshot.no_ask?;

        let combined = yes_ask + no_ask;
        
        if combined >= self.config.threshold_cents {
            return None;
        }

        if let (Some(yes_bps), Some(no_bps)) = (snapshot.yes_spread_bps(), snapshot.no_spread_bps()) {
            if yes_bps > self.config.max_spread_bps || no_bps > self.config.max_spread_bps {
                return None;
            }
        }

        let edge_cents = 100 - combined;
        let edge_pct = edge_cents as f64 / 100.0;

        if edge_pct < self.config.min_edge_pct {
            return None;
        }

        self.opportunities_detected += 1;

        Some(ArbOpportunity::new(
            condition_id.to_string(),
            yes_token.to_string(),
            no_token.to_string(),
            yes_ask,
            no_ask,
            combined,
            edge_pct,
            edge_cents,
            market_name.map(|s| s.to_string()),
        ))
    }

    pub fn record_execution(&mut self, edge_cents: u64) {
        self.opportunities_executed += 1;
        self.total_edge_captured_cents += edge_cents;
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        (self.opportunities_detected, self.opportunities_executed, self.total_edge_captured_cents)
    }

    pub fn estimate_profit(&self, opp: &ArbOpportunity, size_usd: f64) -> f64 {
        let edge_decimal = opp.edge_cents as f64 / 100.0;
        size_usd * edge_decimal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_edge_3_percent() {
        let mut detector = EdgeDetector::new(EdgeDetectorConfig::default());
        let mut snap = OrderbookSnapshot::new();
        snap.yes_ask = Some(47);
        snap.no_ask = Some(50);
        let opp = detector.detect_edge(&snap, "cond1", "yes_tok", "no_tok", None);
        assert!(opp.is_some());
        let opp = opp.unwrap();
        assert_eq!(opp.edge_cents, 3);
        assert!((opp.edge_pct - 0.03).abs() < 0.001);
    }

    #[test]
    fn test_combined_ask() {
        let mut snap = OrderbookSnapshot::new();
        assert!(!snap.has_both_asks());
        assert!(snap.combined_ask().is_none());
        snap.yes_ask = Some(45);
        assert!(!snap.has_both_asks());
        snap.no_ask = Some(52);
        assert!(snap.has_both_asks());
        assert_eq!(snap.combined_ask(), Some(97));
    }
}