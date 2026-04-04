#!/usr/bin/env python3
"""Fix parser to handle actual Polymarket WebSocket format"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Replace the entire parse_and_update_orderbook function
old_func = '''/// Parse WebSocket message and update orderbook
fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut in_array = false;
    let mut pos = 0;

    while pos < bytes.len() {
        let remaining = &bytes[pos..];

        // Find asset_id
        if remaining.starts_with(b"\\\"asset_id\\\":\\\"") && !in_array {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'\\'', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }

        // Find bids array
        if remaining.starts_with(b"\\\"bids\\":[") {
            is_bid = true;
            in_array = true;
            pos += 9;
            continue;
        }

        // Find asks array
        if remaining.starts_with(b"\\\"asks\\":[") {
            is_bid = false;
            in_array = true;
            pos += 9;
            continue;
        }

        // End of array
        if remaining.starts_with(b"]") && in_array {
            in_array = false;
            pos += 1;
            continue;
        }

        // Parse price in array
        if in_array && remaining.starts_with(b"\\\"price\\\":\\\"") {
            let price_start = pos + 9;
            if let Some(price_end) = memchr(b'\\'', &bytes[price_start..]) {
                let price = parse_fixed_6(&bytes[price_start..price_start + price_end]);
                
                if let Some(token_hash) = current_token_hash {
                    if let Some(state) = orderbook.get_mut(&token_hash) {
                        if is_bid {
                            state.update_bid(price, 100);
                        } else {
                            state.update_ask(price, 100);
                        }
                    }
                }
                pos = price_start + price_end + 1;
                continue;
            }
        }

        pos += 1;
    }
}'''

new_func = r'''/// Parse WebSocket message and update orderbook
fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut pos = 0;

    while pos < bytes.len() {
        let remaining = &bytes[pos..];

        // Find asset_id (works for both top-level and inside price_changes)
        if remaining.starts_with(b"\"asset_id\":\"") {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'"', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }

        // Find bids array start
        if remaining.starts_with(b"\"bids\":[") {
            is_bid = true;
            pos += 8;
            continue;
        }

        // Find asks array start
        if remaining.starts_with(b"\"asks\":[") {
            is_bid = false;
            pos += 8;
            continue;
        }

        // Parse price inside bid/ask objects: {"price":"0.5","size":"100"}
        if remaining.starts_with(b"\"price\":\"") {
            let price_start = pos + 9;
            if let Some(price_end) = memchr(b'"', &bytes[price_start..]) {
                let price = parse_fixed_6(&bytes[price_start..price_start + price_end]);
                
                if let Some(token_hash) = current_token_hash {
                    if let Some(state) = orderbook.get_mut(&token_hash) {
                        // Use size=100 as placeholder (real size parsing can be added)
                        if is_bid {
                            state.update_bid(price, 100);
                        } else {
                            state.update_ask(price, 100);
                        }
                    }
                }
                pos = price_start + price_end + 1;
                continue;
            }
        }

        pos += 1;
    }
}'''

content = content.replace(old_func, new_func)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Fixed parser for actual WebSocket format')
