#!/usr/bin/env python3
"""Apply all rate limiting and signature fixes"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Add enums after imports
old_imports = 'use crate::websocket_reader::WebSocketReader;'

new_imports = '''use crate::websocket_reader::WebSocketReader;

/// Commands from rollover thread to hot path
pub enum RolloverCommand {
    AddPair(u64, u64),
    RemovePair(u64),
}

/// Background task for execution thread
pub enum BackgroundTask {
    EdgeDetected {
        yes_token_hash: u64,
        no_token_hash: u64,
        yes_best_bid: u64,
        yes_best_ask: u64,
        yes_ask_size: u64,
        no_best_bid: u64,
        no_best_ask: u64,
        no_ask_size: u64,
        combined_ask: u64,
        timestamp_nanos: u64,
    },
    LatencyStats {
        min_ns: u64,
        max_ns: u64,
        avg_ns: u64,
        p99_ns: u64,
        sample_count: u64,
    },
}'''

content = content.replace(old_imports, new_imports)

# Fix function signature to accept 7 params
old_sig = '''pub fn run_sync_hot_path(
    opportunity_tx: Sender<OpportunitySnapshot>,
    all_tokens: Vec<String>,
    killswitch: Arc<AtomicBool>,
    token_pairs: HashMap<u64, u64>,
) {'''

new_sig = '''pub fn run_sync_hot_path(
    mut ws_stream: WebSocketReader,
    opportunity_tx: Sender<BackgroundTask>,
    all_tokens: Vec<String>,
    killswitch: Arc<AtomicBool>,
    token_pairs: HashMap<u64, u64>,
    edge_counter: Arc<AtomicU64>,
    rollover_rx: Receiver<RolloverCommand>,
) {'''

content = content.replace(old_sig, new_sig)

# Add metrics variables
old_init = '''    let start = Instant::now();
    let mut messages = 0u64;
    let mut pair_checks = 0u64;'''

new_init = '''    let start = Instant::now();
    let mut messages = 0u64;
    let mut pair_checks = 0u64;
    let mut total_evals = 0u64;
    let mut edges_found = 0u64;
    let mut last_report = Instant::now();
    let mut last_eval_count = 0u64;'''

content = content.replace(old_init, new_init)

# Add eval_trackers initialization
old_loop = '''    loop {
        if killswitch.load(Ordering::Relaxed) {'''

new_loop = '''    // Initialize rate limiters
    let mut eval_trackers: FxHashMap<u64, EvalTracker> = FxHashMap::default();
    for &token_hash in token_pairs.keys() {
        eval_trackers.insert(token_hash, EvalTracker::new());
    }
    
    loop {
        if killswitch.load(Ordering::Relaxed) {'''

content = content.replace(old_loop, new_loop)

# Add rate limiting before edge detection
old_edge = '''        // Edge detection: check complement pairs with sanity checks
        for (&token_hash, &complement_hash) in token_pairs.iter() {'''

new_edge = '''        // Edge detection: check complement pairs with rate limiting (100ms per market)
        for (&token_hash, &complement_hash) in token_pairs.iter() {
            // RATE LIMIT: Only evaluate each market once per 100ms
            let now = Instant::now();
            if let Some(tracker) = eval_trackers.get_mut(&token_hash) {
                if !tracker.can_evaluate(now) {
                    continue; // Skip - rate limited
                }
                total_evals += 1;
            }'''

content = content.replace(old_edge, new_edge)

# Fix edge detection to use BackgroundTask
old_send = '''                        let _ec = edge_counter.fetch_add(1, Ordering::Relaxed);
                        
                        // Update metrics
                        crate::hft_metrics::EDGE_COUNT.fetch_add(1, Ordering::Relaxed);'''

new_send = '''                        edges_found += 1;
                        edge_counter.fetch_add(1, Ordering::Relaxed);

                        // Send opportunity
                        let _ = opportunity_tx.try_send(BackgroundTask::EdgeDetected {
                            yes_token_hash: token_hash,
                            no_token_hash: complement_hash,
                            yes_best_bid: 0,
                            yes_best_ask: yes_ask_price,
                            yes_ask_size,
                            no_best_bid: 0,
                            no_best_ask: no_ask_price,
                            no_ask_size,
                            combined_ask,
                            timestamp_nanos: 0,
                        });'''

content = content.replace(old_send, new_send)

# Add metrics report
old_rollover = '''        // Check for rollover every 60 seconds
        if last_rollover_check.elapsed() >= Duration::from_secs(60) {'''

new_rollover = '''        // Periodic metrics report (every 1 second)
        if last_report.elapsed() >= Duration::from_secs(1) {
            let evals_this_sec = total_evals - last_eval_count;
            println!("[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {}",
                start.elapsed().as_secs(),
                messages,
                total_evals,
                edges_found,
                evals_this_sec
            );
            last_eval_count = total_evals;
            last_report = Instant::now();
        }
        
        // Check for rollover every 60 seconds
        if last_rollover_check.elapsed() >= Duration::from_secs(60) {'''

content = content.replace(old_rollover, new_rollover)

# Remove rollover_check init if it doesn't exist
content = content.replace('let mut last_rollover_check = Instant::now();\n    ', '')

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Applied all fixes')
