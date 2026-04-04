//! Market Rollover Thread for HFT Bot
//! 
//! Monitors for new 5m/15m markets and subscribes to them
//! 
//! FIX (2026-04-01 17:05): Send proper JSON subscription payload for incremental WS subscription

use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use crossbeam_channel::Sender;
use crate::hft_hot_path::RolloverCommand;

/// Generate target slugs for a given asset and interval
/// FIX: Check current + past periods (markets stay active until they close)
pub fn get_target_slugs(asset: &str, interval_mins: u64) -> Vec<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let interval_secs = interval_mins * 60;
    let current_period_end = ((now / interval_secs) + 1) * interval_secs;
    
    let mut slugs = Vec::new();
    
    // Check current + past periods
    // For 5m: check last 12 periods (60 min back)
    // For 15m: check last 8 periods (120 min back)
    let check_count = if interval_mins == 5 { 12 } else { 8 };
    
    for i in 0..check_count {
        let period_end = current_period_end - (i * interval_secs);
        slugs.push(format!("{}-updown-{}m-{}", asset, interval_mins, period_end));
    }
    
    slugs
}

/// Rollover thread - monitors for new markets and sends AddPair commands
pub async fn run_rollover_thread(
    client: std::sync::Arc<Client>,
    rollover_tx: Sender<RolloverCommand>,
) {
    println!("[ROLLOVER] Thread started - monitoring for active markets with volume");
    
    let assets = vec!["btc", "eth", "sol", "xrp"];
    let intervals = vec![5, 15];
    
    loop {
        let mut found_markets = Vec::new();
        
        for &interval in &intervals {
            for asset in &assets {
                let slugs = get_target_slugs(asset, interval);
                
                for slug in slugs {
                    if let Ok(tokens) = fetch_market_tokens(&client, &slug).await {
                        if tokens.len() == 2 {
                            found_markets.push((slug, tokens[0].clone(), tokens[1].clone()));
                        }
                    }
                }
            }
        }
        
        let market_count = found_markets.len();
        
        for (slug, yes_token, no_token) in found_markets {
            let asset_type = if slug.starts_with("btc-") { "btc" }
                else if slug.starts_with("eth-") { "eth" }
                else if slug.starts_with("sol-") { "sol" }
                else { "xrp" };
            
            let interval = if slug.contains("-5m-") { "5m" } else { "15m" };
            
            println!("[ROLLOVER] Found: {} (len={})", slug, yes_token.len());
            println!("Adding: {} ({} {})", slug, asset_type, interval);
            
            let yes_hash = crate::token_map::hash_token(&yes_token);
            let no_hash = crate::token_map::hash_token(&no_token);
            
            // FIX: Build proper JSON subscription for incremental WebSocket subscription
            // This payload will be sent via ws_stream.send() in the hot path
            let ws_sub_payload = format!(
                r#"{{"assets": ["{}", "{}"], "type": "market"}}"#,
                yes_token, no_token
            );
            
            let _ = rollover_tx.send(RolloverCommand::AddPair {
                yes_hash,
                no_hash,
                ws_sub_payload,
            });
        }
        
        if market_count > 0 {
            println!("[ROLLOVER] Now tracking {} markets", market_count);
        } else {
            println!("[ROLLOVER] No active markets with volume found - waiting...");
        }
        
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }
}

/// Fetch token IDs for a market by slug
/// FIX: Handle string volume, filter by active=true AND closed=false
async fn fetch_market_tokens(
    client: &Client,
    slug: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let url = format!("https://gamma-api.polymarket.com/markets?slug={}", slug);
    
    let resp = client.get(&url).send().await?;
    let markets: Vec<serde_json::Value> = resp.json().await?;
    
    if markets.is_empty() {
        return Ok(Vec::new());
    }
    
    let market = &markets[0];
    
    let active = market["active"].as_bool().unwrap_or(false);
    let closed = market["closed"].as_bool().unwrap_or(true);
    
    // Must be active AND not closed
    if !active || closed {
        return Ok(Vec::new());
    }
    
    // FIX: Volume/liquidity are strings in API response
    let vol_str = market.get("volume").and_then(|v| v.as_str()).unwrap_or("0");
    let liq_str = market.get("liquidity").and_then(|v| v.as_str()).unwrap_or("0");
    let vol: f64 = vol_str.parse().unwrap_or(0.0);
    let liq: f64 = liq_str.parse().unwrap_or(0.0);
    
    // Skip markets with zero activity (no traders = no WebSocket messages)
    if vol <= 0.0 && liq <= 0.0 {
        println!("[ROLLOVER] Skipping {} (volume=${:.2}, liq=${:.2})", slug, vol, liq);
        return Ok(Vec::new());
    }
    
    println!("[ROLLOVER] Active market: {} (volume=${:.2}, liq=${:.2})", slug, vol, liq);
    
    let clob_token_ids_str = market["clobTokenIds"].as_str().unwrap_or("[]");
    let clob_token_ids: Vec<serde_json::Value> = serde_json::from_str(clob_token_ids_str)?;
    
    if clob_token_ids.len() != 2 {
        return Ok(Vec::new());
    }
    
    let yes_token = clob_token_ids[0].as_str().unwrap_or("").to_string();
    let no_token = clob_token_ids[1].as_str().unwrap_or("").to_string();
    
    Ok(vec![yes_token, no_token])
}
