//! Token Hash → Token ID Mapping (Programmatic Binary Market Detection)
//! 
//! Queries Gamma API for ALL active events and filters by:
//! - outcomes == ["Yes", "No"]
//! - closed == false (at market level)
//! - clobTokenIds is a JSON string containing 2-element array
//! - TIME-BASED FILTERING: startDate <= now <= endDate
//! 
//! This catches ALL binary markets regardless of slug format.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use reqwest::Client;
use serde_json::Value;
use chrono::{DateTime, Utc};

/// Simple u64 hasher for hot path tokens
pub fn hash_token(token_str: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    token_str.hash(&mut hasher);
    hasher.finish()
}

/// Parse ISO 8601 date string safely
fn parse_iso_date(date_str: &str) -> Option<DateTime<Utc>> {
    date_str.parse::<DateTime<Utc>>().ok()
}

/// Build maps from ALL ACTIVE binary markets (time-filtered)
pub async fn build_maps(
    client: &Client,
) -> (HashMap<u64, String>, HashMap<String, String>, HashMap<u64, u64>) {
    let mut hash_to_id = HashMap::new();
    let mut id_to_condition = HashMap::new();
    let mut complement_map = HashMap::new();

    let url = "https://gamma-api.polymarket.com/events?closed=false&active=true&limit=500";
    let now = Utc::now();

    println!("📊 Querying Gamma API for ACTIVE binary markets...");
    println!("⏰ Current time: {}", now.format("%Y-%m-%d %H:%M:%S UTC"));
    
    match client.get(url).send().await {
        Ok(resp) => {
            match resp.json::<Value>().await {
                Ok(json) => {
                    if let Some(events) = json.as_array() {
                        let mut market_count = 0;
                        let mut skipped_closed = 0;
                        let mut skipped_expired = 0;
                        let mut skipped_future = 0;
                        let mut skipped_dates = 0;
                        
                        for event in events {
                            // Event-level dates (fallback)
                            let event_start_str = event["startDate"].as_str().unwrap_or("");
                            let event_end_str = event["endDate"].as_str().unwrap_or("");
                            
                            if let Some(markets) = event["markets"].as_array() {
                                for market in markets {
                                    // Check if market is closed
                                    let is_closed = market["closed"].as_bool().unwrap_or(true);
                                    if is_closed {
                                        skipped_closed += 1;
                                        continue;
                                    }
                                    
                                    // Market-level dates (fallback to event-level)
                                    let start_str = market["startDate"].as_str()
                                        .unwrap_or(event_start_str);
                                    let end_str = market["endDate"].as_str()
                                        .unwrap_or(event_end_str);
                                    
                                    // Parse dates (fallback to None if parsing fails)
                                    let start_time = parse_iso_date(start_str);
                                    let end_time = parse_iso_date(end_str);
                                    
                                    // Skip if end date has passed
                                    if let Some(end) = end_time {
                                        if end <= now {
                                            skipped_expired += 1;
                                            continue;
                                        }
                                    } else {
                                        // No valid end date - skip
                                        skipped_dates += 1;
                                        continue;
                                    }
                                    
                                    // Check if market hasn't started yet (more than 2 minutes away)
                                    if let Some(start) = start_time {
                                        let time_until_start = (start - now).num_seconds();
                                        if time_until_start > 120 {
                                            // Market starts more than 2 minutes from now
                                            skipped_future += 1;
                                            continue;
                                        }
                                    }
                                    
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
                                    
                                    // Parse clobTokenIds (JSON string)
                                    if let Some(clob_ids_str) = market["clobTokenIds"].as_str() {
                                        if let Ok(clob_ids) = serde_json::from_str::<Vec<String>>(clob_ids_str) {
                                            if clob_ids.len() == 2 {
                                                let yes_str = clob_ids[0].clone();
                                                let no_str = clob_ids[1].clone();
                                                
                                                if yes_str.is_empty() || no_str.is_empty() {
                                                    continue;
                                                }
                                                
                                                let yes_hash = hash_token(&yes_str);
                                                let no_hash = hash_token(&no_str);

                                                hash_to_id.insert(yes_hash, yes_str.clone());
                                                hash_to_id.insert(no_hash, no_str.clone());
                                                id_to_condition.insert(yes_str.clone(), condition_id.clone());
                                                id_to_condition.insert(no_str.clone(), condition_id.clone());
                                                complement_map.insert(yes_hash, no_hash);
                                                complement_map.insert(no_hash, yes_hash);
                                                
                                                market_count += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        println!("✅ ACTIVE markets: {} (closed: {}, expired: {}, future: {}, no-date: {})", 
                            market_count, skipped_closed, skipped_expired, skipped_future, skipped_dates);
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

// Legacy exports
pub const MARKET_SLUGS: &[&str] = &[];

pub async fn build_condition_map(
    client: &Client,
    _slugs: &[&str],
) -> (HashMap<u64, String>, HashMap<String, String>) {
    let (hash_to_id, id_to_condition, _) = build_maps(client).await;
    (hash_to_id, id_to_condition)
}