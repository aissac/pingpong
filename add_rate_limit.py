#!/usr/bin/env python3
"""Add rate limiting to current hft_hot_path.rs"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Add imports
if 'use std::time::{Instant, Duration};' not in content:
    content = content.replace(
        'use crossbeam_channel::{Sender, Receiver};',
        'use crossbeam_channel::{Sender, Receiver};\nuse std::time::{Instant, Duration};'
    )

# Add constant
if 'const EVAL_RATE_LIMIT_MS' not in content:
    content = content.replace(
        'const TARGET_SHARES: u64 = 100;',
        'const TARGET_SHARES: u64 = 100;\nconst EVAL_RATE_LIMIT_MS: u64 = 100;'
    )

# Add EvalTracker struct
eval_tracker = '''
/// Track evaluation times for rate limiting (100ms per market)
pub struct EvalTracker {
    last_eval: Instant,
}

impl EvalTracker {
    pub fn new() -> Self {
        Self {
            last_eval: Instant::now() - Duration::from_secs(1),
        }
    }

    pub fn can_evaluate(&mut self, now: Instant) -> bool {
        if now.duration_since(self.last_eval).as_millis() as u64 >= EVAL_RATE_LIMIT_MS {
            self.last_eval = now;
            true
        } else {
            false
        }
    }
}

'''

insert_point = content.find('pub fn run_sync_hot_path(')
if insert_point > 0:
    content = content[:insert_point] + eval_tracker + content[insert_point:]

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)

print('Added rate limiting structs')
