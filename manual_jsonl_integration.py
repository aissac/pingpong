#!/usr/bin/env python3
"""Manual JSONL logger integration - matches actual code structure"""

# 1. Add import to hft_hot_path.rs
with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Add import after existing use statements
if 'use crate::jsonl_logger' not in content:
    content = content.replace(
        'use crate::websocket_reader::WebSocketReader;',
        'use crate::websocket_reader::WebSocketReader;\nuse crate::jsonl_logger::LogEvent;'
    )

# 2. Add log_tx field to HotPath struct - find eval_tracker and add after it
if 'pub log_tx:' not in content:
    content = content.replace(
        'pub eval_tracker: EvalTracker,',
        'pub eval_tracker: EvalTracker,\n    pub log_tx: crossbeam_channel::Sender<LogEvent>,'
    )

# 3. Replace METRICS println with LogEvent
old_metrics = '''println!("[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {} | pairs:{}",
                secs,
                total_messages,
                total_evals,
                total_edges,
                total_evals / secs,
                token_pairs.len()
            );'''

new_metrics = '''let _ = log_tx.try_send(LogEvent::Metric {
                uptime: secs,
                msgs: total_messages,
                evals: total_evals,
                edges: total_edges,
                evals_sec: total_evals / secs,
                pairs: token_pairs.len() as u64,
                dropped: 0,
            });'''

content = content.replace(old_metrics, new_metrics)

# 4. Replace EDGE CHECK println with LogEvent
old_edge = '''println!("[EDGE CHECK] YES price={} size={} | NO price={} size={}", 
                                yes_ask_price, yes_ask_size, no_ask_price, no_ask_size);
                            
                            if yes_ask_price == 0 || yes_ask_price >= 100_000_000 {
                                println!("  FAIL: YES price out of bounds");
                            } else if no_ask_price == 0 || no_ask_price >= 100_000_000 {
                                println!("  FAIL: NO price out of bounds");
                            } else if yes_ask_size < TARGET_SHARES {
                                println!("  FAIL: YES size {} < {}", yes_ask_size, TARGET_SHARES);
                            } else if no_ask_size < TARGET_SHARES {
                                println!("  FAIL: NO size {} < {}", no_ask_size, TARGET_SHARES);
                            } else {
                                let combined = yes_ask_price + no_ask_price;
                                println!("  Combined: {} (threshold {}-{})", 
                                    combined, MIN_VALID_COMBINED_U64, EDGE_THRESHOLD_U64);
                                if combined < MIN_VALID_COMBINED_U64 {
                                    println!("  FAIL: Combined too low");
                                } else if combined > EDGE_THRESHOLD_U64 {
                                    println!("  FAIL: Combined too high");
                                } else {
                                    println!("  PASS: Edge detected!");
                                }
                            }'''

new_edge = '''let combined = yes_ask_price + no_ask_price;
                            let is_dust = combined < MIN_VALID_COMBINED_U64;
                            let _ = log_tx.try_send(LogEvent::EdgeCheck {
                                yes_hash: yes_hash,
                                no_hash: no_hash,
                                yes_ask: yes_ask_price,
                                no_ask: no_ask_price,
                                combined: combined,
                                is_dust: is_dust,
                            });'''

content = content.replace(old_edge, new_edge)

# 5. Replace EDGE FOUND println
old_found = '''println!("EDGE FOUND! Combined ask: {} (threshold {}-{})", combined_ask, MIN_VALID_COMBINED_U64, EDGE_THRESHOLD_U64);'''
new_found = '''let _ = log_tx.try_send(LogEvent::EdgeFound {
                yes_hash: yes_hash,
                no_hash: no_hash,
                yes_ask: yes_ask_price,
                no_ask: no_ask_price,
                combined: combined_ask,
                profit_usd: (1_000_000 - combined_ask) as u64,
            });'''

content = content.replace(old_found, new_found)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print("✅ Updated hft_hot_path.rs")

# 6. Update bin entry point
with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'r') as f:
    content = f.read()

# Add import
if 'use pingpong::jsonl_logger' not in content:
    content = content.replace(
        'use pingpong::hft_hot_path::{HotPath, EvalTracker};',
        'use pingpong::hft_hot_path::{HotPath, EvalTracker};\nuse pingpong::jsonl_logger::{JsonlLogger, LogEvent};'
    )

# Add logger startup - find rollover channel and add logger after
old_rollover = 'let (rollover_tx, rollover_rx) = crossbeam_channel::bounded(64);'
new_rollover = '''let (rollover_tx, rollover_rx) = crossbeam_channel::bounded(64);
    
    // Start JSONL logger thread
    let (log_tx, log_rx) = crossbeam_channel::bounded(4096);
    let _logger_handle = JsonlLogger::start(log_rx);'''

content = content.replace(old_rollover, new_rollover)

# Pass log_tx to HotPath
if 'log_tx' not in content:
    content = content.replace(
        'log_tx: background_tx.clone(),',
        'log_tx: log_tx.clone(),'
    )

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'w') as f:
    f.write(content)

print("✅ Updated hft_pingpong_v2.rs")
