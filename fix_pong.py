# Add pong message filtering to hot_switchover.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Find the message handler and add pong filter
old_handler = '''                        Ok(Message::Text(text)) => {
                            if let Ok(updates) = parse_orderbook_update(&text) {'''

new_handler = '''                        Ok(Message::Text(text)) => {
                            // CRITICAL: Filter out pong messages before parsing
                            // simd-json panics on non-JSON messages
                            if text.contains("pong") || text == "{}" || text.is_empty() {
                                continue;
                            }
                            
                            if let Ok(updates) = parse_orderbook_update(&text) {'''

content = content.replace(old_handler, new_handler)

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Added pong message filter')