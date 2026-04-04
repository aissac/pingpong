//! Crypto Market Discovery for HFT Bot
//! 
//! Fetches active BTC/ETH/SOL/XRP 5m/15m Up/Down markets from Polymarket Gamma API
//! 
//! FIX (2026-04-01): Deterministic slug generation + expiry filtering
//! Only subscribe to markets that are currently ACTIVE (not expired)

use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate deterministic slugs for a given asset and interval
/// 
/// Polymarket creates markets with predictable slugs:
/// btc-updown-15m-1775067300 (timestamp = period END time)
/// 
/// Returns slugs for current period and next 2 periods (for pre-subscription)
pub fn generate_target_slugs(asset: &str, interval_mins: u64) -> Vec<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let interval_secs = interval_mins * 60;
    
    // Round UP to next interval boundary (:00, :15, :30, :45 for 15m)
    // This gives us the END time of the current period
    let current_period_end = ((now / interval_secs) + 1) * interval_secs;
    
    // Next periods for pre-subscription
    let next_period_end = current_period_end + interval_secs;
    let next2_period_end = next_period_end + interval_secs;
    
    vec![
        format!("{}-updown-{}m-{}", asset, interval_mins, current_period_end),
        format!("{}-updown-{}m-{}", asset, interval_mins, next_period_end),
        format!("{}-updown-{}m-{}", asset, interval_mins, next2_period_end),
    ]
}

/// Check if a market is currently active (not expired)
fn is_market_active(end_timestamp: u64, interval_secs: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let start_timestamp = end_timestamp - interval_secs;
    
    // Market is active if: start <= now < end
    now >= start_timestamp && now < end_timestamp
}

/// Fetch a single market by exact slug
/// 
/// Returns (YES_token_id, NO_token_id) if market exists and is ACTIVE
pub async fn fetch_market_by_slug(
    client: &Client,
    slug: &str,
) -> Option<(String, String)> {
    let url = format!("https://gamma-api.polymarket.com/markets?slug={}", slug);
    
    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            // Market doesn't exist yet (will be created by Polymarket)
            return None;
        }
    };
    
    let markets: Vec<Value> = match resp.json().await {
        Ok(v) => v,
        Err(_) => return None,
    };
    
    if markets.is_empty() {
        return None; // Market not created yet
    }
    
    let market = &markets[0];
    
    // Check if active (from API)
    let active = market["active"].as_bool().unwrap_or(false);
    let closed = market["closed"].as_bool().unwrap_or(true);
    
    if !active || closed {
        return None;
    }
    
    // Extract end timestamp from slug to check if expired
    let interval_secs = if slug.contains("-5m-") { 300 } else { 900 };
    let end_timestamp: u64 = slug.split('-').last()?.parse().ok()?;
    
    // DOUBLE CHECK: Is market actually still active (not expired)?
    if !is_market_active(end_timestamp, interval_secs) {
        println!("[SKIP] {} - expired", slug);
        return None;
    }
    
    // FIX: Get token IDs from clobTokenIds field (JSON string array)
    let clob_token_ids_str = market["clobTokenIds"].as_str()?;
    let clob_token_ids: Vec<Value> = serde_json::from_str(clob_token_ids_str).ok()?;
    
    if clob_token_ids.len() != 2 {
        return None;
    }
    
    let yes_token = clob_token_ids[0].as_str()?.to_string();
    let no_token = clob_token_ids[1].as_str()?.to_string();
    
    Some((yes_token, no_token))
}

/// Fetch active crypto markets using deterministic discovery
/// 
/// This is the NEW approach (replaces search/filter):
/// 1. Calculate current time periods for 5m and 15m
/// 2. Generate expected slugs: btc-updown-5m-{timestamp}
/// 3. Query each slug directly
/// 4. Filter out expired markets
/// 5. Return token IDs for markets that are ACTIVE NOW
pub async fn fetch_active_crypto_markets(
    client: &Client,
) -> (Vec<String>, HashMap<u64, String>, Vec<String>) {
    let mut token_ids = Vec::new();
    let mut hash_to_id = HashMap::new();
    let mut market_slugs = Vec::new();
    
    let assets = vec!["btc", "eth", "sol", "xrp"];
    
    println!("Fetching crypto markets using deterministic discovery...");
    
    for asset in assets {
        // Generate slugs for 15m intervals
        let slugs_15m = generate_target_slugs(asset, 15);
        
        // Generate slugs for 5m intervals
        let slugs_5m = generate_target_slugs(asset, 5);
        
        // Combine all slugs to check
        let mut all_slugs = Vec::new();
        all_slugs.extend(slugs_15m);
        all_slugs.extend(slugs_5m);
        
        for slug in all_slugs {
            if let Some((yes_token, no_token)) = fetch_market_by_slug(client, &slug).await {
                println!("[ACTIVE] {} | YES={} NO={}", 
                    slug, 
                    &yes_token[..20.min(yes_token.len())], 
                    &no_token[..20.min(no_token.len())]);
                
                token_ids.push(yes_token.clone());
                token_ids.push(no_token.clone());
                
                let yes_hash = crate::token_map::hash_token(&yes_token);
                let no_hash = crate::token_map::hash_token(&no_token);
                
                hash_to_id.insert(yes_hash, yes_token);
                hash_to_id.insert(no_hash, no_token);
                market_slugs.push(slug);
            }
        }
    }
    
    println!("Discovered {} tokens from {} markets", 
        token_ids.len(), 
        market_slugs.len());
    
    (token_ids, hash_to_id, market_slugs)
}

/// Get current time periods for 5m/15m markets (legacy, kept for compatibility)
pub fn get_current_periods() -> Vec<(i64, &'static str)> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    
    let mut periods = Vec::new();
    
    // 5-minute periods
    let period_5m = ((now / 300) + 1) * 300;
    periods.push((period_5m, "5m"));
    periods.push((period_5m + 300, "5m"));
    
    // 15-minute periods
    let period_15m = ((now / 900) + 1) * 900;
    periods.push((period_15m, "15m"));
    periods.push((period_15m + 900, "15m"));
    
    periods
}

/// Fetch market by slug from Gamma API (for rollover)
pub async fn fetch_market_by_slug_rollover(
    client: &Client,
    slug: &str,
) -> Option<(String, String, String)> {
    let (yes_token, no_token) = fetch_market_by_slug(client, slug).await?;
    
    // For rollover, we also need conditionId - fetch again to get it
    let url = format!("https://gamma-api.polymarket.com/markets?slug={}", slug);
    let resp = client.get(&url).send().await.ok()?;
    let markets: Vec<Value> = resp.json().await.ok()?;
    let market = markets.first()?;
    
    let condition_id = market["conditionId"].as_str()?.to_string();
    
    Some((condition_id, yes_token, no_token))
}
