#!/usr/bin/env python3
"""Fix rollover - add limit=100 to API query to get ALL markets (not just first 20)"""

code = r'''//! Market Rollover - Dynamic Token Pair Management
//!
//! Fetches ALL active crypto markets (limit=100 to avoid pagination).
//! Filters in-memory for 5m/15m markets.
//! Sends AddPair/RemovePair commands to hot path via crossbeam_channel.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
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

/// Fetch ALL active crypto markets (limit=100 to avoid pagination trap)
async fn fetch_all_active_markets(client: &reqwest::Client) -> Vec<(String, String, String)> {
    // CRITICAL: limit=100 to get ALL markets, not just first 20
    let url = "https://gamma-api.polymarket.com/events?limit=100&active=true&closed=false&tag_slug=crypto";
    
    match client.get(url).send().await {
        Ok(response) => {
            if !response.status().is_success() {
                println!("[ROLLOVER] API status: {}", response.status());
                return Vec::new();
            }
            
            match response.json::<Value>().await {
                Ok(json) => {
                    let mut markets = Vec::new();
                    
                    if let Some(events) = json.as_array() {
                        for event in events {
                            let slug = event.get("slug")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_lowercase();
                            
                            // Filter for 5m/15m markets using string matching
                            let is_5m_15m = slug.contains("-5m-") || slug.contains("-15m-");
                            let is_target_asset = slug.starts_with("btc-") 
                                || slug.starts_with("eth-")
                                || slug.starts_with("sol-")
                                || slug.starts_with("xrp-");
                            
                            if !is_5m_15m || !is_target_asset {
                                continue;
                            }
                            
                            // Extract tokens from markets in this event
                            if let Some(event_markets) = event.get("markets").and_then(|v| v.as_array()) {
                                for market in event_markets {
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
                                                markets.push((slug.clone(), yes_token, no_token));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    println!("[ROLLOVER] Found {} markets matching 5m/15m criteria", markets.len());
                    markets
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
    println!("🔄 [ROLLOVER] Thread started - fetching ALL active markets (limit=100)");
    
    let mut tracked_markets: HashSet<String> = HashSet::new();
    
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        
        // Fetch ALL markets with limit=100
        let all_markets = fetch_all_active_markets(&client).await;
        
        let mut active_this_cycle: HashSet<String> = HashSet::new();
        
        for (slug, yes_token, no_token) in &all_markets {
            let market_id = yes_token.clone();
            active_this_cycle.insert(market_id.clone());
            
            // Add if not already tracked
            if !tracked_markets.contains(market_id) {
                println!("🟢 [ROLLOVER] Adding: {}", slug);
                
                let yes_hash = hash_token(yes_token);
                let no_hash = hash_token(no_token);
                
                if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(yes_hash, no_hash)) {
                    eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                    return;
                }
                
                tracked_markets.insert(market_id.clone());
                println!("[ROLLOVER] Now tracking {} markets", tracked_markets.len());
            }
        }
        
        // Remove markets no longer in active list (expired)
        let expired: Vec<String> = tracked_markets
            .iter()
            .filter(|id| !active_this_cycle.contains(*id))
            .cloned()
            .collect();
        
        for expired_id in expired {
            println!("🔴 [ROLLOVER] Removing expired: {}", expired_id);
            
            let expired_hash = hash_token(&expired_id);
            
            let _ = rollover_tx.send(RolloverCommand::RemovePair(expired_hash));
            tracked_markets.remove(&expired_id);
        }
    }
}
'''

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'w') as f:
    f.write(code)

print('✅ Fixed: Added limit=100 to API query - will now see ALL markets')
