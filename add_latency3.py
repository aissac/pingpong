# Add latency measurement to message handler

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'r') as f:
    content = f.read()

# Add timing to message handler
old_handler = '''                        Ok(Message::Text(text)) => {
                            // Parse orderbook updates (serde_json)
                            if let Ok(updates) = parse_orderbook_update(&text) {
                                for update in updates {
                                    let _ = event_tx.send(WsEvent::Update(source, update));
                                }
                            }
                        }'''

new_handler = '''                        Ok(Message::Text(text)) => {
                            // Priority 5: Nanosecond latency measurement
                            let start = std::time::Instant::now();
                            
                            // Parse orderbook updates (serde_json)
                            if let Ok(updates) = parse_orderbook_update(&text) {
                                for update in updates {
                                    let _ = event_tx.send(WsEvent::Update(source, update));
                                }
                            }
                            
                            // Record latency
                            let elapsed_ns = start.elapsed().as_nanos() as u64;
                            HOT_PATH_LATENCY_NS.fetch_add(elapsed_ns, Ordering::Relaxed);
                            let count = HOT_PATH_COUNT.fetch_add(1, Ordering::Relaxed);
                            
                            // Log every 1000 messages
                            if count % 1000 == 0 && count > 0 {
                                let avg_ns = HOT_PATH_LATENCY_NS.load(Ordering::Relaxed) / count;
                                info!("📊 Hot path latency: avg={:.2}µs ({} samples)", avg_ns as f64 / 1000.0, count);
                            }
                        }'''

content = content.replace(old_handler, new_handler)

with open('/home/ubuntu/polymarket-hft-engine/src/hot_switchover.rs', 'w') as f:
    f.write(content)

print('Added latency measurement to message handler')