#!/usr/bin/env python3
"""Add detailed failure reason with USD formatting"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Update the debug to show the combined price and why it fails
old_debug = '''                        // Debug: show why edge detection fails
                        if total_evals % 50 == 0 {
                            let combined = yes_ask_price + no_ask_price;
                            println!("[EDGE CHECK] YES=${{:.4}} size={} | NO=${{:.4}} size={} | Combined=${{:.4}}", 
                                yes_ask_price as f64 / 1e6, yes_ask_size,
                                no_ask_price as f64 / 1e6, no_ask_size,
                                combined as f64 / 1e6);
                            
                            if yes_ask_price == 0 || yes_ask_price >= 100_000_000 {
                                println!("  FAIL: YES price out of bounds");
                            } else if no_ask_price == 0 || no_ask_price >= 100_000_000 {
                                println!("  FAIL: NO price out of bounds");
                            } else if yes_ask_size < TARGET_SHARES {
                                println!("  FAIL: YES size {{}} < {{}}", yes_ask_size, TARGET_SHARES);
                            } else if no_ask_size < TARGET_SHARES {
                                println!("  FAIL: NO size {{}} < {{}}", no_ask_size, TARGET_SHARES);
                            } else if combined < MIN_VALID_COMBINED_U64 {
                                println!("  FAIL: Combined ${{:.4}} < min ${{:.4}} (DUST QUOTES)", 
                                    combined as f64 / 1e6, MIN_VALID_COMBINED_U64 as f64 / 1e6);
                            } else if combined > EDGE_THRESHOLD_U64 {
                                println!("  FAIL: Combined ${{:.4}} > max ${{:.4}} (wide spread)", 
                                    combined as f64 / 1e6, EDGE_THRESHOLD_U64 as f64 / 1e6);
                            } else {
                                println!("  PASS: Edge detected! Combined=${{:.4}}", combined as f64 / 1e6);
                            }
                        }'''

new_debug = '''                        // Debug: show why edge detection fails
                        if total_evals % 50 == 0 {
                            let combined = yes_ask_price + no_ask_price;
                            println!("[EDGE CHECK] YES=${{:.4}} size={} | NO=${{:.4}} size={} | Combined=${{:.4}}", 
                                yes_ask_price as f64 / 1e6, yes_ask_size,
                                no_ask_price as f64 / 1e6, no_ask_size,
                                combined as f64 / 1e6);
                            
                            if yes_ask_price == 0 || yes_ask_price >= 100_000_000 {
                                println!("  X FAIL: YES price out of bounds");
                            } else if no_ask_price == 0 || no_ask_price >= 100_000_000 {
                                println!("  X FAIL: NO price out of bounds");
                            } else if yes_ask_size < TARGET_SHARES {
                                println!("  X FAIL: YES size {{}} < {{}}", yes_ask_size, TARGET_SHARES);
                            } else if no_ask_size < TARGET_SHARES {
                                println!("  X FAIL: NO size {{}} < {{}}", no_ask_size, TARGET_SHARES);
                            } else if combined < MIN_VALID_COMBINED_U64 {
                                println!("  X FAIL: Combined ${{:.4}} < min ${{:.4}} (DUST QUOTES)", 
                                    combined as f64 / 1e6, MIN_VALID_COMBINED_U64 as f64 / 1e6);
                            } else if combined > EDGE_THRESHOLD_U64 {
                                println!("  X FAIL: Combined ${{:.4}} > max ${{:.4}} (wide spread)", 
                                    combined as f64 / 1e6, EDGE_THRESHOLD_U64 as f64 / 1e6);
                            } else {
                                println!("  OK PASS: Edge detected! Combined=${{:.4}}", combined as f64 / 1e6);
                            }
                        }'''

content = content.replace(old_debug, new_debug)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Added detailed failure reason with USD formatting')
