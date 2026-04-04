#!/usr/bin/env python3
"""Fix METRICS logging - exact pattern match"""

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'r') as f:
    content = f.read()

# Exact pattern from the file
old = '''if last_report.elapsed() >= Duration::from_secs(1) {
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
            last_report = Instant::now();
        }'''

new = '''if last_report.elapsed() >= Duration::from_secs(1) {
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
            last_report = Instant::now();
        }'''

if old in content:
    content = content.replace(old, new)
    print('SUCCESS: Replaced METRICS println with LogEvent::Metric')
else:
    print('FAILED: Pattern not found')
    # Debug: show what we have
    idx = content.find('println!("[METRICS]')
    if idx >= 0:
        print(f'Found METRICS at position {idx}')
        print('Context:')
        print(content[max(0,idx-200):idx+300])

with open('/home/ubuntu/polymarket-hft-engine/src/hft_hot_path.rs', 'w') as f:
    f.write(content)
