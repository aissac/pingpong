#!/usr/bin/env python3
"""Fix bin/hft_pingpong_v2.rs to spawn rollover thread correctly"""

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'r') as f:
    content = f.read()

# Replace the broken rollover spawn with working one
old_spawn = '''    thread::spawn(move || {
        println!("⏳ [ROLLOVER] Started monitoring for market transitions");
        
        loop {
            thread::sleep(Duration::from_secs(60));
            
            let now = Utc::now();
            let periods = get_current_periods();
            
            // Check for rollover (currently just logs - TODO: fetch actual tokens)
            let (to_subscribe, to_unsubscribe) = check_rollover(&mut rollover_state, &periods);
            
            if !to_subscribe.is_empty() {
                println!("[ROLLOVER] 📡 Would subscribe to {} tokens at {:02}:{:02}", 
                    to_subscribe.len(), now.hour(), now.minute());
                // TODO: Send actual tokens when token fetching is implemented
                // if rollover_tx.send(RolloverCommand::Subscribe { tokens: to_subscribe }).is_err() {
                //     eprintln!("[ROLLOVER] ⚠️ Channel disconnected");
                //     break;
                // }
            }
            
            if !to_unsubscribe.is_empty() {
                println!("[ROLLOVER] 🗑️ Would unsubscribe from {} tokens at {:02}:{:02}", 
                    to_unsubscribe.len(), now.hour(), now.minute());
            }
            
            println!("[ROLLOVER] 🔄 Checked at {:02}:{:02}:{:02} - {} periods tracked", 
                now.hour(), now.minute(), now.second(), rollover_state.active_periods.len());
        }
    });
    
    println!("✅ Rollover checker thread spawned (60s interval)");'''

new_spawn = '''    // Spawn async rollover thread
    let rollover_client = Arc::clone(&http_client);
    let rollover_tx_clone = rollover_tx.clone();
    
    tokio::spawn(async move {
        market_rollover::run_rollover_thread(rollover_client, rollover_tx_clone).await;
    });
    
    println!("✅ Rollover thread spawned (15s polling)");'''

content = content.replace(old_spawn, new_spawn)

with open('/home/ubuntu/polymarket-hft-engine/src/bin/hft_pingpong_v2.rs', 'w') as f:
    f.write(content)

print('Fixed binary to spawn rollover thread')
