#!/usr/bin/env python3
"""Fix EXEC logging in background_wiring.rs"""

with open('/home/ubuntu/polymarket-hft-engine/src/background_wiring.rs', 'r') as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if '[EXEC] 🎯 Edge detected' in line:
        # Replace with minimal logging
        new_lines.append('    println!("[EDGE] ${:.4}", combined_f);\n')
    elif '[EXEC]' in line:
        # Skip other EXEC logs
        continue
    else:
        new_lines.append(line)

with open('/home/ubuntu/polymarket-hft-engine/src/background_wiring.rs', 'w') as f:
    f.writelines(new_lines)

print('Fixed')
