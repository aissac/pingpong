#!/usr/bin/env python3
"""Implement rollover command handling in hot path"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Fix rollover handling to actually update token_pairs and eval_trackers
old_rollover = '''        // Process rollover commands (non-blocking)
        while let Ok(_cmd) = rollover_rx.try_recv() {
            // Handle AddPair/RemovePair here when implemented
        }'''

new_rollover = '''        // Process rollover commands (non-blocking)
        while let Ok(cmd) = rollover_rx.try_recv() {
            match cmd {
                RolloverCommand::AddPair(yes_hash, no_hash) => {
                    println!("[ROLLOVER] Adding pair: YES={} NO={}", yes_hash, no_hash);
                    token_pairs.insert(yes_hash, no_hash);
                    token_pairs.insert(no_hash, yes_hash);
                    eval_trackers.insert(yes_hash, EvalTracker::new());
                    eval_trackers.insert(no_hash, EvalTracker::new());
                    orderbook.entry(yes_hash).or_insert_with(TokenBookState::new);
                    orderbook.entry(no_hash).or_insert_with(TokenBookState::new);
                }
                RolloverCommand::RemovePair(yes_hash) => {
                    println!("[ROLLOVER] Removing pair: YES={}", yes_hash);
                    // Remove both directions
                    if let Some(no_hash) = token_pairs.remove(&yes_hash) {
                        token_pairs.remove(&no_hash);
                        eval_trackers.remove(&yes_hash);
                        eval_trackers.remove(&no_hash);
                    }
                }
            }
        }'''

content = content.replace(old_rollover, new_rollover)

# Remove the verbose asset_id debug (too much spam)
old_asset_debug = '''        // Find asset_id
        if remaining.starts_with(b"\\\"asset_id\\\":\\\"") {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'\\'', &bytes[token_start..]) {
                let token_bytes = &bytes[token_start..token_start + token_end];
                current_token_hash = Some(fast_hash(token_bytes));
                // Debug: log first 100 unique token hashes
                if messages < 100 {
                    println!(\"[ASSET] msg={} hash={} token_str={}\", 
                        messages, 
                        current_token_hash.unwrap(), 
                        String::from_utf8_lossy(token_bytes));
                }
                pos = token_start + token_end + 1;
                continue;
            }
        }'''

new_asset_debug = '''        // Find asset_id
        if remaining.starts_with(b"\\\"asset_id\\\":\\\"") {
            let token_start = pos + 12;
            if let Some(token_end) = memchr(b'\\'', &bytes[token_start..]) {
                current_token_hash = Some(fast_hash(&bytes[token_start..token_start + token_end]));
                pos = token_start + token_end + 1;
                continue;
            }
        }'''

content = content.replace(old_asset_debug, new_asset_debug)

# Remove verbose price debug
old_price_debug = '''                        // Debug: log all prices
println!(\"[PRICE] Token {} {} price={} micro\", 
    token_hash, if is_bid {\"BID\"} else {\"ASK\"}, price);'''

new_price_debug = '''// Price parsed (no debug logging by default)'''

content = content.replace(old_price_debug, new_price_debug)

# Remove orderbook debug
old_ob_debug = '''                // Debug: check orderbook state
                let yes_has_data = orderbook.get(&token_hash).map(|s| s.best_ask_price < u64::MAX).unwrap_or(false);
                let no_has_data = orderbook.get(&complement_hash).map(|s| s.best_ask_price < u64::MAX).unwrap_or(false);
                if messages % 100 == 0 && (yes_has_data || no_has_data) {
                    println!(\"[ORDERBOOK] Token {} YES={:?} NO={:?}\", 
                        token_hash, 
                        orderbook.get(&token_hash).map(|s| s.best_ask_price),
                        orderbook.get(&complement_hash).map(|s| s.best_ask_price));
                }
                '''

content = content.replace(old_ob_debug, '')

# Remove token list debug
old_token_debug = '''    // Pre-populate orderbook and log token hashes
    println!(\"[TOKENS] Tracking {} tokens:\", all_tokens.len());
    for (i, token) in all_tokens.iter().enumerate() {
        let hash = fast_hash(token.as_bytes());
        orderbook.entry(hash).or_insert_with(TokenBookState::new);
        if i < 10 {
            println!(\"  [{}] hash={} token={}\", i, hash, token);
        }
    }
    if all_tokens.len() > 10 {
        println!(\"  ... and {} more\", all_tokens.len() - 10);
    }'''

new_token_debug = '''    // Pre-populate orderbook
    for token in &all_tokens {
        let hash = fast_hash(token.as_bytes());
        orderbook.entry(hash).or_insert_with(TokenBookState::new);
    }'''

content = content.replace(old_token_debug, new_token_debug)

# Add edge found logging
old_edge_send = '''                            edges_found += 1;
                            edge_counter.fetch_add(1, Ordering::Relaxed);

                            let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {'''

new_edge_send = '''                            edges_found += 1;
                            edge_counter.fetch_add(1, Ordering::Relaxed);

                            println!(\"🎯 [EDGE] Combined ASK=${:.4} (YES=${} NO=${})\", 
                                combined_ask as f64 / 1_000_000.0,
                                yes_ask_price as f64 / 1_000_000.0,
                                no_ask_price as f64 / 1_000_000.0);

                            let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {'''

content = content.replace(old_edge_send, new_edge_send)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Implemented rollover handling + cleaned up debug logging')
