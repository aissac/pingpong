#!/usr/bin/env python3
"""Fix bin to spawn rollover thread with tokio runtime"""

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'r') as f:
    content = f.read()

# Replace tokio spawn with regular thread spawn
old_spawn = '''    // Spawn async rollover thread
    let rollover_client = Arc::clone(&http_client);
    let rollover_tx_clone = rollover_tx.clone();
    
    tokio::spawn(async move {
        pingpong::market_rollover::run_rollover_thread(rollover_client, rollover_tx_clone).await;
    });
    
    println!("✅ Rollover thread spawned (15s polling)");'''

new_spawn = '''    // Spawn rollover thread (blocking, uses reqwest blocking client)
    let rollover_client = Arc::clone(&http_client);
    let rollover_tx_clone = rollover_tx.clone();
    
    std::thread::spawn(move || {
        // Create a tokio runtime for the async function
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            pingpong::market_rollover::run_rollover_thread(rollover_client, rollover_tx_clone).await;
        });
    });
    
    println!("✅ Rollover thread spawned (15s polling)");'''

content = content.replace(old_spawn, new_spawn)

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'w') as f:
    f.write(content)

print('Fixed to use blocking thread with tokio runtime')
