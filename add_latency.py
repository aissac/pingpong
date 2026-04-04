# Add latency measurement to hot_switchover.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Add minstant import at the top
old_imports = '''use tracing::{info, warn, debug, error};'''
new_imports = '''use tracing::{info, warn, debug, error};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;'''

content = content.replace(old_imports, new_imports)

# Add latency stats after struct AppState
old_state = '''pub struct AppState {'''
new_state = '''/// Global latency stats
pub static HOT_PATH_LATENCY_NS: AtomicU64 = AtomicU64::new(0);
pub static HOT_PATH_COUNT: AtomicU64 = AtomicU64::new(0);

pub struct AppState {'''

content = content.replace(old_state, new_state)

# Add timing to the WebSocket message handler
old_handler = '''                        Ok(Message::Text(text)) => {
                            // CRITICAL: Filter out pong messages before parsing
                            // simd-json panics on non-JSON messages
                            if text.contains("pong") || text == "{}" || text.is_empty() {
                                continue;
                            }
                            
                            if let Ok(updates) = parse_orderbook_update(&text) {'''

new_handler = '''                        Ok(Message::Text(text)) => {
                            // Priority 5: Nanosecond measurement
                            let start = std::time::Instant::now();
                            
                            // CRITICAL: Filter out pong messages before parsing
                            // simd-json panics on non-JSON messages
                            if text.contains("pong") || text == "{}" || text.is_empty() {
                                continue;
                            }
                            
                            if let Ok(updates) = parse_orderbook_update(&text) {'''

content = content.replace(old_handler, new_handler)

# Add latency recording after processing
old_process = '''                            // Update the orderbook tracker using the outcome
                            if let Some((condition_id, yes_price, no_price)) = state.update_by_outcome(&update.condition_id, update.price, outcome) {'''

new_process = '''                            // Record latency
                            let elapsed_ns = start.elapsed().as_nanos() as u64;
                            HOT_PATH_LATENCY_NS.fetch_add(elapsed_ns, Ordering::Relaxed);
                            HOT_PATH_COUNT.fetch_add(1, Ordering::Relaxed);
                            
                            // Log every 1000 messages
                            let count = HOT_PATH_COUNT.load(Ordering::Relaxed);
                            if count % 1000 == 0 {
                                let avg_ns = HOT_PATH_LATENCY_NS.load(Ordering::Relaxed) / count;
                                info!("📊 Hot path latency: avg={:.2}µs ({} samples)", avg_ns as f64 / 1000.0, count);
                            }
                            
                            // Update the orderbook tracker using the outcome
                            if let Some((condition_id, yes_price, no_price)) = state.update_by_outcome(&update.condition_id, update.price, outcome) {'''

content = content.replace(old_process, new_process)

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Added latency measurement')