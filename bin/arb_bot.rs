//! Arbitrage Bot Binary Entry Point
//!
//! Connects to Polymarket WebSocket, monitors orderbooks for arbitrage opportunities,
//! and executes risk-free trades when YES_ask + NO_ask < $1.00

use poly_market_maker::arb_monitor::{ArbMonitor, MarketPair};
use poly_market_maker::dual_executor::{DualExecutor, ExecutorConfig};
use poly_market_maker::arb_risk::{ArbCircuitBreaker, ArbRiskLimits};
use poly_market_maker::hot_path::EngineEvent;
use poly_market_maker::network::ws_engine::run_ws_engine;

use crossbeam_channel::bounded;

/// Default tokens for testing (will be replaced by market discovery)
const DEFAULT_YES_TOKEN: &str = "8501497159083948713316135768103773293754490207922884688769443031624417212426";
const DEFAULT_NO_TOKEN: &str = "2527312495175492857904889758552137141356236738032676480522356889996545113869";
const DEFAULT_CONDITION_ID: &str = "0x9c1a953fe92c8357f1b646ba25d983aa83e90c525992db14fb726fa895cb5763";

/// WebSocket URL for Polymarket CLOB
const WS_URL: &str = "wss://ws-subscriptions-polymarket.herokuapp.com";

#[derive(Debug, Clone)]
pub struct BotConfig {
    /// Dry-run mode (no actual orders)
    pub dry_run: bool,
    /// Maximum position size (USD)
    pub max_position: f64,
    /// Minimum edge percentage
    pub min_edge_pct: f64,
    /// Maximum spread (bps)
    pub max_spread_bps: u64,
    /// Private key (optional, for live trading)
    pub private_key: Option<String>,
    /// Funder address (optional, for live trading)
    pub funder_address: Option<String>,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            dry_run: true, // Safety first!
            max_position: 100.0,
            min_edge_pct: 0.02,
            max_spread_bps: 200,
            private_key: None,
            funder_address: None,
        }
    }
}

#[tokio::main]
async fn main() {
    // Parse command line args
    let args: Vec<String> = std::env::args().collect();
    let dry_run = !args.contains(&"--live".to_string());
    
    if !dry_run {
        eprintln!("⚠️ LIVE TRADING MODE ENABLED ⚠️");
        eprintln!("Make sure you have:");
        eprintln!("  - Set PRIVATE_KEY environment variable");
        eprintln!("  - Set FUNDER_ADDRESS environment variable");
        eprintln!("  - Sufficient USDC balance");
    }

    eprintln!("🏓 Arbitrage Bot Starting (Dry Run: {})", dry_run);

    // Create channels
    let (event_tx, event_rx) = bounded(1000);
    let (opp_tx, _opp_rx) = bounded(100);

    // Initialize components
    let mut monitor = ArbMonitor::new(opp_tx);
    let mut executor = DualExecutor::new(ExecutorConfig {
        dry_run,
        max_position_usd: 100.0,
        ..Default::default()
    });
    let risk = ArbCircuitBreaker::new(ArbRiskLimits {
        min_edge_pct: 0.02,
        max_spread_bps: 200,
        ..Default::default()
    });

    // Register default market
    monitor.register_market(MarketPair {
        condition_id: DEFAULT_CONDITION_ID.to_string(),
        yes_token: DEFAULT_YES_TOKEN.to_string(),
        no_token: DEFAULT_NO_TOKEN.to_string(),
        market_name: "Default Test Market".to_string(),
        active: true,
    });

    eprintln!("📊 Monitoring tokens:");
    eprintln!("   YES: {}...", &DEFAULT_YES_TOKEN[..20]);
    eprintln!("   NO:  {}...", &DEFAULT_NO_TOKEN[..20]);

    // Spawn WebSocket task
    let assets = vec![DEFAULT_YES_TOKEN.to_string(), DEFAULT_NO_TOKEN.to_string()];
    let ws_config = poly_market_maker::network::ws_engine::WsConfig {
        url: WS_URL.to_string(),
        assets,
        api_key: None,
        signature: None,
        timestamp: None,
        passphrase: None,
    };

    let _ws_handle = tokio::spawn(async move {
        let _ = run_ws_engine(ws_config, event_tx).await;
    });

    eprintln!("🚀 Waiting for arbitrage opportunities...");
    eprintln!("   Edge threshold: 2%");
    eprintln!("   Max spread: 200 bps");
    eprintln!("   Max position: $100");

    // Main event loop
    let mut update_count = 0u64;
    let mut last_stats = std::time::Instant::now();

    loop {
        // Check for day rollover
        risk.check_day_rollover();

        // Try to receive WebSocket event
        match event_rx.try_recv() {
            Ok(event) => {
                update_count += 1;
                
                match &event {
                    EngineEvent::OrderbookUpdate { token_id, raw_bytes, .. } => {
                        // Process orderbook update
                        let raw_json = String::from_utf8_lossy(raw_bytes);
                        if let Some(opp) = monitor.process_orderbook_update(token_id, &raw_json) {
                            // Opportunity detected!
                            if dry_run {
                                eprintln!("🎯 OPPORTUNITY DETECTED (DRY RUN):");
                                eprintln!("   YES: {:.2}c  NO: {:.2}c  Combined: {:.2}c",
                                    opp.yes_ask_cents, opp.no_ask_cents, opp.combined_cost_cents);
                                eprintln!("   Edge: {:.2}%  Market: {}",
                                    opp.edge_pct * 100.0, opp.market_name.as_deref().unwrap_or("unknown"));
                            }

                            // Check risk limits
                            let spread_bps = {
                                let yes_bps = monitor.orderbooks.get(&opp.yes_token)
                                    .and_then(|s| s.snapshot.yes_spread_bps());
                                let no_bps = monitor.orderbooks.get(&opp.no_token)
                                    .and_then(|s| s.snapshot.no_spread_bps());
                                yes_bps.max(no_bps)
                            };

                            match risk.can_execute(opp.edge_pct, spread_bps) {
                                Ok(()) => {
                                    // Execute (or simulate in dry-run)
                                    let rt = tokio::runtime::Runtime::new().unwrap();
                                    let result = rt.block_on(executor.execute(opp));
                                    
                                    if result.success {
                                        eprintln!("✅ EXECUTION SUCCESS:");
                                        eprintln!("   Profit: ${:.2}", 
                                            result.expected_payout_usd - result.total_cost_usd);
                                        
                                        // Record success
                                        risk.record_success(
                                            result.expected_payout_usd - result.total_cost_usd,
                                            result.total_cost_usd,
                                        );
                                    } else if let Some(err) = result.error {
                                        eprintln!("❌ EXECUTION FAILED: {}", err);
                                        risk.record_failure(0.0, 0.0);
                                    }
                                }
                                Err(reason) => {
                                    eprintln!("⚠️ Risk check failed: {}", reason);
                                }
                            }
                        }
                    }
                    EngineEvent::MarketRollover { asset_symbol, condition_id, .. } => {
                        eprintln!("🔄 Market rollover: {} -> {}", asset_symbol, &condition_id[..16]);
                        monitor.handle_market_rollover(&event);
                    }
                    _ => {}
                }
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {
                // No message available, continue
            }
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                eprintln!("🔴 WebSocket disconnected");
                break;
            }
        }

        // Periodic stats
        if last_stats.elapsed().as_secs() >= 30 {
            let (updates, opps, stale) = monitor.stats();
            let status = risk.status();
            eprintln!("📊 Stats: {} updates, {} opportunities, {} stale", updates, opps, stale);
            eprintln!("📊 Risk: {} trades, ${:.2} deployed, ${:.2} PnL",
                status.daily_trades, status.daily_capital, status.daily_pnl);
            last_stats = std::time::Instant::now();
        }

        // Small sleep to prevent busy loop
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    eprintln!("✅ Bot stopped");
}