#!/usr/bin/env python3
"""Rewrite rollover to use broad events endpoint instead of guessing slugs"""

code = r'''//! Market Rollover - Dynamic Token Pair Management
//!
//! Fetches upcoming markets from Gamma API events endpoint.
//! Filters in-memory for BTC/ETH/SOL/XRP 5m/15m markets.
//! Adds markets that start within 2 minutes (pre-trading phase).
//! Sends AddPair/RemovePair commands to hot path via crossbeam_channel.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use chrono::{Utc, DateTime};
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

/// Fetch all active crypto events and filter for upcoming 5m/15m markets
async fn fetch_upcoming_markets(client: &reqwest::Client) -> Vec<(String, String, String)> {
    // Broad endpoint - returns ALL active crypto events including pre-trading
    let url = "https://gamma-api.polymarket.com/events?active=true&closed=false&tag_slug=crypto";
    
    match client.get(url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                println!("[ROLLOVER] API status: {}", response.status());
                return Vec::new();
            }
            
            match response.json::<Value>().await {
                Ok(json) => {
                    let mut upcoming = Vec::new();
                    let now = Utc::now();
                    let window_end = now + Duration::from_secs(120); // 2 minute window
                    
                    if let Some(events) = json.as_array() {
                        for event in events {
                            let slug = event.get("slug")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            
                            // Filter: btc-*, eth-*, sol-*, xrp-* with -5m- or -15m-
                            let is_target_asset = slug.starts_with("btc-") 
                                || slug.starts_with("eth-")
                                || slug.starts_with("sol-")
                                || slug.starts_with("xrp-");
                            
                            let is_target_timeframe = slug.contains("-5m-") || slug.contains("-15m-");
                            
                            if !is_target_asset || !is_target_timeframe {
                                continue;
                            }
                            
                            // Check markets within this event
                            if let Some(markets) = event.get("markets").and_then(|v| v.as_array()) {
                                for market in markets {
                                    // Parse start date
                                    let start_str = market.get("startDate")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");
                                    
                                    if let Ok(start_time) = DateTime::parse_from_rfc3339(start_str) {
                                        let start_utc = start_time.with_timezone(&Utc);
                                        
                                        // Only add if starting within 2 minutes (pre-trading or just started)
                                        if start_utc > now && start_utc <= window_end {
                                            println!("[ROLLOVER] Found upcoming: {} starts at {}", 
                                                slug, start_utc.format("%H:%M:%S"));
                                            
                                            // Extract tokens
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
                                                        upcoming.push((slug.clone(), yes_token, no_token));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    println!("[ROLLOVER] Found {} upcoming markets", upcoming.len());
                    upcoming
                }
                Err(e) => {
                    println!("[ROLLOVER] JSON parse error: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            println!("[ROLLOVER] Network error: {}", e);
            Vec::new()
        }
    }
}

/// Run the rollover monitoring thread
pub async fn run_rollover_thread(
    client: Arc<reqwest::Client>,
    rollover_tx: Sender<RolloverCommand>,
) {
    println!("🔄 [ROLLOVER] Thread started - monitoring for upcoming markets");
    
    let mut tracked_markets: HashSet<String> = HashSet::new();
    
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        
        // Fetch all upcoming markets from broad endpoint
        let upcoming = fetch_upcoming_markets(&client).await;
        
        let mut active_this_cycle: HashSet<String> = HashSet::new();
        
        for (slug, yes_token, no_token) in upcoming {
            let market_id = yes_token.clone();
            active_this_cycle.insert(market_id.clone());
            
            // Add if not already tracked
            if !tracked_markets.contains(&market_id) {
                println!("🟢 [ROLLOVER] Adding market: {}", slug);
                
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
        
        // Remove markets that are no longer in upcoming list (expired)
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

print('✅ Rewrote rollover to use broad events endpoint')
