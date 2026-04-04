#!/usr/bin/env python3
"""Fix hot path for bi-directional mapping"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Change token_pairs type from HashMap<u64, u64> to HashMap<u64, (u64, u64)>
old_type = '    mut token_pairs: HashMap<u64, u64>,'
new_type = '    mut token_pairs: HashMap<u64, (u64, u64)>,'  # (yes_hash, no_hash)

content = content.replace(old_type, new_type)

# Fix the iteration to use tuple
old_iter = '            for (token_hash, complement_hash) in pairs {'
new_iter = '            for (token_hash, &(yes_hash, no_hash)) in pairs.iter() {'

content = content.replace(old_iter, new_iter)

# Fix edge detection to use yes_hash and no_hash
old_edge = '''                if let (Some(yes_state), Some(no_state)) = 
                    (orderbook.get(&token_hash), orderbook.get(&complement_hash)) {
                    
                    if let (Some((yes_ask_price, yes_ask_size)), 
                            Some((no_ask_price, no_ask_size))) = 
                        (yes_state.get_best_ask(), no_state.get_best_ask()) {'''

new_edge = '''                // Use the bi-directional mapping
                if let (Some(yes_state), Some(no_state)) = 
                    (orderbook.get(&yes_hash), orderbook.get(&no_hash)) {
                    
                    if let (Some((yes_ask_price, yes_ask_size)), 
                            Some((no_ask_price, no_ask_size))) = 
                        (yes_state.get_best_ask(), no_state.get_best_ask()) {'''

content = content.replace(old_edge, new_edge)

# Fix the edge send to use correct hashes
old_send = '''                            let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {
                                yes_token_hash: token_hash,
                                no_token_hash: complement_hash,'''

new_send = '''                            let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {
                                yes_token_hash: yes_hash,
                                no_token_hash: no_hash,'''

content = content.replace(old_send, new_send)

# Fix AddPair handling in rollover loop
old_add = '''                RolloverCommand::AddPair(yes_hash, no_hash) => {
                    println!("[ROLLOVER] Adding pair: YES={} NO={}", yes_hash, no_hash);
                    token_pairs.insert(yes_hash, no_hash);
                    token_pairs.insert(no_hash, yes_hash);'''

new_add = '''                RolloverCommand::AddPair(first_hash, second_hash) => {
                    println!("[ROLLOVER] Adding mapping: {} -> {}", first_hash, second_hash);
                    // Store as (yes, no) tuple for both directions
                    token_pairs.insert(first_hash, (first_hash, second_hash));'''

content = content.replace(old_add, new_add)

# Fix RemovePair handling
old_rem = '''                RolloverCommand::RemovePair(yes_hash) => {
                    println!("[ROLLOVER] Removing pair: YES={}", yes_hash);
                    if let Some(no_hash) = token_pairs.remove(&yes_hash) {
                        token_pairs.remove(&no_hash);'''

new_rem = '''                RolloverCommand::RemovePair(hash) => {
                    println!("[ROLLOVER] Removing mapping: {}", hash);
                    token_pairs.remove(&hash);'''

content = content.replace(old_rem, new_rem)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Fixed hot path for bi-directional mapping')
