#!/usr/bin/env python3
"""Apply rate limiting fixes step 4 and 5"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# STEP 4: Add rate limit check before edge detection
old_loop = '''        // Edge detection: check complement pairs with sanity checks
        for (&token_hash, &complement_hash) in token_pairs.iter() {'''

new_loop = '''        // Edge detection: check complement pairs with rate limiting (100ms per market)
        for (&token_hash, &complement_hash) in token_pairs.iter() {
            // RATE LIMIT: Only evaluate each market once per 100ms
            let now = Instant::now();
            if let Some(tracker) = eval_trackers.get_mut(&token_hash) {
                if !tracker.can_evaluate(now) {
                    continue; // Skip - rate limited
                }
                total_evals += 1;
            }'''

content = content.replace(old_loop, new_loop)

# Also add edges_found counter after edge detection
old_edge = '''let _ec = edge_counter.fetch_add(1, Ordering::Relaxed);'''
new_edge = '''let _ec = edge_counter.fetch_add(1, Ordering::Relaxed);
                        edges_found += 1;'''

content = content.replace(old_edge, new_edge)

# STEP 5: Add periodic metrics report
old_rollover = '''        // Check for rollover every 60 seconds
        if last_rollover_check.elapsed() >= Duration::from_secs(60) {'''

new_rollover = '''        // Periodic metrics report (every 1 second)
        if last_metrics_report.elapsed() >= Duration::from_secs(1) {
            println!("[METRICS] Evals/sec: {} | Edges/sec: {} | Markets: {}",
                total_evals, edges_found, eval_trackers.len());
            total_evals = 0;
            edges_found = 0;
            last_metrics_report = Instant::now();
        }
        
        // Check for rollover every 60 seconds
        if last_rollover_check.elapsed() >= Duration::from_secs(60) {'''

content = content.replace(old_rollover, new_rollover)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Applied rate limiting fixes step 4 and 5')
