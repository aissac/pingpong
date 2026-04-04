#!/usr/bin/env python3
"""Reduce SUBSCRIBE logging verbosity"""

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'r') as f:
    content = f.read()

# Find and replace the SUBSCRIBE loop
old_code = '''    println!("[SUBSCRIBE] Tokens:");
    for (i, token) in all_tokens.iter().enumerate() {
        let hash = hash_token(token.as_bytes());
        println!("[SUBSCRIBE]   #{} len={} hash={:x} token={}", i, token.len(), hash, token);
    }'''

new_code = '''    println!("[SUBSCRIBE] {} tokens (logging first 5 only)", all_tokens.len());
    for (i, token) in all_tokens.iter().take(5).enumerate() {
        let hash = hash_token(token.as_bytes());
        println!("[SUBSCRIBE]   #{} len={} hash={:x} token={}", i, token.len(), hash, token);
    }'''

content = content.replace(old_code, new_code)

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'w') as f:
    f.write(content)

print('SUBSCRIBE logging reduced')
