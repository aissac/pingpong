#!/usr/bin/env python3
"""Write fixed market_rollover.rs with working rollover channel"""

code = r'''//! Market Rollover - Dynamic Token Pair Management
//!
//! Fetches new markets from Gamma API and sends AddPair/RemovePair commands
//! to hot path via crossbeam_channel.

use std::collections::{HashMap, HashSet};
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

/// Market period information
#[derive(Clone, Debug)]
pub struct MarketPeriod {
    pub slug: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub yes_token: String,
    pub no_token: String,
}

/// Rollover state
pub struct RolloverState {
    pub tracked_markets: HashSet<String>,  // yes_token -> tracked
}

impl RolloverState {
    pub fn new() -> Self {
        Self {
            tracked_markets: HashSet::new(),
        }
    }
}

/// Fetch active crypto markets from Gamma API
pub async fn fetch_active_markets(client: &reqwest::Client) -> Vec<MarketPeriod> {
    let url = "https://gamma-api.polymarket.com/events?active=true&closed=false&tag_slug=crypto";
    
    match client.get(url).send().await {
        Ok(response) => {
            match response.json::<Value>().await {
                Ok(json) => {
                    let mut periods = Vec::new();
                    
                    if let Some(events) = json.as_array() {
                        for event in events {
                            let slug = event["slug"].as_str().unwrap_or("");
                            
                            // Filter: btc-*, eth-*, *-up-or-down-*, 5m or 15m
                            if !slug.contains("-up-or-down-") {
                                continue;
                            }
                            if !(slug.starts_with("btc-") || slug.starts_with("eth-")) {
                                continue;
                            }
                            if !(slug.contains("-5m-") || slug.contains("-15m-")) {
                                continue;
                            }
                            
                            if let Some(markets) = event["markets"].as_array() {
                                for market in markets {
                                    let start_str = market["startDate"].as_str().unwrap_or("");
                                    let end_str = market["endDate"].as_str().unwrap_or("");
                                    
                                    let start_time = start_str.parse::<DateTime<Utc>>().ok();
                                    let end_time = end_str.parse::<DateTime<Utc>>().ok();
                                    
                                    if let (Some(start), Some(end)) = (start_time, end_time) {
                                        if let Some(tokens) = market["tokens"].as_array() {
                                            if tokens.len() == 2 {
                                                let yes_token = tokens[0]["token_id"].as_str().unwrap_or("").to_string();
                                                let no_token = tokens[1]["token_id"].as_str().unwrap_or("").to_string();
                                                
                                                if !yes_token.is_empty() && !no_token.is_empty() {
                                                    periods.push(MarketPeriod {
                                                        slug: slug.to_string(),
                                                        start_time: start,
                                                        end_time: end,
                                                        yes_token,
                                                        no_token,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    periods
                }
                Err(e) => {
                    eprintln!("[ROLLOVER] JSON parse error: {}", e);
                    Vec::new()
                }
            }
        }
        Err(e) => {
            eprintln!("[ROLLOVER] API fetch error: {}", e);
            Vec::new()
        }
    }
}

/// Run the rollover monitoring thread
pub async fn run_rollover_thread(
    client: reqwest::Client,
    rollover_tx: Sender<RolloverCommand>,
) {
    println!("🔄 [ROLLOVER] Thread started - monitoring for market transitions");
    
    let mut state = RolloverState::new();
    
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        
        let now = Utc::now();
        let pre_sub_window = now + chrono::Duration::minutes(2);
        
        // Fetch active markets
        let periods = fetch_active_markets(&client).await;
        
        let mut active_this_cycle: HashSet<String> = HashSet::new();
        
        for period in &periods {
            let market_id = period.yes_token.clone();
            
            // Market hasn't ended yet
            if period.end_time > now {
                active_this_cycle.insert(market_id.clone());
                
                // Market starts within 2 minutes (pre-subscription window)
                if period.start_time <= pre_sub_window && period.start_time > now {
                    if !state.tracked_markets.contains(&market_id) {
                        println!("🟢 [ROLLOVER] Adding new market: {} (starts {})", 
                            period.slug, period.start_time.format("%H:%M:%S"));
                        
                        let yes_hash = hash_token(&period.yes_token);
                        let no_hash = hash_token(&period.no_token);
                        
                        // Send AddPair to hot path
                        if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(yes_hash, no_hash)) {
                            eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                            break;
                        }
                        
                        state.tracked_markets.insert(market_id.clone());
                    }
                } else if period.start_time <= now && !state.tracked_markets.contains(&market_id) {
                    // Market already started but we weren't tracking it (startup catch-up)
                    println!("🟢 [ROLLOVER] Catch-up adding: {} (started {})", 
                        period.slug, period.start_time.format("%H:%M:%S"));
                    
                    let yes_hash = hash_token(&period.yes_token);
                    let no_hash = hash_token(&period.no_token);
                    
                    if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(yes_hash, no_hash)) {
                        eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                        break;
                    }
                    
                    state.tracked_markets.insert(market_id.clone());
                }
            }
        }
        
        // Find expired markets
        let expired: Vec<String> = state.tracked_markets
            .iter()
            .filter(|id| !active_this_cycle.contains(*id))
            .cloned()
            .collect();
        
        for expired_id in expired {
            println!("🔴 [ROLLOVER] Market expired, removing: {}", expired_id);
            
            let expired_hash = hash_token(&expired_id);
            
            let _ = rollover_tx.send(RolloverCommand::RemovePair(expired_hash));
            state.tracked_markets.remove(&expired_id);
        }
        
        // Status log
        println!("[ROLLOVER] {} markets tracked, {} active periods fetched", 
            state.tracked_markets.len(), periods.len());
    }
}
'''

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'w') as f:
    f.write(code)

print('✅ Wrote fixed market_rollover.rs')
