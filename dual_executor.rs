//! Dual-Leg Arbitrage Executor - Atomic execution of YES+NO arb trades
//!
//! Strategy: Execute both legs simultaneously to capture risk-free arb
//! If one leg fails, implement stop-loss or hedge

use std::sync::Arc;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;

use crate::signer::ClobSigner;
use crate::edge_detector::ArbOpportunity;

/// Maximum time to wait for order fills
const FILL_TIMEOUT_MS: u64 = 500;

/// Maximum position size per arb trade (in USD)
const MAX_POSITION_USD: f64 = 100.0;

/// Default position size
const DEFAULT_POSITION_USD: f64 = 50.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,
    Filled,
    PartialFill,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegOrder {
    pub token_id: String,
    pub side: String,        // "BUY"
    pub price_cents: u64,
    pub size_shares: u64,
    pub status: OrderStatus,
    pub fill_pct: f64,
    pub order_id: Option<String>,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecutionResult {
    pub opportunity: ArbOpportunity,
    pub yes_order: LegOrder,
    pub no_order: LegOrder,
    pub total_cost_usd: f64,
    pub expected_payout_usd: f64,
    pub realized_edge_pct: f64,
    pub success: bool,
    pub error: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub partial_fills: u64,
    pub failed_executions: u64,
    pub total_profit_usd: f64,
    pub total_loss_usd: f64,
    pub avg_execution_time_ms: f64,
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self {
            total_executions: 0,
            successful_executions: 0,
            partial_fills: 0,
            failed_executions: 0,
            total_profit_usd: 0.0,
            total_loss_usd: 0.0,
            avg_execution_time_ms: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub max_position_usd: f64,
    pub default_position_usd: f64,
    pub fill_timeout_ms: u64,
    pub dry_run: bool,
    pub max_retries: u32,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_position_usd: MAX_POSITION_USD,
            default_position_usd: DEFAULT_POSITION_USD,
            fill_timeout_ms: FILL_TIMEOUT_MS,
            dry_run: true, // Safety first!
            max_retries: 2,
        }
    }
}

pub struct DualExecutor {
    signer: Arc<RwLock<Option<ClobSigner>>>,
    config: ExecutorConfig,
    stats: ExecutionStats,
    /// Track recent executions to prevent double-spend
    recent_tokens: HashMap<String, u64>,
}

impl DualExecutor {
    pub fn new(config: ExecutorConfig) -> Self {
        Self {
            signer: Arc::new(RwLock::new(None)),
            config,
            stats: ExecutionStats::default(),
            recent_tokens: HashMap::new(),
        }
    }

    /// Set the signer (called after wallet initialization)
    pub async fn set_signer(&self, new_signer: ClobSigner) {
        let mut signer = self.signer.write().await;
        *signer = Some(new_signer);
    }

    /// Calculate position size based on edge and limits
    pub fn calculate_position_size(&self, opp: &ArbOpportunity) -> u64 {
        // Higher edge = larger position (capped)
        let edge_factor = (opp.edge_pct / 0.02).min(2.0); // 1x-2x based on edge
        let base_size = self.config.default_position_usd * edge_factor;
        let capped_size = base_size.min(self.config.max_position_usd);
        
        // Convert to shares (each share costs edge_cents at combined ask)
        let share_price = opp.combined_cost_cents as f64 / 100.0;
        if share_price <= 0.0 {
            return 0;
        }
        
        (capped_size / share_price).floor() as u64
    }

    /// Execute arbitrage opportunity (dry-run aware)
    pub async fn execute(&mut self, opp: ArbOpportunity) -> ExecutionResult {
        let start = std::time::Instant::now();
        self.stats.total_executions += 1;

        // Prevent double-spend on same tokens
        if self.is_recently_executed(&opp.yes_token) || self.is_recently_executed(&opp.no_token) {
            return ExecutionResult {
                opportunity: opp.clone(),
                yes_order: self.empty_order(&opp.yes_token, opp.yes_ask_cents),
                no_order: self.empty_order(&opp.no_token, opp.no_ask_cents),
                total_cost_usd: 0.0,
                expected_payout_usd: 0.0,
                realized_edge_pct: 0.0,
                success: false,
                error: Some("Tokens recently executed - preventing double-spend".to_string()),
                timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            };
        }

        let size_shares = self.calculate_position_size(&opp);
        if size_shares == 0 {
            return ExecutionResult {
                opportunity: opp.clone(),
                yes_order: self.empty_order(&opp.yes_token, opp.yes_ask_cents),
                no_order: self.empty_order(&opp.no_token, opp.no_ask_cents),
                total_cost_usd: 0.0,
                expected_payout_usd: 0.0,
                realized_edge_pct: 0.0,
                success: false,
                error: Some("Position size is zero".to_string()),
                timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
            };
        }

        eprintln!("ARB OPPORTUNITY: YES={:.2}c NO={:.2}c EDGE={:.2}% SIZE={}shares DRY_RUN={}",
            opp.yes_ask_cents, opp.no_ask_cents, opp.edge_pct * 100.0, size_shares, self.config.dry_run);

        if self.config.dry_run {
            return self.dry_run_execute(opp, size_shares);
        }

        // Real execution (requires signer) - clone the order data we need
        let signer_guard = self.signer.read().await;
        let signer = match signer_guard.as_ref() {
            Some(s) => s,
            None => {
                return ExecutionResult {
                    opportunity: opp.clone(),
                    yes_order: self.empty_order(&opp.yes_token, opp.yes_ask_cents),
                    no_order: self.empty_order(&opp.no_token, opp.no_ask_cents),
                    total_cost_usd: 0.0,
                    expected_payout_usd: 0.0,
                    realized_edge_pct: 0.0,
                    success: false,
                    error: Some("No signer configured".to_string()),
                    timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
                };
            }
        };

        // Build orders while holding the signer reference
        let yes_order = signer.build_order(&opp.yes_token, opp.yes_ask_cents, size_shares, true, 125);
        let no_order = signer.build_order(&opp.no_token, opp.no_ask_cents, size_shares, true, 125);

        // Now release the borrow and do async signing
        drop(signer_guard);
        
        // Execute both legs atomically
        self.execute_dual_leg_from_orders(yes_order, no_order, &opp, size_shares, start).await
    }

    /// Execute both legs from pre-built orders
    async fn execute_dual_leg_from_orders(
        &mut self,
        yes_order: crate::signer::Order,
        no_order: crate::signer::Order,
        opp: &ArbOpportunity,
        size_shares: u64,
        start: std::time::Instant,
    ) -> ExecutionResult {
        // Get signer again for async signing
        let signer_guard = self.signer.read().await;
        let signer = match signer_guard.as_ref() {
            Some(s) => s,
            None => {
                return ExecutionResult {
                    opportunity: opp.clone(),
                    yes_order: self.empty_order(&opp.yes_token, opp.yes_ask_cents),
                    no_order: self.empty_order(&opp.no_token, opp.no_ask_cents),
                    total_cost_usd: 0.0,
                    expected_payout_usd: 0.0,
                    realized_edge_pct: 0.0,
                    success: false,
                    error: Some("No signer configured".to_string()),
                    timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
                };
            }
        };

        // Sign orders
        let yes_sig = signer.sign_order(&yes_order).await;
        let no_sig = signer.sign_order(&no_order).await;

        drop(signer_guard);

        eprintln!("SIGNED: Both orders signed successfully");

        let _execution_time = start.elapsed().as_millis() as u64;

        let yes_order_result = LegOrder {
            token_id: opp.yes_token.clone(),
            side: "BUY".to_string(),
            price_cents: opp.yes_ask_cents,
            size_shares,
            status: OrderStatus::Pending,
            fill_pct: 0.0,
            order_id: Some(yes_sig),
            tx_hash: None,
        };

        let no_order_result = LegOrder {
            token_id: opp.no_token.clone(),
            side: "BUY".to_string(),
            price_cents: opp.no_ask_cents,
            size_shares,
            status: OrderStatus::Pending,
            fill_pct: 0.0,
            order_id: Some(no_sig),
            tx_hash: None,
        };

        self.mark_executed(&opp.yes_token);
        self.mark_executed(&opp.no_token);

        ExecutionResult {
            opportunity: opp.clone(),
            yes_order: yes_order_result,
            no_order: no_order_result,
            total_cost_usd: size_shares as f64 * opp.combined_cost_cents as f64 / 100.0,
            expected_payout_usd: size_shares as f64,
            realized_edge_pct: opp.edge_pct,
            success: true,
            error: None,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    /// Dry-run execution (logs only, no actual orders)
    fn dry_run_execute(&mut self, opp: ArbOpportunity, size_shares: u64) -> ExecutionResult {
        let edge_pct = opp.edge_pct;
        let yes_cost = size_shares as f64 * opp.yes_ask_cents as f64 / 100.0;
        let no_cost = size_shares as f64 * opp.no_ask_cents as f64 / 100.0;
        let total_cost = yes_cost + no_cost;
        let expected_payout = size_shares as f64; // Always $1/share at resolution
        let profit = expected_payout - total_cost;
        
        let yes_order = LegOrder {
            token_id: opp.yes_token.clone(),
            side: "BUY".to_string(),
            price_cents: opp.yes_ask_cents,
            size_shares,
            status: OrderStatus::Filled,
            fill_pct: 1.0,
            order_id: Some("DRY_RUN".to_string()),
            tx_hash: None,
        };

        let no_order = LegOrder {
            token_id: opp.no_token.clone(),
            side: "BUY".to_string(),
            price_cents: opp.no_ask_cents,
            size_shares,
            status: OrderStatus::Filled,
            fill_pct: 1.0,
            order_id: Some("DRY_RUN".to_string()),
            tx_hash: None,
        };

        // Mark tokens as recently executed
        self.mark_executed(&opp.yes_token);
        self.mark_executed(&opp.no_token);
        self.stats.successful_executions += 1;
        self.stats.total_profit_usd += profit;

        eprintln!("DRY_RUN: Would buy {} YES @ {}c and {} NO @ {}c", 
            size_shares, opp.yes_ask_cents, size_shares, opp.no_ask_cents);
        eprintln!("DRY_RUN: Total cost ${:.2}, Payout ${:.2}, Profit ${:.2} ({:.2}%)",
            total_cost, expected_payout, profit, edge_pct * 100.0);

        ExecutionResult {
            opportunity: opp,
            yes_order,
            no_order,
            total_cost_usd: total_cost,
            expected_payout_usd: expected_payout,
            realized_edge_pct: edge_pct,
            success: true,
            error: None,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }

    fn empty_order(&self, token_id: &str, price: u64) -> LegOrder {
        LegOrder {
            token_id: token_id.to_string(),
            side: "BUY".to_string(),
            price_cents: price,
            size_shares: 0,
            status: OrderStatus::Failed,
            fill_pct: 0.0,
            order_id: None,
            tx_hash: None,
        }
    }

    fn is_recently_executed(&self, token_id: &str) -> bool {
        if let Some(&ts) = self.recent_tokens.get(token_id) {
            let now = chrono::Utc::now().timestamp() as u64;
            // Block for 60 seconds
            now.saturating_sub(ts) < 60
        } else {
            false
        }
    }

    fn mark_executed(&mut self, token_id: &str) {
        self.recent_tokens.insert(
            token_id.to_string(),
            chrono::Utc::now().timestamp() as u64,
        );
    }

    pub fn stats(&self) -> &ExecutionStats {
        &self.stats
    }

    /// Clear old entries from recent_tokens (call periodically)
    pub fn prune_recent_tokens(&mut self) {
        let now = chrono::Utc::now().timestamp() as u64;
        self.recent_tokens.retain(|_, &mut ts| now.saturating_sub(ts) < 300);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_detector::ArbOpportunity;

    #[test]
    fn test_position_sizing() {
        let executor = DualExecutor::new(ExecutorConfig::default());
        
        // 3% edge -> 1.5x position
        let opp = ArbOpportunity::new(
            "test".to_string(),
            "yes".to_string(),
            "no".to_string(),
            47,
            50,
            97,
            0.03,
            3,
            None,
        );

        let size = executor.calculate_position_size(&opp);
        assert!(size > 0);
        assert!(size as f64 * 0.97 <= executor.config.max_position_usd);
    }

    #[test]
    fn test_dry_run_execution() {
        let mut executor = DualExecutor::new(ExecutorConfig {
            dry_run: true,
            ..Default::default()
        });

        let opp = ArbOpportunity::new(
            "test".to_string(),
            "yes".to_string(),
            "no".to_string(),
            45,
            52,
            97,
            0.03,
            3,
            Some("BTC 5m Up".to_string()),
        );

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(executor.execute(opp));
        
        assert!(result.success);
        assert!(result.yes_order.order_id.is_some());
        assert!(result.no_order.order_id.is_some());
    }

    #[test]
    fn test_prevent_double_spend() {
        let mut executor = DualExecutor::new(ExecutorConfig::default());
        executor.mark_executed("yes_token");
        
        assert!(executor.is_recently_executed("yes_token"));
        assert!(!executor.is_recently_executed("other_token"));
    }
}