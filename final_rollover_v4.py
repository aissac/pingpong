#!/usr/bin/env python3
"""Fix rollover to check FUTURE periods, not current ones"""

code = r'''//! Market Rollover - Dynamic Token Pair Management
//!
//! Fetches markets for FUTURE periods (next 5m/15m cycles).
//! Uses same slug construction as startup but for upcoming periods.
//! Sends AddPair/RemovePair commands to hot path via crossbeam_channel.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use chrono::{Utc, DateTime, Timelike};
use serde_json::Value;
use crossbeam_channel::Sender;
use crate::hft_hot_path::RolloverCommand;

/// Hash token ID string to u64 (MUST match hot path fast_hash)
pub fn hash_token(token_id: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    token_id.as_bytes().hash(&mut hasher);
    hasher.finish()
}

/// Get FUTURE period timestamps (for markets that haven't started yet)
fn get_future_periods() -> Vec<(i64, &'static str)> {
    let now = Utc::now();
    let mut periods = Vec::new();
    
    // Current 5m period
    let minute_5 = (now.minute() / 5) * 5;
    let period_5m = now.with_minute(minute_5).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let ts_5m = period_5m.timestamp();
    
    // Add NEXT 5m periods (current + 1, current + 2)
    periods.push((ts_5m + 300, "5m"));   // Next 5m
    periods.push((ts_5m + 600, "5m"));   // +2 5m
    
    // Current 15m period
    let minute_15 = (now.minute() / 15) * 15;
    let period_15m = now.with_minute(minute_15).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let ts_15m = period_15m.timestamp();
    
    // Add NEXT 15m periods
    periods.push((ts_15m + 900, "15m"));  // Next 15m
    periods.push((ts_15m + 1800, "15m")); // +2 15m
    
    // Current 1h period
    let period_1h = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let ts_1h = period_1h.timestamp();
    
    // Add NEXT 1h period
    periods.push((ts_1h + 3600, "1h"));
    
    periods
}

/// Fetch tokens for a specific market slug
async fn fetch_market_tokens(client: &reqwest::Client, slug: &str) -> Option<(String, String)> {
    let url = format!("https://gamma-api.polymarket.com/markets?slug={}", slug);
    
    match client.get(&url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                return None;
            }
            
            match response.json::<Value>().await {
                Ok(json) => {
                    if let Some(markets) = json.as_array() {
                        if let Some(market) = markets.first() {
                            // Check if market is active and not closed
                            let active = market.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
                            let closed = market.get("closed").and_then(|v| v.as_bool()).unwrap_or(true);
                            
                            if !active || closed {
                                return None;
                            }
                            
                            if let Some(tokens) = market.get("tokens").and_then(|v| v.as_array()) {
                                if tokens.len() == 2 {
                                    let yes_token = tokens[0].get("token_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let no_token = tokens[1].get("token_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    
                                    if !yes_token.is_empty() && !no_token.is_empty() {
                                        return Some((yes_token, no_token));
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

/// Run the rollover monitoring thread
pub async fn run_rollover_thread(
    client: Arc<reqwest::Client>,
    rollover_tx: Sender<RolloverCommand>,
) {
    println!("🔄 [ROLLOVER] Thread started - monitoring for future markets");
    
    let mut tracked_markets: HashSet<String> = HashSet::new();
    
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        
        let periods = get_future_periods();
        let assets = ["btc", "eth", "sol", "xrp"];
        
        let mut active_this_cycle: HashSet<String> = HashSet::new();
        
        for asset in &assets {
            for (period_ts, timeframe) in &periods {
                // Construct slug (same as crypto_markets)
                let slug = match *timeframe {
                    "5m" => format!("{}-updown-5m-{}", asset, period_ts - 600),
                    "15m" => format!("{}-updown-15m-{}", asset, period_ts),
                    "1h" => format!("{}-updown-1h-{}", asset, period_ts),
                    _ => continue,
                };
                
                match fetch_market_tokens(&client, &slug).await {
                    Some((yes_token, no_token)) => {
                        let market_id = yes_token.clone();
                        active_this_cycle.insert(market_id.clone());
                        
                        if !tracked_markets.contains(&market_id) {
                            println!("🟢 [ROLLOVER] Adding market: {} ({} {})", 
                                slug, asset, timeframe);
                            
                            let yes_hash = hash_token(&yes_token);
                            let no_hash = hash_token(&no_token);
                            
                            if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(yes_hash, no_hash)) {
                                eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                                return;
                            }
                            
                            tracked_markets.insert(market_id.clone());
                            println!("[ROLLOVER] Now tracking {} markets", tracked_markets.len());
                        }
                    }
                    None => {
                        // Market doesn't exist yet or is closed
                    }
                }
            }
        }
        
        // Remove expired markets
        let expired: Vec<String> = tracked_markets
            .iter()
            .filter(|id| !active_this_cycle.contains(*id))
            .cloned()
            .collect();
        
        for expired_id in expired {
            println!("🔴 [ROLLOVER] Market expired, removing: {}", expired_id);
            
            let expired_hash = hash_token(&expired_id);
            
            let _ = rollover_tx.send(RolloverCommand::RemovePair(expired_hash));
            tracked_markets.remove(&expired_id);
        }
    }
}
'''

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'w') as f:
    f.write(code)

print('✅ Fixed rollover to check FUTURE periods')
