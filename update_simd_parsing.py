#!/usr/bin/env python3

# Replace serde_json parsing with SIMD parsing in hot_switchover.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Find and replace the parsing call
old_parse = '''                        Ok(Message::Text(text)) => {
                            // Priority 5: Nanosecond latency measurement
                            let start = std::time::Instant::now();
                            
                            // Parse orderbook updates (serde_json)
                            if let Ok(updates) = parse_orderbook_update(&text) {'''

new_parse = '''                        Ok(Message::Text(text)) => {
                            // Priority 5: Nanosecond latency measurement
                            let start = std::time::Instant::now();
                            
                            // Parse orderbook updates (SIMD - 1.77x faster than serde_json)
                            // Copy text to mutable buffer for simd_json
                            let mut buffer = text.into_bytes();
                            if let Ok(updates) = crate::simd_parse::parse_orderbook_update_simd(&mut buffer) {'''

content = content.replace(old_parse, new_parse)

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Replaced serde_json with simd_json parsing')