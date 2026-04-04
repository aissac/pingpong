//! Polymarket WebSocket Client using Official SDK
//! 
//! This replaces the raw tungstenite WebSocket with the official polymarket-client-sdk
//! which handles Cloudflare WAF bypass automatically.

use polymarket_client_sdk::clob::ws::Client as WsClient;
use futures::StreamExt;
use std::time::Duration;
use tokio::time::sleep;

/// Connect to Polymarket WebSocket using official SDK
/// Returns a stream of orderbook updates
pub async fn create_orderbook_stream(
    token_ids: Vec<String>,
) -> Result<impl futures::Stream<Item = Result<polymarket_client_sdk::clob::ws::OrderbookUpdate, anyhow::Error>>, anyhow::Error> {
    
    println!("🔗 [SDK] Connecting to Polymarket WebSocket via official SDK...");
    
    // SDK handles:
    // - Correct WebSocket URL (wss://ws-subscriptions-clob.polymarket.com/ws/market)
    // - Cloudflare WAF bypass (correct TLS fingerprints)
    // - Ping/pong keepalives
    // - Reconnection logic
    let client = WsClient::default();
    
    println!("📡 [SDK] Subscribing to {} tokens...", token_ids.len());
    
    // Subscribe to orderbook updates
    let stream = client.subscribe_orderbook(token_ids)?;
    
    println!("✅ [SDK] WebSocket subscription successful!");
    
    Ok(stream)
}

/// Run WebSocket stream and forward updates to channel
pub async fn run_websocket_stream(
    token_ids: Vec<String>,
    orderbook_tx: crossbeam_channel::Sender<polymarket_client_sdk::clob::ws::OrderbookUpdate>,
) -> Result<(), anyhow::Error> {
    
    let mut stream = create_orderbook_stream(token_ids).await?;
    
    println!("🔄 [SDK] Starting WebSocket stream loop...");
    
    let mut msg_count = 0u64;
    let start_time = std::time::Instant::now();
    
    while let Some(book_result) = stream.next().await {
        match book_result {
            Ok(book) => {
                msg_count += 1;
                
                // Log stats every 100 messages
                if msg_count % 100 == 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let msg_per_sec = msg_count as f64 / elapsed;
                    println!("📊 [SDK] Received {} messages ({:.1} msg/s)", msg_count, msg_per_sec);
                }
                
                // Forward to hot path
                if orderbook_tx.send(book).is_err() {
                    eprintln!("❌ [SDK] Channel closed, stopping WebSocket");
                    break;
                }
            }
            Err(e) => {
                eprintln!("🚨 [SDK] WebSocket error: {}", e);
                // SDK handles reconnection automatically
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_websocket_connection() {
        // Test with a known active token
        let token_id = "32911245970249139387900295047767609805180164726280382719832452627263709390368".to_string();
        
        let result = create_orderbook_stream(vec![token_id]).await;
        assert!(result.is_ok(), "WebSocket connection should succeed");
    }
}
