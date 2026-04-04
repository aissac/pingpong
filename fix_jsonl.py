#!/usr/bin/env python3
"""Fix jsonl_logger.rs to include branch counters"""

with open('/home/ubuntu/polymarket-hft-engine/src/jsonl_logger.rs', 'r') as f:
    content = f.read()

# Step 1: Add fields to Metric enum variant
content = content.replace(
    'Metric {\n        uptime: u64,\n        msgs: u64,\n        evals: u64,\n        edges: u64,\n        evals_sec: u64,\n        pairs: u64,\n        dropped: u64,\n    },',
    'Metric {\n        uptime: u64,\n        msgs: u64,\n        evals: u64,\n        edges: u64,\n        evals_sec: u64,\n        pairs: u64,\n        dropped: u64,\n        valid_evals: u64,\n        missing_data: u64,\n    },'
)

# Step 2: Update the formatter
old_fmt = '''LogEvent::Metric { uptime, msgs, evals, edges, evals_sec, pairs, dropped } => {
                write!(&mut cursor, 
                    "{{\\"t\\":\\"m\\",\\"up\\":{},\\"msg\\":{},\\"ev\\":{},\\"ed\\":{},\\"rps\\":{},\\"pr\\":{},\\"dr\\":{}}}\\n",
                    uptime, msgs, evals, edges, evals_sec, pairs, dropped
                ).unwrap();
            },'''

new_fmt = '''LogEvent::Metric { uptime, msgs, evals, edges, evals_sec, pairs, dropped, valid_evals, missing_data } => {
                write!(&mut cursor, 
                    "{{\\"t\\":\\"m\\",\\"up\\":{},\\"msg\\":{},\\"ev\\":{},\\"ed\\":{},\\"rps\\":{},\\"pr\\":{},\\"dr\\":{},\\"valid\\":{},\\"missing\\":{}}}\\n",
                    uptime, msgs, evals, edges, evals_sec, pairs, dropped, valid_evals, missing_data
                ).unwrap();
            },'''

if old_fmt in content:
    content = content.replace(old_fmt, new_fmt)
    print('✅ Fixed formatter')
else:
    print('Formatter pattern not found')

with open('/home/ubuntu/polymarket-hft-engine/src/jsonl_logger.rs', 'w') as f:
    f.write(content)

print('Done')
