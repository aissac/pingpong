// src/telemetry.rs
// Zero-allocation structs that cross the hot path → background thread boundary
// Everything implements Copy - no heap allocations when passing through crossbeam_channel

/// Represents tasks sent from pinned hot thread to background async thread
#[derive(Clone, Copy)]
pub enum BackgroundTask {
    /// Sent when combined YES + NO ask price drops below edge threshold
    OpportunityDetected(OpportunitySnapshot),
    
    /// Sent periodically to batch-log processing latencies
    LatencyStats(LatencyBatch),
    
    /// Sent when a real trade executes (for PnL reporting)
    TradeExecuted(TradeExecution),
}

/// Cache-line aligned snapshot of an arbitrage opportunity
/// Fixed-point prices (e.g., 670_000 = $0.67) to avoid f64 in hot path
#[derive(Clone, Copy)]
#[repr(align(64))]
pub struct OpportunitySnapshot {
    pub condition_hash: u64,      // Hash of market condition_id
    pub yes_token_hash: u64,
    pub no_token_hash: u64,
    pub yes_ask_price: u64,        // Fixed-point 6 decimals
    pub no_ask_price: u64,         // Fixed-point 6 decimals
    pub yes_depth: u64,            // Available liquidity
    pub no_depth: u64,
    pub edge_bps: u32,             // Edge in basis points (e.g., 500 = 5%)
    pub timestamp_nanos: u64,      // TSC timestamp
}

/// Batched latency measurements from hot path
#[derive(Clone, Copy)]
pub struct LatencyBatch {
    pub min_nanos: u64,
    pub max_nanos: u64,
    pub avg_nanos: u64,
    pub p99_nanos: u64,
    pub sample_count: u32,
    pub timestamp_nanos: u64,
}

/// Trade execution for PnL tracking
#[derive(Clone, Copy)]
pub struct TradeExecution {
    pub condition_hash: u64,
    pub token_hash: u64,
    pub fill_price: u64,           // Fixed-point 6 decimals
    pub shares: u64,
    pub is_stop_loss: bool,
    pub pnl_cents: i64,            // Profit in cents (can be negative)
    pub timestamp_nanos: u64,
}

impl OpportunitySnapshot {
    pub fn combined_price(&self) -> f64 {
        (self.yes_ask_price + self.no_ask_price) as f64 / 1_000_000.0
    }
    
    pub fn edge_percent(&self) -> f64 {
        self.edge_bps as f64 / 100.0
    }
}