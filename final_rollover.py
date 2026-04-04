#!/usr/bin/env python3
"""Rewrite market_rollover.rs with proper debug"""

code = r'''//! Market Rollover - Dynamic Token Pair Management
//!
//! Fetches new markets from Gamma API using same logic as startup.
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

/// Get current period timestamps (same as crypto_markets)
fn get_current_periods() -> Vec<(i64, &'static str)> {
    let now = Utc::now();
    let mut periods = Vec::new();
    
    let minute_5 = (now.minute() / 5) * 5;
    let period_5m = now.with_minute(minute_5).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let ts_5m = period_5m.timestamp();
    periods.push((ts_5m, "5m"));
    periods.push((ts_5m + 300, "5m"));
    
    let minute_15 = (now.minute() / 15) * 15;
    let period_15m = now.with_minute(minute_15).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let ts_15m = period_15m.timestamp();
    periods.push((ts_15m, "15m"));
    periods.push((ts_15m + 900, "15m"));
    
    let period_1h = now.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap();
    let ts_1h = period_1h.timestamp();
    periods.push((ts_1h, "1h"));
    periods.push((ts_1h + 3600, "1h"));
    
    periods
}

/// Fetch tokens for a specific market slug
async fn fetch_market_tokens(client: &reqwest::Client, slug: &str) -> Option<(String, String)> {
    let url = format!("https://gamma-api.polymarket.com/markets?slug={}", slug);
    println!("[ROLLOVER] Fetching: {}", slug);
    
    match client.get(&url).send().await {
        Ok(response) => {
            let status = response.status();
            println!("[ROLLOVER] {} status: {}", slug, status);
            if !response.status().is_success() {
                return None;
            }
            
            match response.json::<Value>().await {
                Ok(json) => {
                    if let Some(markets) = json.as_array() {
                        if let Some(market) = markets.first() {
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
                                        println!("[ROLLOVER] {} tokens found (YES len={} NO len={})", 
                                            slug, yes_token.len(), no_token.len());
                                        return Some((yes_token, no_token));
                                    }
                                }
                            }
                        }
                    }
                    println!("[ROLLOVER] {} - no tokens found", slug);
                    None
                }
                Err(e) => {
                    println!("[ROLLOVER] {} - JSON error: {}", slug, e);
                    None
                }
            }
        }
        Err(e) => {
            println!("[ROLLOVER] {} - network error: {}", slug, e);
            None
        }
    }
}

/// Run the rollover monitoring thread
pub async fn run_rollover_thread(
    client: Arc<reqwest::Client>,
    rollover_tx: Sender<RolloverCommand>,
) {
    println!("🔄 [ROLLOVER] Thread started - monitoring for market transitions");
    
    let mut tracked_markets: HashSet<String> = HashSet::new();
    
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        
        let now = Utc::now();
        let periods = get_current_periods();
        let assets = ["btc", "eth", "sol", "xrp"];
        
        let mut active_this_cycle: HashSet<String> = HashSet::new();
        
        for asset in &assets {
            for (period_ts, timeframe) in &periods {
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
                        // Market doesn't exist yet (pre-trading) or expired
                    }
                }
            }
        }
        
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

print('✅ Rewrote market_rollover.rs')
