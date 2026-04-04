//! Token Hash → Token ID Mapping (Programmatic Binary Market Detection)
//! 
//! Queries Gamma API for ALL active events and filters by:
//! - outcomes == ["Yes", "No"]
//! - clobTokenIds is a JSON string containing 2-element array
//! 
//! This catches ALL binary markets regardless of slug format.

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

/// Build maps from ALL active binary markets
pub async fn build_maps(
    client: &Client,
) -> (HashMap<u64, String>, HashMap<String, String>, HashMap<u64, u64>) {
    let mut hash_to_id = HashMap::new();
    let mut id_to_condition = HashMap::new();
    let mut complement_map = HashMap::new();

    let url = "https://gamma-api.polymarket.com/events?closed=false&active=true&limit=500";

    println!("📊 Querying Gamma API for active binary markets...");
    
    match client.get(url).send().await {
        Ok(resp) => {
            match resp.json::<Value>().await {
                Ok(json) => {
                    if let Some(events) = json.as_array() {
                        let mut market_count = 0;
                        
                        for event in events {
                            if let Some(markets) = event["markets"].as_array() {
                                for market in markets {
                                    // Only accept binary Yes/No markets
                                    let outcomes = market["outcomes"].as_array();
                                    let is_binary = outcomes.map_or(false, |o| {
                                        o.len() == 2 && 
                                        o.get(0).and_then(|v| v.as_str()) == Some("Yes") &&
                                        o.get(1).and_then(|v| v.as_str()) == Some("No")
                                    });
                                    
                                    if !is_binary {
                                        continue;
                                    }

                                    let condition_id = market["conditionId"]
                                        .as_str()
                                        .unwrap_or("")
                                        .to_string();
                                    
                                    // CRITICAL: clobTokenIds is a JSON STRING, not an array!
                                    // API returns: "clobTokenIds": "[\"token1\", \"token2\"]"
                                    if let Some(clob_ids_str) = market["clobTokenIds"].as_str() {
                                        // Parse the JSON string
                                        if let Ok(clob_ids) = serde_json::from_str::<Vec<String>>(clob_ids_str) {
                                            if clob_ids.len() == 2 {
                                                let yes_str = clob_ids[0].clone();
                                                let no_str = clob_ids[1].clone();
                                                
                                                if yes_str.is_empty() || no_str.is_empty() {
                                                    continue;
                                                }
                                                
                                                let yes_hash = hash_token(&yes_str);
                                                let no_hash = hash_token(&no_str);

                                                // Map Hashes to Strings
                                                hash_to_id.insert(yes_hash, yes_str.clone());
                                                hash_to_id.insert(no_hash, no_str.clone());

                                                // Map Strings to Condition ID (For CTF Merges)
                                                id_to_condition.insert(yes_str.clone(), condition_id.clone());
                                                id_to_condition.insert(no_str.clone(), condition_id.clone());

                                                // Build the Complement Map (For Hot Path)
                                                complement_map.insert(yes_hash, no_hash);
                                                complement_map.insert(no_hash, yes_hash);
                                                
                                                market_count += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("✅ Found {} binary markets", market_count);
                    }
                }
                Err(e) => eprintln!("⚠️ Failed to parse Gamma API JSON: {}", e),
            }
        }
        Err(e) => eprintln!("⚠️ Failed to connect to Gamma API: {}", e),
    }
    
    println!("✅ Mapped {} token hashes, {} conditions, {} complement pairs", 
        hash_to_id.len(), id_to_condition.len(), complement_map.len() / 2);
    (hash_to_id, id_to_condition, complement_map)
}

// Legacy exports for backward compatibility
pub const MARKET_SLUGS: &[&str] = &[];

pub async fn build_condition_map(
    client: &Client,
    _slugs: &[&str],
) -> (HashMap<u64, String>, HashMap<String, String>) {
    let (hash_to_id, id_to_condition, _) = build_maps(client).await;
    (hash_to_id, id_to_condition)
}