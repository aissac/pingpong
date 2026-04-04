//! Crypto Market Discovery for HFT Bot
//! 
//! Fetches active BTC/ETH/SOL/XRP 5m/15m Up/Down markets from Polymarket Gamma API
//! 
//! FIX (2026-04-01): Use /events endpoint with proper pagination
//! Old approach (?slug=btc-updown-5m) returned [] because slug needs exact match
//! New approach: Fetch all crypto events, filter locally

use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use rustc_hash::FxHashMap;

/// Fetch active crypto markets from Gamma API
/// 
/// FIX: Use /events endpoint with limit=100 to bypass pagination
/// Then filter locally for BTC/ETH/SOL/XRP 5m/15m markets
/// 
/// Returns:
/// - Vec<String>: token IDs for WebSocket subscription
/// - HashMap<u64, String>: hash -> token_id mapping
/// - Vec<String>: market slugs for tracking
pub async fn fetch_active_crypto_markets(
    client: &Client,
) -> (Vec<String>, FxHashMap<u64, String>, Vec<String>) {
    let mut token_ids = Vec::new();
    let mut hash_to_id = FxHashMap::default();
    let mut market_slugs = Vec::new();
    
    // FIX: Use /events endpoint with proper pagination (NotebookLM recommendation)
    let url = "https://gamma-api.polymarket.com/events?limit=100&active=true&closed=false&tag_slug=crypto";
    
    println!("🎯 Fetching crypto markets from: {}", url);
    
    let resp = match client.get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Gamma API request failed: {}", e);
            return (token_ids, hash_to_id, market_slugs);
        }
    };
    
    let events: Vec<Value> = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("❌ Failed to parse Gamma API response: {}", e);
            return (token_ids, hash_to_id, market_slugs);
        }
    };
    
    println!("📊 Received {} events from Gamma API", events.len());
    
    // Filter for BTC/ETH/SOL/XRP 5m/15m markets
    for event in &events {
        let event_slug = event["slug"].as_str().unwrap_or("");
        
        // Check if this is a crypto Up/Down market we care about
        let is_target_asset = event_slug.starts_with("btc-") 
            || event_slug.starts_with("eth-") 
            || event_slug.starts_with("sol-")
            || event_slug.starts_with("xrp-");
        
        let is_target_timeframe = event_slug.contains("-5m-") 
            || event_slug.contains("-15m-");
        
        if !is_target_asset || !is_target_timeframe {
            continue;
        }
        
        // Check if market is active and on CLOB
        let active = event["active"].as_bool().unwrap_or(false);
        let closed = event["closed"].as_bool().unwrap_or(true);
        let enable_orderbook = event["enableOrderBook"].as_bool().unwrap_or(false);
        
        if !active || closed || !enable_orderbook {
            continue;
        }
        
        // Extract tokens from markets array
        if let Some(markets) = event["markets"].as_array() {
            for market in markets {
                // Verify it's a binary market (Yes/No outcomes)
                let outcomes = market["outcomes"].as_array();
                if outcomes.map_or(true, |o| o.len() != 2) {
                    continue;
                }
                
                // Get token IDs
                if let Some(tokens) = market["tokens"].as_array() {
                    if tokens.len() == 2 {
                        let yes_token = tokens[0]["token_id"].as_str().unwrap_or("").to_string();
                        let no_token = tokens[1]["token_id"].as_str().unwrap_or("").to_string();
                        
                        if yes_token.is_empty() || no_token.is_empty() {
                            continue;
                        }
                        
                        // Hash tokens for fast lookup
                        let yes_hash = crate::token_map::hash_token(&yes_token);
                        let no_hash = crate::token_map::hash_token(&no_token);
                        
                        println!("🟢 [ACTIVE] {} market | YES={} NO={}", 
                            event_slug, 
                            &yes_token[..20.min(yes_token.len())], 
                            &no_token[..20.min(no_token.len())]);
                        
                        token_ids.push(yes_token.clone());
                        token_ids.push(no_token.clone());
                        hash_to_id.insert(yes_hash, yes_token);
                        hash_to_id.insert(no_hash, no_token);
                        market_slugs.push(event_slug.to_string());
                    }
                }
            }
        }
    }
    
    println!("✅ Discovered {} tokens from {} markets", 
        token_ids.len(), 
        market_slugs.len());
    
    (token_ids, hash_to_id, market_slugs)
}

/// Get current time periods for 5m/15m markets
/// Returns (timestamp, timeframe) pairs for current and next periods
pub fn get_current_periods() -> Vec<(i64, &'static str)> {
    use chrono::{Utc, Timelike};
    
    let now = Utc::now();
    let mut periods = Vec::new();
    
    // 5-minute periods (round to nearest 5min)
    let minute_5 = (now.minute() / 5) * 5;
    let period_5m = now.with_minute(minute_5).unwrap()
        .with_second(0).unwrap()
        .with_nanosecond(0).unwrap();
    let ts_5m = period_5m.timestamp();
    periods.push((ts_5m, "5m"));
    periods.push((ts_5m + 300, "5m")); // Next 5m period
    
    // 15-minute periods (round to nearest 15min)
    let minute_15 = (now.minute() / 15) * 15;
    let period_15m = now.with_minute(minute_15).unwrap()
        .with_second(0).unwrap()
        .with_nanosecond(0).unwrap();
    let ts_15m = period_15m.timestamp();
    periods.push((ts_15m, "15m"));
    periods.push((ts_15m + 900, "15m")); // Next 15m period
    
    periods
}

/// Fetch market by slug from Gamma API (for rollover verification)
pub async fn fetch_market_by_slug(
    client: &Client,
    slug: &str,
) -> Option<(String, String, String)> {
    // FIX: Use /events endpoint and filter locally
    let url = "https://gamma-api.polymarket.com/events?limit=100&active=true&closed=false&tag_slug=crypto";
    
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    
    let events: Vec<Value> = resp.json().await.ok()?;
    
    for event in events {
        let event_slug = event["slug"].as_str().unwrap_or("");
        
        // Check if this event matches our target slug (contains match)
        if event_slug.contains(slug) {
            if let Some(markets) = event["markets"].as_array() {
                for market in markets {
                    let active = market["active"].as_bool().unwrap_or(false);
                    let closed = market["closed"].as_bool().unwrap_or(true);
                    
                    if !active || closed {
                        continue;
                    }
                    
                    if let Some(tokens) = market["tokens"].as_array() {
                        if tokens.len() == 2 {
                            let yes_token = tokens[0]["token_id"].as_str()?.to_string();
                            let no_token = tokens[1]["token_id"].as_str()?.to_string();
                            let condition_id = market["conditionId"].as_str()?.to_string();
                            
                            return Some((condition_id, yes_token, no_token));
                        }
                    }
                }
            }
        }
    }
    
    None
}
