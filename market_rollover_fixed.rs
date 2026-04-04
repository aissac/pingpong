//! Market Rollover Thread for HFT Bot
//! 
//! Monitors for new 5m/15m markets and subscribes to them
//! 
//! FIX (2026-04-01): Use correct timestamp calculation (END of period, not START)
//! Filter out markets with <2 minutes remaining
//! Pre-subscribe to next period 1 minute before start

use reqwest::Client;
use std::time::{SystemTime, UNIX_EPOCH};
use crossbeam_channel::Sender;
use crate::hft_hot_path::RolloverCommand;

/// Generate target slugs for a given asset and interval
/// 
/// FIX: Calculate END of period (not START)
/// Filter: Only return markets with >2 minutes remaining
/// Pre-subscribe: Return next period when <1 minute left
pub fn get_target_slugs(asset: &str, interval_mins: u64) -> Vec<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let interval_secs = interval_mins * 60;
    
    // FIX: Add +1 to get END of current period (NotebookLM fix)
    let current_period_end = ((now / interval_secs) + 1) * interval_secs;
    let time_remaining = current_period_end.saturating_sub(now);
    
    let mut slugs = Vec::new();
    
    // FILTER: Only trade current market if >2 minutes (120 seconds) remaining
    if time_remaining > 120 {
        slugs.push(format!("{}-updown-{}m-{}", asset, interval_mins, current_period_end));
    }
    
    // PRE-SUBSCRIBE: Track next market 1 minute (60 seconds) before it starts
    // (Triggered when current period has <=60 seconds remaining)
    if time_remaining <= 60 {
        let next_period_end = current_period_end + interval_secs;
        slugs.push(format!("{}-updown-{}m-{}", asset, interval_mins, next_period_end));
    }
    
    slugs
}

/// Rollover thread - monitors for new markets and sends AddPair commands
pub async fn run_rollover_thread(
    client: std::sync::Arc<Client>,
    rollover_tx: Sender<RolloverCommand>,
) {
    println!("[ROLLOVER] Thread started - monitoring for active markets");
    
    let assets = vec!["btc", "eth", "sol", "xrp"];
    let intervals = vec![5, 15];
    
    loop {
        let mut found_markets = Vec::new();
        
        // Check all asset/interval combinations
        for &interval in &intervals {
            for asset in &assets {
                let slugs = get_target_slugs(asset, interval);
                
                for slug in slugs {
                    // Fetch market to get token IDs
                    if let Ok(tokens) = fetch_market_tokens(&client, &slug).await {
                        if tokens.len() == 2 {
                            found_markets.push((slug, tokens[0].clone(), tokens[1].clone()));
                        }
                    }
                }
            }
        }
        
        // Send AddPair commands for new markets
        for (slug, yes_token, no_token) in found_markets {
            let asset_type = if slug.starts_with("btc-") { "btc" }
                else if slug.starts_with("eth-") { "eth" }
                else if slug.starts_with("sol-") { "sol" }
                else { "xrp" };
            
            let interval = if slug.contains("-5m-") { "5m" } else { "15m" };
            
            println!("[ROLLOVER] Found: {} (clobTokenIds[0] len={})", slug, yes_token.len());
            println!("🟢 [ROLLOVER] Adding: {} ({} {})", slug, asset_type, interval);
            
            // Send AddPair command to hot path
            let _ = rollover_tx.send(RolloverCommand::AddPair {
                yes_token,
                no_token,
                asset_type: asset_type.to_string(),
                interval: interval.to_string(),
                ws_sub_payload: format!("{}-updown", asset_type),
            });
        }
        
        if !found_markets.is_empty() {
            println!("[ROLLOVER] Now tracking {} markets", found_markets.len());
        }
        
        // Check every 15 seconds
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }
}

/// Fetch token IDs for a market by slug
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
    
    // Check if active
    let active = market["active"].as_bool().unwrap_or(false);
    let closed = market["closed"].as_bool().unwrap_or(true);
    
    if !active || closed {
        return Ok(Vec::new());
    }
    
    // Get token IDs from clobTokenIds field
    let clob_token_ids_str = market["clobTokenIds"].as_str().unwrap_or("[]");
    let clob_token_ids: Vec<serde_json::Value> = serde_json::from_str(clob_token_ids_str)?;
    
    if clob_token_ids.len() != 2 {
        return Ok(Vec::new());
    }
    
    let yes_token = clob_token_ids[0].as_str().unwrap_or("").to_string();
    let no_token = clob_token_ids[1].as_str().unwrap_or("").to_string();
    
    Ok(vec![yes_token, no_token])
}
