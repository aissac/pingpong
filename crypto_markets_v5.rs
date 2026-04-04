//! Crypto Market Discovery for HFT Bot
//! 
//! Fetches active BTC/ETH/SOL/XRP 5m/15m Up/Down markets from Polymarket Gamma API
//! 
//! FIX (2026-04-01): Use pagination to fetch 2000+ markets
//! Crypto 5m/15m markets exist at positions #400-#1800

use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;

/// Fetch active crypto markets from Gamma API
/// 
/// FIX: Use pagination to fetch 2000 markets in 4 batches of 500
/// Crypto 5m/15m markets rank #400-#1800 by volume
pub async fn fetch_active_crypto_markets(
    client: &Client,
) -> (Vec<String>, HashMap<u64, String>, Vec<String>) {
    let mut token_ids = Vec::new();
    let mut hash_to_id = HashMap::new();
    let mut market_slugs = Vec::new();
    
    // FIX: Fetch 2000 markets in batches (API max is 500 per request)
    let mut all_markets = Vec::new();
    
    for offset in [0, 500, 1000, 1500] {
        let url = format!("https://gamma-api.polymarket.com/markets?limit=500&active=true&closed=false&offset={}", offset);
        
        let resp = match client.get(&url).send().await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Gamma API request failed for offset {}: {}", offset, e);
                continue;
            }
        };
        
        let markets: Vec<Value> = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to parse Gamma API response: {}", e);
                continue;
            }
        };
        
        if markets.is_empty() {
            break; // No more markets
        }
        
        all_markets.extend(markets);
    }
    
    println!("Fetched {} total markets from Gamma API", all_markets.len());
    
    // Filter for BTC/ETH/SOL/XRP 5m/15m markets by SLUG PATTERN
    for market in &all_markets {
        let slug = market["slug"].as_str().unwrap_or("");
        
        // Check if this is a crypto Up/Down market we care about
        let is_target_asset = slug.starts_with("btc-") 
            || slug.starts_with("eth-") 
            || slug.starts_with("sol-")
            || slug.starts_with("xrp-");
        
        let is_target_timeframe = slug.contains("-5m-") 
            || slug.contains("-15m-")
            || slug.contains("-updown-");
        
        if !is_target_asset || !is_target_timeframe {
            continue;
        }
        
        // Check if market is active
        let active = market["active"].as_bool().unwrap_or(false);
        let closed = market["closed"].as_bool().unwrap_or(true);
        
        if !active || closed {
            continue;
        }
        
        // Get token IDs directly from market
        if let Some(tokens) = market.get("tokens").and_then(|v| v.as_array()) {
            if tokens.len() == 2 {
                let yes_token = tokens[0]["token_id"].as_str().unwrap_or("").to_string();
                let no_token = tokens[1]["token_id"].as_str().unwrap_or("").to_string();
                
                if yes_token.is_empty() || no_token.is_empty() {
                    continue;
                }
                
                // Hash tokens for fast lookup
                let yes_hash = crate::token_map::hash_token(&yes_token);
                let no_hash = crate::token_map::hash_token(&no_token);
                
                println!("[ACTIVE] {} | YES={} NO={}", 
                    slug, 
                    &yes_token[..20.min(yes_token.len())], 
                    &no_token[..20.min(no_token.len())]);
                
                token_ids.push(yes_token.clone());
                token_ids.push(no_token.clone());
                hash_to_id.insert(yes_hash, yes_token);
                hash_to_id.insert(no_hash, no_token);
                market_slugs.push(slug.to_string());
            }
        }
    }
    
    println!("Discovered {} tokens from {} markets", 
        token_ids.len(), 
        market_slugs.len());
    
    (token_ids, hash_to_id, market_slugs)
}

/// Get current time periods for 5m/15m markets
pub fn get_current_periods() -> Vec<(i64, &'static str)> {
    use chrono::{Utc, Timelike};
    
    let now = Utc::now();
    let mut periods = Vec::new();
    
    // 5-minute periods
    let minute_5 = (now.minute() / 5) * 5;
    let period_5m = now.with_minute(minute_5).unwrap()
        .with_second(0).unwrap()
        .with_nanosecond(0).unwrap();
    let ts_5m = period_5m.timestamp();
    periods.push((ts_5m, "5m"));
    periods.push((ts_5m + 300, "5m"));
    
    // 15-minute periods
    let minute_15 = (now.minute() / 15) * 15;
    let period_15m = now.with_minute(minute_15).unwrap()
        .with_second(0).unwrap()
        .with_nanosecond(0).unwrap();
    let ts_15m = period_15m.timestamp();
    periods.push((ts_15m, "15m"));
    periods.push((ts_15m + 900, "15m"));
    
    periods
}

/// Fetch market by slug from Gamma API (for rollover)
pub async fn fetch_market_by_slug(
    client: &Client,
    slug: &str,
) -> Option<(String, String, String)> {
    let url = format!("https://gamma-api.polymarket.com/markets?limit=1000&active=true&closed=false&search={}", slug);
    
    let resp = client.get(&url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    
    let markets: Vec<Value> = resp.json().await.ok()?;
    
    for market in markets {
        let market_slug = market["slug"].as_str().unwrap_or("");
        
        if market_slug.contains(slug) {
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
    
    None
}
