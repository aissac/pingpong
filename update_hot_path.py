#!/usr/bin/env python3
"""Update hft_hot_path.rs to use metrics instead of verbose logging"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Replace the pair_checks logging block
old_block = '''                    pair_checks += 1;
                    // Log every 10M checks to reduce log volume
                    if pair_checks % 10_000_000 == 0 {
                        println!("[HFT] pair_checks={} edges={}", pair_checks, edge_counter.load(Ordering::Relaxed));
                    }
                    
                    // Only trigger if mathematically valid
                    if combined_ask <= EDGE_THRESHOLD_U64 && combined_ask >= MIN_VALID_COMBINED_U64 {
                        let ec = edge_counter.fetch_add(1, Ordering::Relaxed);
                        // Log every 1000th edge to reduce log volume
                        if ec % 1000 == 0 {
                            println!("[EDGE] #{} combined=${:.4}", ec, combined_ask as f64 / 1_000_000.0);
                        }'''

new_block = '''                    pair_checks += 1;
                    
                    // Update metrics (heartbeat will log summary)
                    crate::hft_metrics::PAIR_CHECKS.fetch_add(1, Ordering::Relaxed);
                    
                    // Only trigger if mathematically valid
                    if combined_ask <= EDGE_THRESHOLD_U64 && combined_ask >= MIN_VALID_COMBINED_U64 {
                        let _ec = edge_counter.fetch_add(1, Ordering::Relaxed);
                        
                        // Update metrics
                        crate::hft_metrics::EDGE_COUNT.fetch_add(1, Ordering::Relaxed);
                        
                        // Track deepest edge
                        let deepest = crate::hft_metrics::DEEPEST_EDGE.load(Ordering::Relaxed);
                        if combined_ask < deepest || deepest == 0 {
                            crate::hft_metrics::DEEPEST_EDGE.store(combined_ask, Ordering::Relaxed);
                        }'''

content = content.replace(old_block, new_block)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Updated hft_hot_path.rs')