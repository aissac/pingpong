#!/usr/bin/env python3
"""Add debug to rollover and fix filtering"""

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'r') as f:
    content = f.read()

# Add debug to fetch_active_markets
old_func = '''pub async fn fetch_active_markets(client: &reqwest::Client) -> Vec<MarketPeriod> {
    let url = "https://gamma-api.polymarket.com/events?active=true&closed=false&tag_slug=crypto";
    
    match client.get(url).send().await {'''

new_func = '''pub async fn fetch_active_markets(client: &reqwest::Client) -> Vec<MarketPeriod> {
    let url = "https://gamma-api.polymarket.com/events?active=true&closed=false&tag_slug=crypto";
    
    println!("[ROLLOVER] Fetching: {}", url);
    
    match client.get(url).send().await {'''

content = content.replace(old_func, new_func)

# Add more debug after response
old_match = '''        Ok(response) => {
            match response.json::<Value>().await {'''

new_match = '''        Ok(response) => {
            let status = response.status();
            println!("[ROLLOVER] API status: {}", status);
            match response.json::<Value>().await {'''

content = content.replace(old_match, new_match)

# Fix the filtering - be more lenient
old_filter = '''                            // Filter: btc-*, eth-*, *-up-or-down-*, 5m or 15m
                            if !slug.contains("-up-or-down-") {
                                continue;
                            }
                            if !(slug.starts_with("btc-") || slug.starts_with("eth-")) {
                                continue;
                            }
                            if !(slug.contains("-5m-") || slug.contains("-15m-")) {
                                continue;
                            }'''

new_filter = '''                            // Filter: btc-*, eth-*, *-up-or-down-*, 5m or 15m
                            println!("[ROLLOVER] Checking slug: {}", slug);
                            if !slug.contains("-up-or-down-") {
                                println!("[ROLLOVER] Skipping - not up-or-down");
                                continue;
                            }
                            if !(slug.starts_with("btc-") || slug.starts_with("eth-")) {
                                println!("[ROLLOVER] Skipping - not btc/eth");
                                continue;
                            }
                            if !(slug.contains("-5m-") || slug.contains("-15m-")) {
                                println!("[ROLLOVER] Skipping - not 5m/15m");
                                continue;
                            }
                            println!("[ROLLOVER] MATCHED: {}", slug);'''

content = content.replace(old_filter, new_filter)

# Add debug when no events found
old_empty = '''                    periods
                }
                Err(e) => {
                    eprintln!("[ROLLOVER] JSON parse error: {}", e);
                    Vec::new()
                }'''

new_empty = '''                    println!("[ROLLOVER] Found {} periods", periods.len());
                    periods
                }
                Err(e) => {
                    eprintln!("[ROLLOVER] JSON parse error: {}", e);
                    Vec::new()
                }'''

content = content.replace(old_empty, new_empty)

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'w') as f:
    f.write(content)

print('Added debug logging')
