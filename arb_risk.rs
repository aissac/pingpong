//! Arbitrage-Specific Risk Management
//!
//! Risk limits specific to arbitrage strategy execution

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Risk limits for arbitrage execution
#[derive(Debug, Clone)]
pub struct ArbRiskLimits {
    /// Maximum capital per trade (USD)
    pub max_capital_per_trade: f64,
    /// Maximum capital per day (USD)
    pub max_capital_per_day: f64,
    /// Daily loss limit as percentage (0.03 = 3%)
    pub daily_loss_limit_pct: f64,
    /// Weekly loss limit as percentage (0.08 = 8%)
    pub weekly_loss_limit_pct: f64,
    /// Minimum edge percentage to execute (0.02 = 2%)
    pub min_edge_pct: f64,
    /// Maximum spread in basis points (200 = 2%)
    pub max_spread_bps: u64,
    /// Maximum concurrent positions
    pub max_concurrent_positions: u32,
    /// Maximum position age (seconds)
    pub max_position_age_secs: u64,
}

impl Default for ArbRiskLimits {
    fn default() -> Self {
        Self {
            max_capital_per_trade: 100.0,
            max_capital_per_day: 2000.0,
            daily_loss_limit_pct: 0.03,
            weekly_loss_limit_pct: 0.08,
            min_edge_pct: 0.02,
            max_spread_bps: 200,
            max_concurrent_positions: 5,
            max_position_age_secs: 300, // 5 minutes
        }
    }
}

/// Daily statistics for risk management (thread-safe)
#[derive(Debug)]
pub struct DailyStats {
    pub trades_executed: AtomicU64,
    pub capital_deployed: AtomicI64,  // In micro-dollars
    pub profit_usd: AtomicI64,         // In micro-dollars
    pub loss_usd: AtomicI64,           // In micro-dollars
    pub start_of_day: Mutex<Instant>,
}

impl Default for DailyStats {
    fn default() -> Self {
        Self {
            trades_executed: AtomicU64::new(0),
            capital_deployed: AtomicI64::new(0),
            profit_usd: AtomicI64::new(0),
            loss_usd: AtomicI64::new(0),
            start_of_day: Mutex::new(Instant::now()),
        }
    }
}

impl DailyStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn net_pnl(&self) -> f64 {
        let profit = self.profit_usd.load(Ordering::SeqCst) as f64 / 1_000_000.0;
        let loss = self.loss_usd.load(Ordering::SeqCst) as f64 / 1_000_000.0;
        profit - loss
    }

    pub fn capital_deployed(&self) -> f64 {
        self.capital_deployed.load(Ordering::SeqCst) as f64 / 1_000_000.0
    }

    pub fn record_trade(&self, profit: f64, capital: f64) {
        self.trades_executed.fetch_add(1, Ordering::SeqCst);
        self.capital_deployed.fetch_add((capital * 1_000_000.0) as i64, Ordering::SeqCst);
        if profit >= 0.0 {
            self.profit_usd.fetch_add((profit * 1_000_000.0) as i64, Ordering::SeqCst);
        } else {
            self.loss_usd.fetch_add(((-profit) * 1_000_000.0) as i64, Ordering::SeqCst);
        }
    }

    pub fn reset(&self) {
        self.trades_executed.store(0, Ordering::SeqCst);
        self.capital_deployed.store(0, Ordering::SeqCst);
        self.profit_usd.store(0, Ordering::SeqCst);
        self.loss_usd.store(0, Ordering::SeqCst);
        *self.start_of_day.lock().unwrap() = Instant::now();
    }

    pub fn is_new_day(&self) -> bool {
        let start = *self.start_of_day.lock().unwrap();
        Instant::now().duration_since(start) > Duration::from_secs(86400)
    }
}

/// Circuit breaker specifically for arbitrage execution
pub struct ArbCircuitBreaker {
    limits: ArbRiskLimits,
    daily_stats: DailyStats,
    weekly_loss: AtomicI64,  // In micro-dollars
    halted: AtomicBool,
    halt_reason: Mutex<Option<String>>,
    consecutive_failures: AtomicU64,
    max_consecutive_failures: u32,
}

impl ArbCircuitBreaker {
    pub fn new(limits: ArbRiskLimits) -> Self {
        Self {
            limits,
            daily_stats: DailyStats::new(),
            weekly_loss: AtomicI64::new(0),
            halted: AtomicBool::new(false),
            halt_reason: Mutex::new(None),
            consecutive_failures: AtomicU64::new(0),
            max_consecutive_failures: 3,
        }
    }

    /// Check if we can execute an arb trade
    pub fn can_execute(&self, edge_pct: f64, spread_bps: Option<u64>) -> Result<(), String> {
        // Check global halt
        if self.halted.load(Ordering::SeqCst) {
            return Err(format!("HALTED: {}", self.halt_reason.lock().unwrap().as_ref().unwrap_or(&"Unknown".to_string())));
        }

        // Check edge meets minimum
        if edge_pct < self.limits.min_edge_pct {
            return Err(format!("Edge {:.2}% below minimum {:.2}%", edge_pct * 100.0, self.limits.min_edge_pct * 100.0));
        }

        // Check spread
        if let Some(spread) = spread_bps {
            if spread > self.limits.max_spread_bps {
                return Err(format!("Spread {} bps exceeds maximum {} bps", spread, self.limits.max_spread_bps));
            }
        }

        // Check daily capital limit
        let deployed = self.daily_stats.capital_deployed();
        if deployed >= self.limits.max_capital_per_day {
            return Err(format!("Daily capital limit reached: ${:.2}/${:.2}", 
                deployed, self.limits.max_capital_per_day));
        }

        // Check daily loss limit
        let daily_loss = (-self.daily_stats.net_pnl()).max(0.0);
        let daily_limit = self.limits.daily_loss_limit_pct * self.limits.max_capital_per_day;
        if daily_loss > daily_limit {
            return Err(format!("Daily loss limit reached: ${:.2} > ${:.2} limit", daily_loss, daily_limit));
        }

        // Check weekly loss limit
        let weekly_loss = self.weekly_loss.load(Ordering::SeqCst) as f64 / 1_000_000.0;
        let weekly_limit = self.limits.weekly_loss_limit_pct * self.limits.max_capital_per_day * 7.0;
        if weekly_loss > weekly_limit {
            return Err(format!("Weekly loss limit reached: ${:.2} > ${:.2} limit", weekly_loss, weekly_limit));
        }

        // Check consecutive failures
        let failures = self.consecutive_failures.load(Ordering::SeqCst);
        if failures >= self.max_consecutive_failures as u64 {
            return Err(format!("Too many consecutive failures: {}", failures));
        }

        Ok(())
    }

    /// Record a successful trade
    pub fn record_success(&self, profit_usd: f64, capital_used: f64) {
        self.daily_stats.record_trade(profit_usd, capital_used);
        self.consecutive_failures.store(0, Ordering::SeqCst);
    }

    /// Record a failed trade
    pub fn record_failure(&self, loss_usd: f64, capital_used: f64) {
        self.daily_stats.record_trade(-loss_usd, capital_used);
        
        let failures = self.consecutive_failures.fetch_add(1, Ordering::SeqCst) + 1;
        if failures >= self.max_consecutive_failures as u64 {
            self.halt(format!("Consecutive failures: {}", failures));
        }

        // Check daily loss limit
        let daily_loss = (-self.daily_stats.net_pnl()).max(0.0);
        let daily_limit = self.limits.daily_loss_limit_pct * self.limits.max_capital_per_day;
        if daily_loss > daily_limit {
            self.halt(format!("Daily loss limit exceeded: ${:.2}", daily_loss));
        }
    }

    /// Record weekly loss
    pub fn record_weekly_loss(&self, loss_usd: f64) {
        let loss_micro = (loss_usd * 1_000_000.0) as i64;
        self.weekly_loss.fetch_add(loss_micro, Ordering::SeqCst);

        let weekly_loss = self.weekly_loss.load(Ordering::SeqCst) as f64 / 1_000_000.0;
        let weekly_limit = self.limits.weekly_loss_limit_pct * self.limits.max_capital_per_day * 7.0;
        
        if weekly_loss > weekly_limit {
            self.halt(format!("Weekly loss limit exceeded: ${:.2}", weekly_loss));
        }
    }

    /// Halt all trading
    pub fn halt(&self, reason: String) {
        eprintln!("🚨 ARB CIRCUIT BREAKER TRIPPED: {}", reason);
        self.halted.store(true, Ordering::SeqCst);
        *self.halt_reason.lock().unwrap() = Some(reason);
    }

    /// Resume trading (manual intervention required)
    pub fn resume(&self) -> Result<(), String> {
        if !self.halted.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Check if it's safe to resume
        let daily_loss = (-self.daily_stats.net_pnl()).max(0.0);
        let daily_limit = self.limits.daily_loss_limit_pct * self.limits.max_capital_per_day;
        if daily_loss > daily_limit {
            return Err(format!("Cannot resume: daily loss ${:.2} exceeds limit ${:.2}", daily_loss, daily_limit));
        }

        let weekly_loss = self.weekly_loss.load(Ordering::SeqCst) as f64 / 1_000_000.0;
        let weekly_limit = self.limits.weekly_loss_limit_pct * self.limits.max_capital_per_day * 7.0;
        if weekly_loss > weekly_limit {
            return Err(format!("Cannot resume: weekly loss ${:.2} exceeds limit ${:.2}", weekly_loss, weekly_limit));
        }

        self.halted.store(false, Ordering::SeqCst);
        *self.halt_reason.lock().unwrap() = None;
        self.consecutive_failures.store(0, Ordering::SeqCst);
        eprintln!("✅ ARB CIRCUIT BREAKER RESET - Trading resumed");
        Ok(())
    }

    /// Check and reset daily stats if new day
    pub fn check_day_rollover(&self) {
        if self.daily_stats.is_new_day() {
            let previous_pnl = self.daily_stats.net_pnl();
            if previous_pnl < 0.0 {
                // Add to weekly loss
                self.record_weekly_loss(-previous_pnl);
            }
            self.daily_stats.reset();
            eprintln!("📅 Daily stats reset. Previous PnL: ${:.2}", previous_pnl);
        }
    }

    /// Get current status
    pub fn status(&self) -> ArbStatus {
        ArbStatus {
            halted: self.halted.load(Ordering::SeqCst),
            halt_reason: self.halt_reason.lock().unwrap().clone(),
            daily_trades: self.daily_stats.trades_executed.load(Ordering::SeqCst),
            daily_capital: self.daily_stats.capital_deployed(),
            daily_pnl: self.daily_stats.net_pnl(),
            weekly_loss: self.weekly_loss.load(Ordering::SeqCst) as f64 / 1_000_000.0,
            consecutive_failures: self.consecutive_failures.load(Ordering::SeqCst),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArbStatus {
    pub halted: bool,
    pub halt_reason: Option<String>,
    pub daily_trades: u64,
    pub daily_capital: f64,
    pub daily_pnl: f64,
    pub weekly_loss: f64,
    pub consecutive_failures: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_execute_edge_too_low() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits::default());
        let result = breaker.can_execute(0.01, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Edge"));
    }

    #[test]
    fn test_can_execute_spread_too_wide() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits::default());
        let result = breaker.can_execute(0.03, Some(300));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Spread"));
    }

    #[test]
    fn test_can_execute_success() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits::default());
        let result = breaker.can_execute(0.03, Some(100));
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_success() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits::default());
        breaker.record_success(5.0, 50.0);
        
        let status = breaker.status();
        assert_eq!(status.daily_trades, 1);
        assert!((status.daily_pnl - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_daily_loss_limit() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits {
            max_capital_per_day: 100.0,
            daily_loss_limit_pct: 0.10, // 10% = $10
            ..Default::default()
        });

        breaker.record_failure(15.0, 50.0);
        
        let result = breaker.can_execute(0.03, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Daily loss limit"));
    }

    #[test]
    fn test_consecutive_failures() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits {
            max_capital_per_day: 1000.0,
            ..Default::default()
        });

        // Record 3 failures
        breaker.record_failure(1.0, 10.0);
        breaker.record_failure(1.0, 10.0);
        breaker.record_failure(1.0, 10.0);

        let result = breaker.can_execute(0.03, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("consecutive failures"));
    }

    #[test]
    fn test_resume_after_halt() {
        let breaker = ArbCircuitBreaker::new(ArbRiskLimits::default());
        breaker.halt("Test halt".to_string());
        assert!(breaker.halted.load(Ordering::SeqCst));

        let result = breaker.resume();
        assert!(result.is_ok());
        assert!(!breaker.halted.load(Ordering::SeqCst));
    }
}