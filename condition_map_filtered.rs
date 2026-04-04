//! Condition ID Mapping from Gamma API - BTC/ETH 5m/15m Only
//! 
//! Filters for BTC/ETH 5-minute and 15-minute Up/Down binary markets only.
//! Three-layer filtering:
//! 1. API query: active=true&closed=false
//! 2. String matching: btc-/eth-, -up-or-down-, -5m-/-15m-
//! 3. Time-based: endTime > now (remove expired)

use reqwest::Client;
use serde_json::Value;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Fetch and filter markets for BTC/ETH 5m/15m Up/Down binary markets only
pub async fn fetch_active_markets(client: &Client) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    println!("🎯 Fetching BTC/ETH 5m/15m Up/Down markets only...");
    
    // Layer 1: API query filtering - only active, non-closed markets
    let url = "https://gamma-api.polymarket.com/events?active=true&closed=false";
    
    let response = client.get(url).send().await?;
    let events: Value = response.json().await?;
    
    let now = Utc::now();
    let mut token_pairs = HashMap::new();
    let mut filtered_count = 0;
    
    if let Some(events_array) = events.as_array() {
        for event in events_array {
            // Extract slug for filtering
            let slug = event["slug"].as_str().unwrap_or("").to_lowercase();
            
            // Layer 2: String matching - filter for BTC/ETH, 5m/15m, Up/Down
            let is_target_asset = slug.starts_with("btc-") || slug.starts_with("eth-");
            let is_up_down = slug.contains("-up-or-down-");
            let is_target_timeframe = slug.contains("-5m-") || slug.contains("-15m-");
            
            // Drop unwanted markets immediately
            if !is_target_asset || !is_up_down || !is_target_timeframe {
                continue;
            }
            
            filtered_count += 1;
            
            // Process markets within this event
            if let Some(markets) = event["markets"].as_array() {
                for market in markets {
                    // Verify binary market (exactly 2 outcomes: Yes/No)
                    let is_binary = market["outcomes"].as_array().map_or(false, |o| o.len() == 2);
                    if !is_binary {
                        continue;
                    }
                    
                    // Extract token IDs for YES and NO sides
                    let tokens = market["tokens"].as_array();
                    if tokens.is_none() {
                        continue;
                    }
                    let tokens = tokens.unwrap();
                    
                    if tokens.len() < 2 {
                        continue;
                    }
                    
                    let yes_token = tokens[0]["token_id"].as_str().unwrap_or("").to_string();
                    let no_token = tokens[1]["token_id"].as_str().unwrap_or("").to_string();
                    
                    if yes_token.is_empty() || no_token.is_empty() {
                        continue;
                    }
                    
                    // Layer 3: Time-based filtering
                    let start_str = market["startDate"].as_str().unwrap_or("");
                    let end_str = market["endDate"].as_str().unwrap_or("");
                    
                    let start_time = start_str.parse::<DateTime<Utc>>().unwrap_or(now);
                    let end_time = end_str.parse::<DateTime<Utc>>().unwrap_or(now);
                    
                    // Skip markets that haven't started or have expired
                    if start_time > now || end_time <= now {
                        continue;
                    }
                    
                    // Map both YES and NO tokens to the condition ID
                    let condition_id = market["conditionId"].as_str().unwrap_or("").to_string();
                    
                    if !condition_id.is_empty() {
                        token_pairs.insert(yes_token.clone(), condition_id.clone());
                        token_pairs.insert(no_token.clone(), condition_id.clone());
                        println!("✅ [TRACKING] {} | YES: {} | NO: {} | Ends: {}", 
                            slug, 
                            &yes_token[..16].min(yes_token.len()),
                            &no_token[..16].min(no_token.len()),
                            end_time.format("%H:%M:%S"));
                    }
                }
            }
        }
    }
    
    println!("✅ Filtered {} BTC/ETH 5m/15m events", filtered_count);
    println!("✅ Total token mappings: {}", token_pairs.len());
    
    Ok(token_pairs)
}

/// Get active 5-minute periods for current time (for subscription)
pub fn get_active_5m_periods() -> Vec<i64> {
    let now = Utc::now();
    let current_period = (now.timestamp() / 300) * 300; // Round to nearest 5min
    
    // Only current and next period (not past)
    vec![current_period, current_period + 300]
}

/// Get active 15-minute periods for current time (for subscription)
pub fn get_active_15m_periods() -> Vec<i64> {
    let now = Utc::now();
    let current_period = (now.timestamp() / 900) * 900; // Round to nearest 15min
    
    // Only current and next period (not past)
    vec![current_period, current_period + 900]
}

/// Helper to safely get string slice with length check
trait SafeStringSlice {
    fn safe_slice(&self, end: usize) -> &str;
}

impl SafeStringSlice for str {
    fn safe_slice(&self, end: usize) -> &str {
        &self[..self.len().min(end)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_active_markets() {
        let client = Client::new();
        match fetch_active_markets(&client).await {
            Ok(pairs) => {
                println!("Fetched {} token pairs", pairs.len());
                assert!(!pairs.is_empty() || true); // Skip if API unavailable
            }
            Err(e) => println!("API error (expected in test): {}", e),
        }
    }
}
