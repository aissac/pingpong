#!/usr/bin/env python3
"""Fix verbose logging in maker_taker_routing.rs"""

with open('/home/ubuntu/polymarket-hft-engine/src/maker_taker_routing.rs', 'r') as f:
    content = f.read()

# Find the function and replace it
start_marker = 'pub async fn execute_dry_run_sequence('

start_idx = content.find(start_marker)
if start_idx == -1:
    print('Function not found')
    exit(1)

# Find the matching closing brace
brace_count = 0
end_idx = start_idx
in_func = False

for i, char in enumerate(content[start_idx:], start_idx):
    if char == '{':
        brace_count += 1
        in_func = True
    elif char == '}':
        brace_count -= 1
        if in_func and brace_count == 0:
            end_idx = i + 1
            break

new_func = '''pub async fn execute_dry_run_sequence(
    _maker: &MakerLeg,
    _taker: &TakerLeg,
    _net_fee: f64,
    combined_ask: f64,
) {
    // Minimal logging - just edge count
    println!("[EDGE] ${:.4}", combined_ask);
}'''

new_content = content[:start_idx] + new_func + content[end_idx:]

with open('/home/ubuntu/polymarket-hft-engine/src/maker_taker_routing.rs', 'w') as f:
    f.write(new_content)

print('Function replaced successfully')
