#!/usr/bin/env python3

# Read the file
with open('/home/ubuntu/polymarket-hft-engine/src/main.rs', 'r') as f:
    content = f.read()

# Find the MAKER HYBRID log and add ghost tracking after it
old_log = '''                        if is_extreme_prob {
                            // Log Maker hybrid opportunity
                            info!(
                                "🎯 MAKER HYBRID! | {} | {} @ ${:.4} (Maker) + {} @ ${:.4} (Taker) = ${:.4} | Edge: {:.1}% | Fee: {:.2}% | Net: ${:.4}/sh",
                                &condition_id[..8.min(condition_id.len())],
                                maker_side,
                                maker_price,
                                if maker_side == "YES" { "NO" } else { "YES" },
                                taker_price,
                                combined,
                                edge * 100.0,
                                dynamic_fee * 100.0,
                                profit_with_rebate
                            );
                        } else {'''

new_log = '''                        if is_extreme_prob {
                            // Log Maker hybrid opportunity
                            info!(
                                "🎯 MAKER HYBRID! | {} | {} @ ${:.4} (Maker) + {} @ ${:.4} (Taker) = ${:.4} | Edge: {:.1}% | Fee: {:.2}% | Net: ${:.4}/sh",
                                &condition_id[..8.min(condition_id.len())],
                                maker_side,
                                maker_price,
                                if maker_side == "YES" { "NO" } else { "YES" },
                                taker_price,
                                combined,
                                edge * 100.0,
                                dynamic_fee * 100.0,
                                profit_with_rebate
                            );
                            
                            // GHOST SIMULATION: Track this signal for ghost detection
                            // Simulates what would happen if we placed orders
                            state.ghost_simulator.track_signal(
                                &condition_id,
                                maker_side,
                                maker_price,
                                if maker_side == "YES" { yes_depth } else { no_depth },
                                Some(shares_f),
                            );
                        } else {'''

content = content.replace(old_log, new_log)

# Write back
with open('/home/ubuntu/polymarket-hft-engine/src/main.rs', 'w') as f:
    f.write(content)

print('Added ghost tracking to MAKER HYBRID detection')
