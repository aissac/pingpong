#!/usr/bin/env python3
"""Add parser counter debug"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Add counters to parser
old_func = '''fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut pos = 0;'''

new_func = '''fn parse_and_update_orderbook(
    bytes: &[u8],
    orderbook: &mut FxHashMap<u64, TokenBookState>,
    token_pairs: &HashMap<u64, u64>,
) {
    let mut current_token_hash: Option<u64> = None;
    let mut is_bid = false;
    let mut pos = 0;
    let mut found_asset = 0u64;
    let mut found_price = 0u64;'''

content = content.replace(old_func, new_func)

# Count assets
old_asset = '''        if remaining.starts_with(asset_marker) {
            let token_start = pos + asset_marker.len();
            if let Some(token_end) = memchr(b'"', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }'''

new_asset = '''        if remaining.starts_with(asset_marker) {
            let token_start = pos + asset_marker.len();
            if let Some(token_end) = memchr(b'"', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                found_asset += 1;
                pos = token_start + token_end + 1;
                continue;
            }
        }'''

content = content.replace(old_asset, new_asset)

# Count prices
old_price = '''        if remaining.starts_with(price_marker) {
            let price_start = pos + price_marker.len();
            if let Some(price_end) = memchr(b'"', &bytes[price_start..]) {
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
        }'''

new_price = '''        if remaining.starts_with(price_marker) {
            let price_start = pos + price_marker.len();
            if let Some(price_end) = memchr(b'"', &bytes[price_start..]) {
                let price = parse_fixed_6(&bytes[price_start..price_start + price_end]);
                found_price += 1;
                
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
        }'''

content = content.replace(old_price, new_price)

# Add summary at end
old_end = '''        pos += 1;
    }
}'''

new_end = '''        pos += 1;
    }
    
    println!("[PARSE] assets={} prices={}", found_asset, found_price);
}'''

content = content.replace(old_end, new_end)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Added parser counters')
