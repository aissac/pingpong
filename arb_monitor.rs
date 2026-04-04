//! Arbitrage Monitor - WebSocket-based orderbook monitoring
//! 
//! Subscribes to YES/NO token pairs and tracks combined ask prices
//! Triggers edge detection when conditions are met

use std::collections::HashMap;
use crossbeam_channel::Sender;
use std::time::Instant;

use crate::edge_detector::{EdgeDetector, OrderbookSnapshot, ArbOpportunity};
use crate::hot_path::EngineEvent;

/// Maximum age for orderbook data before considered stale
const STALE_THRESHOLD_MS: u64 = 5000;

#[derive(Debug, Clone)]
pub struct MarketPair {
    pub condition_id: String,
    pub yes_token: String,
    pub no_token: String,
    pub market_name: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct OrderbookState {
    pub snapshot: OrderbookSnapshot,
    pub last_yes_update: Instant,
    pub last_no_update: Instant,
}

impl OrderbookState {
    pub fn new() -> Self {
        Self {
            snapshot: OrderbookSnapshot::new(),
            last_yes_update: Instant::now(),
            last_no_update: Instant::now(),
        }
    }

    pub fn is_stale(&self) -> bool {
        let now = Instant::now();
        let yes_age = now.duration_since(self.last_yes_update).as_millis() as u64;
        let no_age = now.duration_since(self.last_no_update).as_millis() as u64;
        yes_age > STALE_THRESHOLD_MS || no_age > STALE_THRESHOLD_MS
    }
}

pub struct ArbMonitor {
    /// Token -> OrderbookState mapping
    pub orderbooks: HashMap<String, OrderbookState>,
    /// Condition ID -> MarketPair mapping
    pub markets: HashMap<String, MarketPair>,
    /// Token -> Condition ID mapping (for quick lookup)
    token_to_condition: HashMap<String, String>,
    /// Edge detector
    detector: EdgeDetector,
    /// Detected opportunities channel
    opportunity_tx: Sender<ArbOpportunity>,
    /// Statistics
    updates_processed: u64,
    opportunities_found: u64,
    stale_count: u64,
}

impl ArbMonitor {
    pub fn new(opportunity_tx: Sender<ArbOpportunity>) -> Self {
        Self {
            orderbooks: HashMap::new(),
            markets: HashMap::new(),
            token_to_condition: HashMap::new(),
            detector: EdgeDetector::new(Default::default()),
            opportunity_tx,
            updates_processed: 0,
            opportunities_found: 0,
            stale_count: 0,
        }
    }

    /// Register a new market pair for monitoring
    pub fn register_market(&mut self, pair: MarketPair) {
        eprintln!("📊 Registering market: {} (YES={}, NO={})", 
            pair.market_name, 
            &pair.yes_token[..8.min(pair.yes_token.len())],
            &pair.no_token[..8.min(pair.no_token.len())]);

        // Initialize orderbook states
        self.orderbooks.insert(pair.yes_token.clone(), OrderbookState::new());
        self.orderbooks.insert(pair.no_token.clone(), OrderbookState::new());

        // Create reverse mappings
        self.token_to_condition.insert(pair.yes_token.clone(), pair.condition_id.clone());
        self.token_to_condition.insert(pair.no_token.clone(), pair.condition_id.clone());

        // Store market pair
        self.markets.insert(pair.condition_id.clone(), pair);
    }

    /// Handle WebSocket orderbook update
    pub fn process_orderbook_update(&mut self, token_id: &str, raw_json: &str) -> Option<ArbOpportunity> {
        self.updates_processed += 1;

        // Parse the orderbook JSON
        let json: serde_json::Value = match serde_json::from_str(raw_json) {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Failed to parse orderbook JSON: {}", e);
                return None;
            }
        };

        // Get condition ID for this token
        let condition_id = match self.token_to_condition.get(token_id) {
            Some(c) => c.clone(),
            None => {
                // Unknown token - skip
                return None;
            }
        };

        // Get market pair
        let market = match self.markets.get(&condition_id) {
            Some(m) => m.clone(),  // Clone to avoid borrow issues
            None => return None,
        };

        // Determine if this is YES or NO token
        let is_yes = token_id == market.yes_token;

        // Extract ask price from orderbook
        if let Some(asks) = json.get("asks").and_then(|v| v.as_array()) {
            if let Some(best_ask) = asks.first() {
                if let Some(price_str) = best_ask.get("price").and_then(|v| v.as_str()) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        let ask_cents = (price * 100.0).round() as u64;
                        
                        // Get liquidity at this price level
                        let liquidity = best_ask.get("size")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<f64>().ok())
                            .map(|v| v as u64)
                            .unwrap_or(0);

                        // Update orderbook state
                        if let Some(state) = self.orderbooks.get_mut(token_id) {
                            state.snapshot.update_ask(is_yes, ask_cents, liquidity);
                            if is_yes {
                                state.last_yes_update = Instant::now();
                            } else {
                                state.last_no_update = Instant::now();
                            }
                        }
                    }
                }
            }
        }

        // Extract bid price (for spread calculation)
        if let Some(bids) = json.get("bids").and_then(|v| v.as_array()) {
            if let Some(best_bid) = bids.first() {
                if let Some(price_str) = best_bid.get("price").and_then(|v| v.as_str()) {
                    if let Ok(price) = price_str.parse::<f64>() {
                        let bid_cents = (price * 100.0).round() as u64;
                        
                        if let Some(state) = self.orderbooks.get_mut(token_id) {
                            state.snapshot.update_bid(is_yes, bid_cents);
                        }
                    }
                }
            }
        }

        // Check for arbitrage opportunity
        self.check_arb_opportunity(&condition_id, &market)
    }

    /// Check if an arb opportunity exists for this market
    fn check_arb_opportunity(&mut self, condition_id: &str, market: &MarketPair) -> Option<ArbOpportunity> {
        // Get both orderbook states
        let yes_state = self.orderbooks.get(&market.yes_token)?;
        let no_state = self.orderbooks.get(&market.no_token)?;

        // Check for stale data
        if yes_state.is_stale() || no_state.is_stale() {
            self.stale_count += 1;
            return None;
        }

        // Create combined snapshot
        let mut combined = OrderbookSnapshot::new();
        combined.yes_ask = yes_state.snapshot.yes_ask;
        combined.yes_bid = yes_state.snapshot.yes_bid;
        combined.yes_liquidity = yes_state.snapshot.yes_liquidity;
        combined.no_ask = no_state.snapshot.no_ask;
        combined.no_bid = no_state.snapshot.no_bid;
        combined.no_liquidity = no_state.snapshot.no_liquidity;

        // Check for edge
        let opp = self.detector.detect_edge(
            &combined,
            condition_id,
            &market.yes_token,
            &market.no_token,
            Some(&market.market_name),
        );

        if let Some(ref opp) = opp {
            self.opportunities_found += 1;
            eprintln!("🎯 EDGE DETECTED: {} YES={:.2}c NO={:.2}c EDGE={:.2}%",
                market.market_name,
                opp.yes_ask_cents,
                opp.no_ask_cents,
                opp.edge_pct * 100.0);
            
            // Send to channel
            let _ = self.opportunity_tx.send(opp.clone());
        }

        opp
    }

    /// Get current stats
    pub fn stats(&self) -> (u64, u64, u64) {
        (self.updates_processed, self.opportunities_found, self.stale_count)
    }

    /// Prune stale markets
    pub fn prune_stale_markets(&mut self) {
        let stale_tokens: Vec<String> = self.orderbooks
            .iter()
            .filter(|(_, state)| state.is_stale())
            .map(|(token, _)| token.clone())
            .collect();

        for token in stale_tokens {
            self.orderbooks.remove(&token);
        }
    }

    /// Handle market rollover (new 5m/15m market)
    pub fn handle_market_rollover(&mut self, event: &EngineEvent) {
        if let EngineEvent::MarketRollover { asset_symbol, condition_id, yes_token, no_token } = event {
            // Remove old market if exists
            if let Some(old) = self.markets.get(condition_id) {
                self.orderbooks.remove(&old.yes_token);
                self.orderbooks.remove(&old.no_token);
                self.token_to_condition.remove(&old.yes_token);
                self.token_to_condition.remove(&old.no_token);
            }

            // Register new market
            let name = format!("{} 5m", asset_symbol);
            self.register_market(MarketPair {
                condition_id: condition_id.clone(),
                yes_token: yes_token.clone(),
                no_token: no_token.clone(),
                market_name: name,
                active: true,
            });
        }
    }
}

/// WebSocket message types for Polymarket CLOB
#[derive(Debug, Clone, serde::Deserialize)]
pub struct WsBookMessage {
    #[serde(rename = "event_type")]
    pub event_type: String,
    #[serde(rename = "market")]
    pub condition_id: String,
    #[serde(rename = "asset_id")]
    pub token_id: String,
    pub asks: Vec<PriceLevel>,
    pub bids: Vec<PriceLevel>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PriceLevel {
    pub price: String,
    pub size: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::bounded;

    #[test]
    fn test_register_market() {
        let (tx, _rx) = bounded(10);
        let mut monitor = ArbMonitor::new(tx);
        
        monitor.register_market(MarketPair {
            condition_id: "cond1".to_string(),
            yes_token: "yes1".to_string(),
            no_token: "no1".to_string(),
            market_name: "Test Market".to_string(),
            active: true,
        });

        assert!(monitor.markets.contains_key("cond1"));
        assert!(monitor.orderbooks.contains_key("yes1"));
        assert!(monitor.orderbooks.contains_key("no1"));
    }

    #[test]
    fn test_orderbook_update() {
        let (tx, _rx) = bounded(10);
        let mut monitor = ArbMonitor::new(tx);
        
        monitor.register_market(MarketPair {
            condition_id: "cond1".to_string(),
            yes_token: "yes1".to_string(),
            no_token: "no1".to_string(),
            market_name: "Test".to_string(),
            active: true,
        });

        // Update YES orderbook
        let json = r#"{"asks":[{"price":"0.47","size":"100"}],"bids":[{"price":"0.45","size":"200"}]}"#;
        let result = monitor.process_orderbook_update("yes1", json);
        assert!(result.is_none()); // No edge yet (need both sides)

        // Update NO orderbook with low ask to create edge
        let json = r#"{"asks":[{"price":"0.50","size":"100"}],"bids":[{"price":"0.48","size":"200"}]}"#;
        let result = monitor.process_orderbook_update("no1", json);
        // Combined = 47 + 50 = 97 = 3% edge
        // Should detect opportunity
        assert!(result.is_some());
    }

    #[test]
    fn test_stale_detection() {
        let mut state = OrderbookState::new();
        assert!(!state.is_stale()); // Fresh
        
        // Manually age the state
        state.last_yes_update = Instant::now() - std::time::Duration::from_secs(10);
        assert!(state.is_stale());
    }
}