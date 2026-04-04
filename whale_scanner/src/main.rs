//! Whale Scanner - Safe, isolated analysis of Polymarket traders
//! Uses Polymarket Data API (no auth required)

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct TradeResponse {
    pub data: Vec<WhaleTrade>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WhaleTrade {
    pub transaction_hash: String,
    pub condition_id: String,
    pub asset_id: String,
    pub side: String,
    pub price: f64,
    pub size: f64,
    #[serde(rename = "takerAmount")]
    pub taker_amount: Option<f64>,
    pub timestamp: u64,
}

#[derive(Debug, Default)]
pub struct PositionMetrics {
    pub total_invested: f64,
    pub total_shares: f64,
    pub realized_pnl: f64,
    pub buy_count: u32,
    pub sell_count: u32,
}

pub struct WhaleScanner {
    client: Client,
    wallet_address: String,
}

impl WhaleScanner {
    pub fn new(wallet_address: &str) -> Self {
        Self {
            client: Client::new(),
            wallet_address: wallet_address.to_string(),
        }
    }

    pub async fn fetch_trade_history(&self) -> Result<Vec<WhaleTrade>, Box<dyn Error>> {
        println!("🔍 Fetching trade history for {}...", self.wallet_address);
        
        let url = format!(
            "https://data-api.polymarket.com/trades?user={}",
            self.wallet_address
        );

        let response = self.client.get(&url)
            .header("User-Agent", "Mozilla/5.0")
            .send()
            .await?;
        
        if !response.status().is_success() {
            println!("⚠️ API returned status: {}", response.status());
            return Ok(vec![]);
        }

        let payload: TradeResponse = response.json().await?;
        println!("✅ Found {} trades", payload.data.len());
        Ok(payload.data)
    }

    pub fn run_simulation(&self, trades: &[WhaleTrade]) {
        let mut portfolio: HashMap<String, PositionMetrics> = HashMap::new();
        let mut total_wins = 0u32;
        let mut total_trades = 0u32;
        let mut total_volume = 0.0f64;

        let mut chronological = trades.to_vec();
        chronological.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        for trade in chronological {
            total_trades += 1;
            total_volume += trade.size;
            let pos = portfolio.entry(trade.asset_id.clone()).or_default();

            if trade.side == "BUY" {
                let cost = trade.price * trade.size;
                pos.total_invested += cost;
                pos.total_shares += trade.size;
                pos.buy_count += 1;
                
                let time_str = chrono::DateTime::from_timestamp(trade.timestamp as i64, 0)
                    .map(|dt| dt.format("%m/%d %H:%M").to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                
                println!("[BUY]  {} | {:8} | ${:.4} | {:.0} shares | ${:.2}",
                    time_str, &trade.asset_id[..12], trade.price, trade.size, cost);
            } else if trade.side == "SELL" {
                let revenue = trade.price * trade.size;
                let avg_cost = if pos.total_shares > 0.0 { 
                    pos.total_invested / pos.total_shares 
                } else { 0.0 };
                
                let trade_pnl = revenue - (avg_cost * trade.size);
                pos.realized_pnl += trade_pnl;
                pos.total_shares -= trade.size;
                pos.total_invested -= avg_cost * trade.size;
                pos.sell_count += 1;

                if trade_pnl > 0.0 { total_wins += 1; }

                let time_str = chrono::DateTime::from_timestamp(trade.timestamp as i64, 0)
                    .map(|dt| dt.format("%m/%d %H:%M").to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                let pnl_icon = if trade_pnl >= 0.0 { "✅" } else { "❌" };
                println!("[SELL] {} | {:8} | ${:.4} | {:.0} shares | {} ${:.2}",
                    time_str, &trade.asset_id[..12], trade.price, trade.size, pnl_icon, trade_pnl);
            }
        }

        let mut total_realized_pnl = 0.0f64;
        let mut open_positions_value = 0.0f64;
        
        println!("\n📊 === WHALE VALIDATION REPORT ===");
        println!("===================================");
        
        for (asset, metrics) in portfolio.iter() {
            total_realized_pnl += metrics.realized_pnl;
            if metrics.total_shares > 0.0 {
                let open_value = metrics.total_shares * 1.0; // Assume $1 resolution
                open_positions_value += open_value - metrics.total_invested;
                println!("⚠️  [OPEN] {:12} | {:.0} shares | Cost: ${:.2} | Unrealized: ${:.2}",
                    &asset[..12], metrics.total_shares, metrics.total_invested, open_value - metrics.total_invested);
            }
        }

        let win_rate = if total_trades > 0 { (total_wins as f64 / total_trades as f64) * 100.0 } else { 0.0 };
        let total_pnl = total_realized_pnl + open_positions_value;

        println!("===================================");
        println!("Total Trades:      {}", total_trades);
        println!("Total Volume:      ${:.2}", total_volume);
        println!("Win Rate:          {:.1}%", win_rate);
        println!("Realized PnL:      ${:.2}", total_realized_pnl);
        println!("Open Positions:    ${:.2}", open_positions_value);
        println!("TOTAL PnL:         ${:.2}", total_pnl);
        println!("===================================");
        
        if total_pnl > 1000.0 && win_rate > 50.0 {
            println!("✅ EVALUATION: PROFITABLE WHALE - Safe to copy-trade");
        } else if total_pnl > 0.0 {
            println!("⚠️  EVALUATION: Marginally profitable - Monitor closely");
        } else {
            println!("❌ EVALUATION: UNPROFITABLE - Do NOT copy");
        }
    }
}

#[tokio::main]
async fn main() {
    let target_wallet = "0xe1d6b51521bd4365769199f392f9818661bd907";
    
    println!("🐋 Whale Scanner v0.1");
    println!("Target: {}", target_wallet);
    println!("");
    
    let scanner = WhaleScanner::new(target_wallet);

    match scanner.fetch_trade_history().await {
        Ok(trades) => {
            if trades.is_empty() {
                println!("❌ No trades found for this wallet");
            } else {
                scanner.run_simulation(&trades);
            }
        }
        Err(e) => eprintln!("❌ Failed to scan wallet: {}", e),
    }
}
