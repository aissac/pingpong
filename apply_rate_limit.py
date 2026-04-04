#!/usr/bin/env python3
"""Apply rate limiting fixes to hft_hot_path.rs"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# 1. Add imports after existing ones
old_imports = 'use crossbeam_channel::Sender;'
new_imports = 'use crossbeam_channel::Sender;\nuse std::time::{Instant, Duration};\nuse rustc_hash::FxHashMap;'
content = content.replace(old_imports, new_imports)

# 2. Add rate limit constant
old_consts = 'const TARGET_SHARES: u64 = 100;'
new_consts = 'const TARGET_SHARES: u64 = 100;\nconst EVAL_RATE_LIMIT_MS: u64 = 100; // Only evaluate each market once per 100ms'
content = content.replace(old_consts, new_consts)

# 3. Add EvalTracker struct
old_struct = '/// Token orderbook state - tracks both bid and ask\n#[derive(Clone, Debug)]\npub struct TokenBookState {'

new_struct = '''/// Track evaluation times for rate limiting
struct EvalTracker {
    last_eval: Instant,
    eval_count: u64,
}

impl EvalTracker {
    fn new() -> Self {
        Self {
            last_eval: Instant::now() - Duration::from_millis(EVAL_RATE_LIMIT_MS),
            eval_count: 0,
        }
    }
    
    fn can_evaluate(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_eval) >= Duration::from_millis(EVAL_RATE_LIMIT_MS) {
            self.last_eval = now;
            self.eval_count += 1;
            true
        } else {
            false
        }
    }
}

/// Token orderbook state - tracks both bid and ask
#[derive(Clone, Debug)]
pub struct TokenBookState {'''

content = content.replace(old_struct, new_struct)

# 4. Add metrics variables to hot path
old_init = '''    let start = Instant::now();
    let mut messages = 0u64;
    let mut pair_checks = 0u64;'''

new_init = '''    let start = Instant::now();
    let mut messages = 0u64;
    let mut pair_checks = 0u64;
    let mut total_evals = 0u64;
    let mut edges_found = 0u64;
    let mut last_report = Instant::now();
    let mut last_eval_count = 0u64;
    
    // Initialize rate limiters for all tokens
    let mut eval_trackers: FxHashMap<u64, EvalTracker> = FxHashMap::default();
    for &token_hash in token_pairs.keys() {
        eval_trackers.insert(token_hash, EvalTracker::new());
    }'''

content = content.replace(old_init, new_init)

# 5. Add rate limiting before edge detection loop
old_loop = '''        // Edge detection: check complement pairs with sanity checks
        for (&token_hash, &complement_hash) in token_pairs.iter() {'''

new_loop = '''        // Edge detection: check complement pairs with rate limiting
        for (&token_hash, &complement_hash) in token_pairs.iter() {
            // RATE LIMIT: Only evaluate each market once per 100ms
            if let Some(tracker) = eval_trackers.get_mut(&token_hash) {
                if !tracker.can_evaluate() {
                    continue; // Skip this market - rate limited
                }
                total_evals += 1;
            }'''

content = content.replace(old_loop, new_loop)

# 6. Add edges_found counter
old_edge = 'let _ec = edge_counter.fetch_add(1, Ordering::Relaxed);'
new_edge = 'let _ec = edge_counter.fetch_add(1, Ordering::Relaxed);\n                        edges_found += 1;'
content = content.replace(old_edge, new_edge)

# 7. Add periodic metrics report
old_rollover = '''        // Check for rollover every 60 seconds
        if last_rollover_check.elapsed() >= Duration::from_secs(60) {'''

new_rollover = '''        // Periodic metrics report (every 1 second)
        if last_report.elapsed() >= Duration::from_secs(1) {
            let evals_this_sec = total_evals - last_eval_count;
            println!("[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {}",
                start.elapsed().as_secs(), messages, total_evals, edges_found, evals_this_sec);
            last_eval_count = total_evals;
            last_report = Instant::now();
        }
        
        // Check for rollover every 60 seconds
        if last_rollover_check.elapsed() >= Duration::from_secs(60) {'''

content = content.replace(old_rollover, new_rollover)

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Applied all rate limiting fixes')
