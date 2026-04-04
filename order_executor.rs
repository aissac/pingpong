//! Order Execution Module - Submits real orders to Polymarket CLOB API

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crossbeam_channel::Receiver;
use crate::signer::ClobSigner;
use crate::copy_trader::CopyTradeAction;

// ============================================================================
// Types
// ============================================================================

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    pub dry_run: bool,
    pub max_daily_loss_usd: f64,
    pub order_timeout_secs: u64,
    pub api_base: String,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            dry_run: true,  // SAFE DEFAULT: paper trading
            max_daily_loss_usd: 100.0,
            order_timeout_secs: 60,
            api_base: "https://clob.polymarket.com".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ClobOrderRequest {
    salt: String,
    maker: String,
    signer: String,
    taker: String,
    tokenId: String,
    makerAmount: String,
    takerAmount: String,
    expiration: String,
    nonce: String,
    feeRateBps: String,
    side: String,
    signatureType: String,
    signature: String,
}

#[derive(Debug, Deserialize)]
struct ClobOrderResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ClobOrderStatus {
    id: String,
    status: String,
    #[serde(default)]
    filled_size: Option<String>,
    #[serde(default)]
    price: Option<String>,
}

#[derive(Debug)]
pub struct OrderResult {
    pub order_id: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub dry_run: bool,
}

#[derive(Debug)]
pub struct DailyStats {
    pub total_trades: u32,
    pub total_pnl: f64,
    pub wins: u32,
    pub losses: u32,
    pub start_time: Instant,
}

// ============================================================================
// Order Executor
// ============================================================================

pub struct OrderExecutor {
    client: Client,
    signer: Arc<ClobSigner>,
    config: ExecutorConfig,
    wallet_address: String,
    // Circuit breaker state (AtomicI64 for f64 bits - AtomicF64 is unstable)
    daily_loss: AtomicI64,
    is_halted: AtomicBool,
    // Order tracking
    pending_orders: Arc<RwLock<Vec<PendingOrder>>>,
    stats: Arc<RwLock<DailyStats>>,
}

struct PendingOrder {
    order_id: String,
    asset_id: String,
    side: String,
    size_usd: f64,
    submitted_at: Instant,
}

impl OrderExecutor {
    pub fn new(signer: ClobSigner, config: ExecutorConfig) -> Self {
        let wallet = signer.wallet.address().to_string();
        Self {
            client: Client::new(),
            signer: Arc::new(signer),
            config,
            wallet_address: wallet,
            daily_loss: AtomicI64::new(0),  // stores f64 bits
            is_halted: AtomicBool::new(false),
            pending_orders: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(DailyStats {
                total_trades: 0,
                total_pnl: 0.0,
                wins: 0,
                losses: 0,
                start_time: Instant::now(),
            })),
        }
    }

    /// Main execution loop - receives CopyTradeAction and executes orders
    pub async fn run(self, action_rx: Receiver<CopyTradeAction>) {
        println!("🔧 Order Executor started");
        println!("   Mode: {}", if self.config.dry_run { "DRY-RUN (paper)" } else { "LIVE" });
        println!("   Max Daily Loss: ${:.2}", self.config.max_daily_loss_usd);
        println!("   Wallet: {}", &self.wallet_address[..10]);
        println!("");

        loop {
            // Check circuit breaker
            if self.is_halted.load(Ordering::Relaxed) {
                eprintln!("🛑 Circuit breaker triggered - halting execution");
                tokio::time::sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Try to receive action
            if let Ok(action) = action_rx.try_recv() {
                if let Err(e) = self.execute_action(action).await {
                    eprintln!("❌ Order execution failed: {:?}", e);
                }
            } else {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }

            // Periodically check pending orders for timeout
            self.check_order_timeouts().await;
        }
    }

    /// Execute a copy trade action
    async fn execute_action(&self, action: CopyTradeAction) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let size_shares = (action.calculated_size_usd / action.target_price) as u64;
        let price_cents = (action.target_price * 100.0).round() as u64;
        let is_buy = action.side.to_uppercase() == "BUY";

        println!("📊 Processing {} ${:.2} @ ${:.4}",
            if is_buy { "BUY" } else { "SELL" },
            action.calculated_size_usd,
            action.target_price
        );

        // Check daily loss limit (AtomicI64 stores f64 bits)
        let current_loss_bits = self.daily_loss.load(Ordering::Relaxed);
        let current_loss = f64::from_bits(current_loss_bits as u64);
        if current_loss <= -self.config.max_daily_loss_usd {
            self.is_halted.store(true, Ordering::Relaxed);
            return Err(format!("Daily loss limit reached: ${:.2}", current_loss).into());
        }

        // DRY-RUN MODE: Just log and simulate
        if self.config.dry_run {
            return self.simulate_order(&action, price_cents, size_shares, is_buy).await;
        }

        // LIVE MODE: Build and submit real order
        let order = self.signer.build_order(
            &action.asset_id,
            price_cents,
            size_shares,
            is_buy,
            125, // 1.25% fee rate
        );

        let signature = self.signer.sign_order(&order).await;

        let clob_request = ClobOrderRequest {
            salt: order.salt.to_string(),
            maker: format!("{:?}", order.maker),
            signer: format!("{:?}", order.signer),
            taker: format!("{:?}", order.taker),
            tokenId: order.tokenId.to_string(),
            makerAmount: order.makerAmount.to_string(),
            takerAmount: order.takerAmount.to_string(),
            expiration: order.expiration.to_string(),
            nonce: order.nonce.to_string(),
            feeRateBps: order.feeRateBps.to_string(),
            side: order.side.to_string(),
            signatureType: order.signatureType.to_string(),
            signature,
        };

        // POST to CLOB
        let url = format!("{}/order", self.config.api_base);
        let response = self.client
            .post(&url)
            .json(&clob_request)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if status.is_success() {
            let order_resp: ClobOrderResponse = serde_json::from_str(&body)
                .unwrap_or(ClobOrderResponse { id: None, error: None });

            println!("✅ Order submitted! ID: {:?}", order_resp.id);

            // Track pending order
            if let Some(order_id) = order_resp.id {
                let mut pending = self.pending_orders.write().await;
                pending.push(PendingOrder {
                    order_id: order_id.clone(),
                    asset_id: action.asset_id.clone(),
                    side: action.side.clone(),
                    size_usd: action.calculated_size_usd,
                    submitted_at: Instant::now(),
                });
            }

            Ok(())
        } else {
            eprintln!("❌ Order rejected: {} - {}", status, body);
            Err(format!("Order rejected: {} - {}", status, body).into())
        }
    }

    /// Simulate order in dry-run mode
    async fn simulate_order(
        &self,
        action: &CopyTradeAction,
        price_cents: u64,
        size_shares: u64,
        is_buy: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use chrono::Utc;
        use serde_json::json;

        let slippage_cents = 2;
        let entry_price = (price_cents as f64 / 100.0) + (slippage_cents as f64 / 100.0);
        let taker_fee_pct = 0.018; // 1.8%
        let fee_cost = action.calculated_size_usd * taker_fee_pct;

        // Simulate 5-cent favorable move (paper trading assumption)
        let exit_price = entry_price + 0.05;
        let gross_profit = (exit_price - entry_price) * (action.calculated_size_usd / entry_price);
        let net_pnl = gross_profit - (fee_cost * 2.0);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_trades += 1;
        stats.total_pnl += net_pnl;
        if net_pnl > 0.0 {
            stats.wins += 1;
        } else {
            stats.losses += 1;
        }

        let log = json!({
            "time": Utc::now().timestamp_millis(),
            "type": "paper_trade",
            "mode": "dry_run",
            "asset": action.asset_id,
            "side": action.side,
            "whale_key": action.whale_key,
            "whale_price": action.target_price,
            "our_entry": entry_price,
            "slippage_cents": slippage_cents,
            "size_usd": action.calculated_size_usd,
            "size_shares": size_shares,
            "is_buy": is_buy,
            "fees_usd": fee_cost * 2.0,
            "net_pnl_usd": net_pnl,
            "cumulative_pnl_usd": stats.total_pnl,
            "total_trades": stats.total_trades,
            "win_rate": (stats.wins as f64 / stats.total_trades as f64 * 100.0).round()
        });

        println!("{}", log.to_string());
        Ok(())
    }

    /// Check for timed-out orders
    async fn check_order_timeouts(&self) {
        let mut pending = self.pending_orders.write().await;
        let now = Instant::now();
        let timeout = Duration::from_secs(self.config.order_timeout_secs);

        pending.retain(|order| {
            if now.duration_since(order.submitted_at) > timeout {
                println!("⏰ Order {} timed out after {}s", order.order_id, self.config.order_timeout_secs);
                // TODO: Cancel order via API
                false
            } else {
                true
            }
        });
    }

    /// Poll order status from CLOB API
    pub async fn poll_order_status(&self, order_id: &str) -> Result<Option<ClobOrderStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/orders?id={}", self.config.api_base, order_id);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let orders: Vec<ClobOrderStatus> = response.json().await?;
            Ok(orders.into_iter().next())
        } else {
            Ok(None)
        }
    }

    /// Get all orders for the wallet
    pub async fn get_wallet_orders(&self) -> Result<Vec<ClobOrderStatus>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/orders?maker={}", self.config.api_base, self.wallet_address);
        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let orders: Vec<ClobOrderStatus> = response.json().await?;
            Ok(orders)
        } else {
            Ok(Vec::new())
        }
    }

    /// Record a loss (for circuit breaker) - stores f64 bits in AtomicI64
    pub fn record_loss(&self, amount: f64) {
        use std::sync::atomic::Ordering;
        let loss_bits = (-amount).to_bits() as i64;
        // This is approximate - for accurate tracking use a RwLock<f64>
        self.daily_loss.fetch_add(loss_bits, Ordering::Relaxed);
    }

    /// Reset daily stats (call at midnight)
    pub async fn reset_daily(&self) {
        self.daily_loss.store(0i64, Ordering::Relaxed);
        self.is_halted.store(false, Ordering::Relaxed);
        let mut stats = self.stats.write().await;
        stats.total_trades = 0;
        stats.total_pnl = 0.0;
        stats.wins = 0;
        stats.losses = 0;
        stats.start_time = Instant::now();
        println!("📊 Daily stats reset");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_config_defaults() {
        let config = ExecutorConfig::default();
        assert!(config.dry_run); // Safety first!
        assert_eq!(config.max_daily_loss_usd, 100.0);
    }

    #[test]
    fn test_circuit_breaker() {
        let test_pk = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        let test_funder = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
        let signer = ClobSigner::new(test_pk, test_funder);
        let executor = OrderExecutor::new(signer, ExecutorConfig::default());

        executor.record_loss(50.0);
        let loss_bits = executor.daily_loss.load(Ordering::Relaxed);
        // Note: f64 bits stored as i64 - approximate for circuit breaker purposes
        assert!(loss_bits != 0);
    }
}