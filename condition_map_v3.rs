//! Token Hash → Token ID Mapping (Dynamic Slug Filtering)
//! 
//! Queries Gamma API for ALL active events and filters locally
//! for BTC/ETH up/down markets regardless of exact timestamp

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use reqwest::Client;
use serde_json::Value;

/// Simple u64 hasher for hot path tokens
pub fn hash_token(token_str: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token_str.hash(&mut hasher);
    hasher.finish()
}

/// Build maps from ALL active events, filtering locally for crypto up/down
pub async fn build_maps(
    client: &Client,
) -> (HashMap<u64, String>, HashMap<String, String>) {
    let mut hash_to_id = HashMap::new();
    let mut id_to_condition = HashMap::new();

    // Query for currently active, unclosed events
    let url = "https://gamma-api.polymarket.com/events?closed=false&active=true&limit=1000";

    println!("📊 Querying Gamma API for active markets...");
    
    match client.get(url).send().await {
        Ok(resp) => {
            match resp.json::<Value>().await {
                Ok(json) => {
                    if let Some(events) = json.as_array() {
                        let mut market_count = 0;
                        
                        for event in events {
                            let slug = event["slug"].as_str().unwrap_or("").to_lowercase();
                            
                            // Dynamically catch any 5m or 15m crypto market slug
                            if slug.contains("btc-updown-15m") || 
                               slug.contains("eth-updown-15m") ||
                               slug.contains("btc-updown-5m") || 
                               slug.contains("eth-updown-5m") ||
                               slug.contains("sol-updown-15m") || 
                               slug.contains("sol-updown-5m") {
                                
                                if let Some(markets) = event["markets"].as_array() {
                                    for market in markets {
                                        let condition_id = market["conditionId"]
                                            .as_str().unwrap_or("").to_string();
                                        
                                        // Use clobTokenIds (array of token ID strings)
                                        if let Some(token_ids_str) = market["clobTokenIds"].as_str() {
                                            if let Ok(token_ids) = serde_json::from_str::<Vec<String>>(token_ids_str) {
                                                if token_ids.len() >= 2 {
                                                    // First is YES, second is NO
                                                    for token_id in &token_ids {
                                                        let token_hash = hash_token(token_id);
                                                        hash_to_id.insert(token_hash, token_id.clone());
                                                        id_to_condition.insert(token_id.clone(), condition_id.clone());
                                                    }
                                                    market_count += 1;
                                                    println!("✅ Found: {} (condition: {}..{})", 
                                                        slug, &condition_id[..10], &condition_id[condition_id.len()-6..]);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("✅ Found {} up/down markets", market_count);
                    }
                }
                Err(e) => eprintln!("⚠️ Failed to parse Gamma API JSON: {}", e),
            }
        }
        Err(e) => eprintln!("⚠️ Failed to connect to Gamma API: {}", e),
    }
    
    println!("✅ Mapped {} token hashes, {} conditions", hash_to_id.len(), id_to_condition.len());
    (hash_to_id, id_to_condition)
}