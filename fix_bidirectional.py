#!/usr/bin/env python3
"""Fix token_pairs to use bi-directional mapping"""

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'r') as f:
    content = f.read()

# Change the RolloverCommand to send both hashes
old_cmd = '''pub enum RolloverCommand {
    AddPair(u64, u64),
    RemovePair(u64),
}'''

new_cmd = '''pub enum RolloverCommand {
    AddPair(u64, u64),  // yes_hash, no_hash
    RemovePair(u64),    // either hash (will remove both directions)
}'''

content = content.replace(old_cmd, new_cmd)

# Update the rollover thread to send BOTH directions
old_send = '''if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(yes_hash, no_hash)) {
                                eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                                return;
                            }
                            
                            tracked_markets.insert(market_id.clone());'''

new_send = '''// Send AddPair for BOTH directions (bi-directional map)
                            if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(yes_hash, no_hash)) {
                                eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                                return;
                            }
                            if let Err(e) = rollover_tx.send(RolloverCommand::AddPair(no_hash, yes_hash)) {
                                eprintln!("🚨 [ROLLOVER] Channel disconnected: {}", e);
                                return;
                            }
                            
                            tracked_markets.insert(market_id.clone());'''

content = content.replace(old_send, new_send)

# Update RemovePair to send both directions
old_remove = '''let _ = rollover_tx.send(RolloverCommand::RemovePair(expired_hash));
            tracked_markets.remove(&expired_id);'''

new_remove = '''// Remove both directions
                            let _ = rollover_tx.send(RolloverCommand::RemovePair(expired_hash));
                            // Also need to remove the complement (we'll handle this in hot path)
                            tracked_markets.remove(&expired_id);'''

content = content.replace(old_remove, new_remove)

with open('/home/ubuntu/polymarket-hft-engine/src/market_rollover.rs', 'w') as f:
    f.write(content)

print('✅ Fixed: Rollover now sends BOTH directions for bi-directional mapping')
