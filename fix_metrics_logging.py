#!/usr/bin/env python3
"""Fix METRICS logging to use LogEvent"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Find and replace the METRICS println
old = '''// Periodic metrics (every 30 seconds)
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

new = '''// Periodic metrics (every 1 second)
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

if old in content:
    content = content.replace(old, new)
    print('Replaced METRICS println with LogEvent::Metric')
else:
    print('Pattern not found - checking actual content...')
    # Find the actual pattern
    import re
    match = re.search(r'println!\(\s*"\[METRICS\]', content)
    if match:
        start = max(0, match.start() - 100)
        end = min(len(content), match.end() + 300)
        print(f'Found at {start}-{end}:')
        print(content[start:end])

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)
