# Add latency measurement to hot_switchover.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Add static variables after imports
old_imports = '''use tracing::{info, warn, error, debug};

// Re-export OrderBookUpdate from websocket module'''

new_imports = '''use tracing::{info, warn, error, debug};
use std::sync::atomic::{AtomicU64, Ordering};

// Latency tracking (Priority 5)
static HOT_PATH_LATENCY_NS: AtomicU64 = AtomicU64::new(0);
static HOT_PATH_COUNT: AtomicU64 = AtomicU64::new(0);

// Re-export OrderBookUpdate from websocket module'''

content = content.replace(old_imports, new_imports)

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Added latency tracking')