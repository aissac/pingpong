#!/usr/bin/env python3
"""Integrate JSONL logger into hft_hot_path.rs"""

import re

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# 1. Add module import at top
if 'use crate::jsonl_logger' not in content:
    content = content.replace(
        'use crate::fees::calculate_hybrid_fee;',
        'use crate::fees::calculate_hybrid_fee;\nuse crate::jsonl_logger::{JsonlLogger, LogEvent};'
    )

# 2. Add logger field to HotPath struct
if 'log_tx:' not in content:
    content = content.replace(
        'pub eval_tracker: EvalTracker,',
        'pub eval_tracker: EvalTracker,\n    pub log_tx: crossbeam_channel::Sender<LogEvent>,'
    )

# 3. Replace println! metrics with LogEvent::Metric
old_metrics_print = '''// Periodic metrics (every 30 seconds)
        if total_evals % 7200 == 0 {
            let elapsed = start_time.elapsed();
            let secs = elapsed.as_secs().max(1);
            println!(
                "[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {} | pairs:{}",
                secs,
                total_messages,
                total_evals,
                total_edges,
                total_evals / secs,
                token_pairs.len()
            );
        }'''

new_metrics_print = '''// Periodic metrics (every 1 second via heartbeat)
        if total_evals % 300 == 0 {
            let elapsed = start_time.elapsed();
            let secs = elapsed.as_secs().max(1);
            let _ = log_tx.try_send(LogEvent::Metric {
                uptime: secs,
                msgs: total_messages,
                evals: total_evals,
                edges: total_edges,
                evals_sec: total_evals / secs,
                pairs: token_pairs.len() as u64,
                dropped: 0,
            });
        }'''

content = content.replace(old_metrics_print, new_metrics_print)

# 4. Replace edge check println! with LogEvent::EdgeCheck
old_edge_check = '''// Debug: show why edge detection fails
                        if total_evals % 50 == 0 {
                            let combined = yes_ask_price + no_ask_price;
                            println!("[EDGE CHECK] YES price={} size={} | NO price={} size={} | Combined=${:.4}", 
                                yes_ask_price, yes_ask_size, no_ask_price, no_ask_size,
                                combined as f64 / 1e6);
                            
                            if yes_ask_price == 0 || yes_ask_price >= 100_000_000 {
                                println!("  FAIL: YES price out of bounds");
                            } else if no_ask_price == 0 || no_ask_price >= 100_000_000 {
                                println!("  FAIL: NO price out of bounds");
                            } else if yes_ask_size < TARGET_SHARES {
                                println!("  FAIL: YES size {} < {}", yes_ask_size, TARGET_SHARES);
                            } else if no_ask_size < TARGET_SHARES {
                                println!("  FAIL: NO size {} < {}", no_ask_size, TARGET_SHARES);
                            } else if combined < MIN_VALID_COMBINED_U64 {
                                println!("  FAIL: Combined too low (DUST QUOTES)");
                            } else if combined > EDGE_THRESHOLD_U64 {
                                println!("  FAIL: Combined too high (wide spread)");
                            } else {
                                println!("  PASS: Edge detected!");
                            }
                        }'''

new_edge_check = '''// Edge check logging (every 50th evaluation)
                        if total_evals % 50 == 0 {
                            let combined = yes_ask_price + no_ask_price;
                            let is_dust = combined < MIN_VALID_COMBINED_U64;
                            let _ = log_tx.try_send(LogEvent::EdgeCheck {
                                yes_hash: yes_hash,
                                no_hash: no_hash,
                                yes_ask: yes_ask_price,
                                no_ask: no_ask_price,
                                combined: combined,
                                is_dust: is_dust,
                            });
                        }'''

content = content.replace(old_edge_check, new_edge_check)

# 5. Replace edge found println! with LogEvent::EdgeFound
old_edge_found = '''println!("EDGE FOUND! Combined ask: {} (threshold {}-{})", combined_ask, MIN_VALID_COMBINED_U64, EDGE_THRESHOLD_U64);'''
new_edge_found = '''let _ = log_tx.try_send(LogEvent::EdgeFound {
    yes_hash: yes_hash,
    no_hash: no_hash,
    yes_ask: yes_ask_price,
    no_ask: no_ask_price,
    combined: combined_ask,
    profit_usd: (1_000_000 - combined_ask) as u64,
});'''

content = content.replace(old_edge_found, new_edge_found)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print("✅ Integrated JSONL logger into hft_hot_path.rs")
