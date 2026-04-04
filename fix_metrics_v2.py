#!/usr/bin/env python3
"""Fix METRICS logging to use LogEvent - correct pattern"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Find and replace the actual METRICS println pattern
old = '''if total_evals - last_eval_count >= 300 || start.elapsed() - last_time >= Duration::from_secs(1) {
            let evals_this_sec = total_evals - last_eval_count;
            println!("[METRICS] {}s | msg: {} | evals: {} | edges: {} | evals/sec: {} | pairs:{}",
                start.elapsed().as_secs(),
                messages,
                total_evals,
                edges_found,
                evals_this_sec,
                token_pairs.len()
            );
            last_eval_count = total_evals;
            last_time = start.elapsed();
        }'''

new = '''if total_evals - last_eval_count >= 300 || start.elapsed() - last_time >= Duration::from_secs(1) {
            let evals_this_sec = total_evals - last_eval_count;
            let _ = log_tx.try_send(LogEvent::Metric {
                uptime: start.elapsed().as_secs(),
                msgs: messages,
                evals: total_evals,
                edges: edges_found,
                evals_sec: evals_this_sec as u64,
                pairs: token_pairs.len() as u64,
                dropped: 0,
            });
            last_eval_count = total_evals;
            last_time = start.elapsed();
        }'''

if old in content:
    content = content.replace(old, new)
    print('Replaced METRICS println with LogEvent::Metric')
else:
    print('Pattern not found')
    # Show what we have around the METRICS line
    import re
    match = re.search(r'println!\("\[METRICS\].*?token_pairs\.len\(\)', content, re.DOTALL)
    if match:
        start = max(0, match.start() - 50)
        end = min(len(content), match.end() + 100)
        print(f'Found: {content[start:end]}')

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)
