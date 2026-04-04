#!/usr/bin/env python3

# Read the file
with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Find the exact pattern and add ghost checking after it
old_pattern = '''                state.tracker.update(&market_id, Some(update.price), Some(update.price), yes_depth, no_depth);
                
                // Forward to main loop for processing'''

new_pattern = '''                state.tracker.update(&market_id, Some(update.price), Some(update.price), yes_depth, no_depth);
                
                // GHOST SIMULATION: Check pending signals against new depth
                state.ghost_simulator.check_ghosts(&market_id, "YES", yes_depth);
                state.ghost_simulator.check_ghosts(&market_id, "NO", no_depth);
                
                // Forward to main loop for processing'''

content = content.replace(old_pattern, new_pattern)

# Write back
with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Added ghost checking to orderbook updates v2')