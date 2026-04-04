//! Polymarket Copy-Trader - Live Order Execution

use crossbeam_channel::unbounded;
use std::env;
use std::fs;

mod copy_trader;
mod signer;
mod models;
mod hot_path;
mod market_state;
mod fee_oracle;
mod order_executor;

use crate::copy_trader::CopyTraderEngine;
use crate::order_executor::{OrderExecutor, ExecutorConfig};
use crate::signer::ClobSigner;

#[tokio::main]
async fn main() {
    println!("🚀 Starting Polymarket Copy-Trader...");
    println!("");

    // Load .env file
    if let Ok(content) = fs::read_to_string(".env") {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                if key.starts_with('#') || key.trim().is_empty() {
                    continue;
                }
                unsafe {
                    env::set_var(key.trim(), value.trim());
                }
            }
        }
    }

    // Read configuration
    let dry_run = env::var("DRY_RUN")
        .unwrap_or_else(|_| "true".to_string())
        .parse()
        .unwrap_or(true);

    let max_daily_loss = env::var("MAX_DAILY_LOSS_USD")
        .unwrap_or_else(|_| "100.0".to_string())
        .parse()
        .unwrap_or(100.0);

    let order_timeout = env::var("ORDER_TIMEOUT_SECS")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or(60);

    let target_wallet = env::var("TARGET_WALLET")
        .unwrap_or_else(|_| "0xe1d6b51521bd4365769199f392f9818661bd907".to_string());

    let copy_ratio = env::var("COPY_RATIO")
        .unwrap_or_else(|_| "0.10".to_string())
        .parse()
        .unwrap_or(0.10);

    let max_trade_cap = env::var("MAX_TRADE_CAP_USD")
        .unwrap_or_else(|_| "500.0".to_string())
        .parse()
        .unwrap_or(500.0);

    let private_key = env::var("POLYMARKET_PRIVATE_KEY")
        .unwrap_or_else(|_| "".to_string());

    let funder_address = env::var("POLYMARKET_FUNDER_ADDRESS")
        .unwrap_or_else(|_| "".to_string());

    // Determine mode
    let is_live = !private_key.is_empty() && !funder_address.is_empty() && !dry_run;

    println!("📋 Configuration:");
    println!("   Mode: {}", if is_live { "🔴 LIVE TRADING" } else { "🟢 PAPER TRADING (DRY-RUN)" });
    println!("   Target Whale: {}", &target_wallet[..10]);
    println!("   Copy Ratio: {:.1}%", copy_ratio * 100.0);
    println!("   Max Trade Cap: ${:.2}", max_trade_cap);
    println!("   Max Daily Loss: ${:.2}", max_daily_loss);
    println!("   Order Timeout: {}s", order_timeout);
    println!("");

    // Safety check
    if is_live {
        println!("⚠️  LIVE TRADING ENABLED - Orders will be submitted to Polymarket!");
        println!("   Private Key: {}...{}", &private_key[..6], &private_key[private_key.len()-4..]);
        println!("   Funder: {}", &funder_address[..10]);
        println!("");
    } else if private_key.is_empty() || funder_address.is_empty() {
        println!("ℹ️  No wallet configured - running in paper mode");
        println!("   Set POLYMARKET_PRIVATE_KEY and POLYMARKET_FUNDER_ADDRESS to enable live trading");
        println!("");
    }

    // 1. Create crossbeam channel for hot-path
    let (hot_path_tx, hot_path_rx) = unbounded();

    // 2. Initialize Copy Trader Engine
    let mut copy_trader = CopyTraderEngine::new(
        &target_wallet,
        hot_path_tx,
        copy_ratio,
        max_trade_cap
    );

    // 3. Pre-warm cache
    if let Err(e) = copy_trader.pre_warm_cache().await {
        eprintln!("⚠️ Cache pre-warm failed: {:?}", e);
    }

    // 4. Spawn polling loop
    tokio::spawn(async move {
        copy_trader.run_polling_loop().await;
    });

    println!("✅ Copy-trader running. Waiting for whale trades...");
    println!("");

    // 5. Initialize Order Executor
    let executor_config = ExecutorConfig {
        dry_run: !is_live,
        max_daily_loss_usd: max_daily_loss,
        order_timeout_secs: order_timeout,
        api_base: "https://clob.polymarket.com".to_string(),
    };

    if is_live && !private_key.is_empty() && !funder_address.is_empty() {
        // LIVE MODE: Create signer and executor
        let signer = ClobSigner::new(&private_key, &funder_address);
        let executor = OrderExecutor::new(signer, executor_config);

        println!("🎯 Starting live order executor...");
        executor.run(hot_path_rx).await;
    } else {
        // PAPER MODE: Run paper trading loop
        println!("📊 Starting paper trading loop...");
        run_paper_trading_loop(hot_path_rx).await;
    }
}

/// Paper trading validation loop (fallback)
async fn run_paper_trading_loop(rx: crossbeam_channel::Receiver<crate::copy_trader::CopyTradeAction>) {
    use serde_json::json;
    use chrono::Utc;

    let mut total_trades = 0u32;
    let mut cumulative_pnl = 0.0f64;
    let taker_fee_pct = 0.018; // 1.80% crypto fee

    loop {
        if let Ok(action) = rx.try_recv() {
            total_trades += 1;

            // Simulated metrics
            let slippage = 0.02; // 2 cents slippage
            let entry_price = action.target_price + slippage;
            let fee_cost = action.calculated_size_usd * taker_fee_pct;

            // Simulate 5-cent favorable move
            let exit_price = entry_price + 0.05;
            let gross_profit = (exit_price - entry_price) * (action.calculated_size_usd / entry_price);
            let net_pnl = gross_profit - (fee_cost * 2.0);
            cumulative_pnl += net_pnl;

            // JSON log output
            let log = json!({
                "time": Utc::now().timestamp_millis(),
                "type": "paper_trade",
                "mode": "legacy_paper",
                "asset": action.asset_id,
                "side": action.side,
                "whale_price": action.target_price,
                "our_entry": entry_price,
                "slippage_cents": slippage * 100.0,
                "size_usd": action.calculated_size_usd,
                "fees_usd": fee_cost * 2.0,
                "net_pnl_usd": net_pnl,
                "cumulative_pnl_usd": cumulative_pnl,
                "total_trades": total_trades
            });

            println!("{}", log.to_string());
        } else {
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    }
}