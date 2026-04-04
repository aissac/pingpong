# Fix type errors in hot_path_optimized.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_path_optimized.rs', 'r') as f:
    lines = f.readlines()

# Find and fix the evaluate_and_trade function
new_lines = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Fix the combined comparison
    if 'if combined > 1_020_000' in line:
        new_lines.append(line.replace('if combined > 1_020_000', 'if combined > 1.02'))
        i += 1
        continue
    
    # Fix the edge calculation
    if 'let edge = 1_000_000 - combined' in line:
        new_lines.append(line.replace('let edge = 1_000_000 - combined', 'let edge = 1.0 - combined'))
        i += 1
        continue
    
    # Fix the edge_pct calculation
    if 'let edge_pct = (edge * 100) / combined' in line:
        new_lines.append(line.replace('let edge_pct = (edge * 100) / combined', 'let edge_pct = (edge / combined) * 100.0'))
        i += 1
        continue
    
    # Fix the edge_pct comparison
    if 'if edge_pct > 35' in line:
        new_lines.append(line.replace('if edge_pct > 35', 'if edge_pct > 3.5'))
        i += 1
        continue
    
    # Fix the debug log
    if 'edge_pct as f64 / 10.0' in line:
        new_lines.append(line.replace('edge_pct as f64 / 10.0', 'edge_pct'))
        i += 1
        continue
    
    new_lines.append(line)
    i += 1

with open('/home/ubuntu/polymarket-hft-engine/src/hot_path_optimized.rs', 'w') as f:
    f.writelines(new_lines)

print('Fixed type errors')