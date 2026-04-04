# Fix type errors in hot_path_optimized.rs

with open('/home/ubuntu/polymarket-hft-engine/src/hot_path_optimized.rs', 'r') as f:
    content = f.read()

# Fix the evaluate_and_trade function
content = content.replace(
    'if combined > 1_020_000',
    'if combined > 1.02'
)
content = content.replace(
    'let edge = 1_000_000 - combined',
    'let edge = 1.0 - combined'
)
content = content.replace(
    'let edge_pct = (edge * 100) / combined',
    'let edge_pct = (edge / combined) * 100.0'
)
content = content.replace(
    'if edge_pct > 35',
    'if edge_pct > 3.5'
)
content = content.replace(
    'edge_pct as f64 / 10.0',
    'edge_pct'
)

with open('/home/ubuntu/polymarket-hft-engine/src/hot_path_optimized.rs', 'w') as f:
    f.write(content)

print('Fixed type errors')