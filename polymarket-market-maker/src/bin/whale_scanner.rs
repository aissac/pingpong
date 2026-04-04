//! Whale Scanner - Safe, isolated analysis of Polymarket traders

use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

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
}

#[tokio::main]
async fn main() {
    let target_wallet = "0xe1d6b51521bd4365769199f392f9818661bd907";
    
    println!("🐋 Whale Scanner v0.1");
    println!("Target: {}", target_wallet);
    println!("");
    
    let client = Client::new();
    let url = format!("https://data-api.polymarket.com/trades?user={}", target_wallet);
    
    println!("🔍 Fetching trade history...");
    
    match client.get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await 
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                println!("⚠️ API returned: {}", resp.status());
                return;
            }
            
            match resp.json::<TradeResponse>().await {
                Ok(payload) => {
                    println!("✅ Found {} trades", payload.data.len());
                    analyze_trades(&payload.data);
                }
                Err(e) => println!("❌ JSON error: {}", e),
            }
        }
        Err(e) => println!("❌ Request error: {}", e),
    }
}

fn analyze_trades(trades: &[WhaleTrade]) {
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
            
            let time_str = chrono::DateTime::from_timestamp(trade.timestamp as i64, 0)
                .map(|dt| dt.format("%m/%d %H:%M").to_string())
                .unwrap_or_default();
            
            let asset_display = if trade.asset_id.len() >= 12 { 
                &trade.asset_id[..12] 
            } else { 
                &trade.asset_id 
            };
            
            println!("[BUY]  {} | {:12} | ${:.4} | {:.0} shares",
                time_str, asset_display, trade.price, trade.size);
        } else if trade.side == "SELL" {
            let revenue = trade.price * trade.size;
            let avg_cost = if pos.total_shares > 0.0 { 
                pos.total_invested / pos.total_shares 
            } else { 0.0 };
            
            let trade_pnl = revenue - (avg_cost * trade.size);
            pos.realized_pnl += trade_pnl;
            pos.total_shares -= trade.size;
            pos.total_invested -= avg_cost * trade.size;

            if trade_pnl > 0.0 { total_wins += 1; }

            let time_str = chrono::DateTime::from_timestamp(trade.timestamp as i64, 0)
                .map(|dt| dt.format("%m/%d %H:%M").to_string())
                .unwrap_or_default();

            let icon = if trade_pnl >= 0.0 { "✅" } else { "❌" };
            let asset_display = if trade.asset_id.len() >= 12 { 
                &trade.asset_id[..12] 
            } else { 
                &trade.asset_id 
            };
            
            println!("[SELL] {} | {:12} | ${:.4} | {:.0} shares | {} ${:.2}",
                time_str, asset_display, trade.price, trade.size, icon, trade_pnl);
        }
    }

    let mut total_realized = 0.0f64;
    let mut open_value = 0.0f64;
    
    println!("\n📊 === WHALE VALIDATION REPORT ===");
    
    for (asset, m) in portfolio.iter() {
        total_realized += m.realized_pnl;
        if m.total_shares > 0.0 {
            open_value += m.total_shares - m.total_invested;
            let asset_display = if asset.len() >= 12 { 
                &asset[..12] 
            } else { 
                asset 
            };
            println!("⚠️ [OPEN] {:12} | {:.0} shares | Cost: ${:.2}",
                asset_display, m.total_shares, m.total_invested);
        }
    }

    let win_rate = if total_trades > 0 { (total_wins as f64 / total_trades as f64) * 100.0 } else { 0.0 };
    let total_pnl = total_realized + open_value;

    println!("=================================");
    println!("Total Trades:   {}", total_trades);
    println!("Total Volume:   ${:.2}", total_volume);
    println!("Win Rate:       {:.1}%", win_rate);
    println!("Realized PnL:   ${:.2}", total_realized);
    println!("Open Positions: ${:.2}", open_value);
    println!("TOTAL PnL:      ${:.2}", total_pnl);
    println!("=================================");
    
    if total_pnl > 1000.0 && win_rate > 50.0 {
        println!("✅ PROFITABLE WHALE - Safe to copy");
    } else if total_pnl > 0.0 {
        println!("⚠️ Marginally profitable");
    } else {
        println!("❌ UNPROFITABLE - Do NOT copy");
    }
}
