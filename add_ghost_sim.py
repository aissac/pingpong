import re

with open('/home/ubuntu/polymarket-hft-engine/src/main.rs', 'r') as f:
    content = f.read()

# Find the location after pnl_tracker.record_fill_attempt();
old_code = '''                        // Send to trading engine for real execution
                        pnl_tracker.record_arb_opportunity();
                        pnl_tracker.record_fill_attempt();'''

new_code = '''                        // Send to trading engine for real execution
                        pnl_tracker.record_arb_opportunity();
                        pnl_tracker.record_fill_attempt();
                        
                        // GHOST SIMULATION: Track if liquidity vanishes after 50ms RTT
                        // Mark current depth as baseline
                        orderbook_tracker.mark_queue_start(&condition_id);
                        
                        // Store opportunity details for ghost check
                        let ghost_condition = condition_id.clone();
                        let ghost_yes_depth = yes_depth;
                        let ghost_no_depth = no_depth;
                        let ghost_side = if is_extreme_prob { maker_side.to_string() } else { "BOTH".to_string() };
                        let ghost_price = combined;
                        
                        // Spawn async task to check after 50ms RTT
                        let tracker_clone = orderbook_tracker.clone();
                        tokio::spawn(async move {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            
                            // Check if depth still exists
                            if let Some(prices) = tracker_clone.get(&ghost_condition) {
                                let yes_ok = prices.yes_depth >= ghost_yes_depth * 0.5;
                                let no_ok = prices.no_depth >= ghost_no_depth * 0.5;
                                
                                if !yes_ok && !no_ok {
                                    tracing::info!(
                                        "GHOST SIMULATION: {} | {} @ ${:.4} | Depth vanished: YES {:.0}->{:.0}, NO {:.0}->{:.0}",
                                        &ghost_condition[..8.min(ghost_condition.len())],
                                        ghost_side,
                                        ghost_price,
                                        ghost_yes_depth, prices.yes_depth,
                                        ghost_no_depth, prices.no_depth
                                    );
                                } else {
                                    tracing::info!(
                                        "EXECUTABLE SIMULATION: {} | {} @ ${:.4} | Depth OK: YES {:.0}->{:.0}, NO {:.0}->{:.0}",
                                        &ghost_condition[..8.min(ghost_condition.len())],
                                        ghost_side,
                                        ghost_price,
                                        ghost_yes_depth, prices.yes_depth,
                                        ghost_no_depth, prices.no_depth
                                    );
                                }
                            }
                        });'''

content = content.replace(old_code, new_code)

with open('/home/ubuntu/polymarket-hft-engine/src/main.rs', 'w') as f:
    f.write(content)

print('Added ghost simulation')