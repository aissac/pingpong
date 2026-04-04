#!/usr/bin/env python3
"""Fix parser to handle price_changes with side field"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

old_func = '''fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
    messages: u64,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut in_array = false;
    let mut pos = 0;

    while pos < bytes.len() {
        let remaining = &bytes[pos..];

        // Find asset_id
        if remaining.starts_with(b"\\"asset_id\\":\\"") && !in_array {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'\\'', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }

        // Find bids array
        if remaining.starts_with(b"\\"bids\\":[") {
            is_bid = true;
            in_array = true;
            pos += 9;
            continue;
        }

        // Find asks array
        if remaining.starts_with(b"\\"asks\\":[") {
            is_bid = false;
            in_array = true;
            pos += 9;
            continue;
        }'''

new_func = '''fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
    messages: u64,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut pos = 0;

    while pos < bytes.len() {
        let remaining = &bytes[pos..];

        // Find asset_id (works in price_changes array)
        if remaining.starts_with(b"\\"asset_id\\":\\"") {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'\\'', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }

        // Check for side field to determine bid/ask
        if remaining.starts_with(b"\\"side\\":\\"BUY\\"") {
            is_bid = true;
        } else if remaining.starts_with(b"\\"side\\":\\"SELL\\"") {
            is_bid = false;
        }'''

content = content.replace(old_func, new_func)

# Remove in_array logic since we don't need it
content = content.replace('let mut in_array = false;', '// removed in_array')
content = content.replace('in_array = true;', '// was in_array = true')
content = content.replace('in_array = false;', '// was in_array = false')
content = content.replace('&& !in_array', '')
content = content.replace('&& in_array', '')

# Remove the bids/asks array detection lines that are now orphaned
content = content.replace('''        // Find bids array
        if remaining.starts_with(b"\\"bids\\":[") {
            is_bid = true;
            // was in_array = true
            pos += 9;
            continue;
        }

        // Find asks array
        if remaining.starts_with(b"\\"asks\\":[") {
            is_bid = false;
            // was in_array = true
            pos += 9;
            continue;
        }''', '')

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Fixed parser for price_changes format')
