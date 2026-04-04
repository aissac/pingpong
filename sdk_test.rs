//! Simple test: Connect to Polymarket WebSocket using official SDK
//! This proves the SDK works and receives messages

use polymarket_client_sdk::clob::ws::Client as WsClient;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎯 Polymarket SDK WebSocket Test");
    println!("================================");
    println!("");
    
    // Use a known active token (BTC 5m market)
    let token_id = "32911245970249139387900295047767609805180164726280382719832452627263709390368".to_string();
    
    println!("🔗 Connecting to Polymarket WebSocket...");
    println!("   Token: {}... (first 50 chars)", &token_id[..50]);
    println!("");
    
    // Create SDK client (handles WAF bypass automatically)
    let client = WsClient::default();
    
    // Subscribe to orderbook
    println!("📡 Subscribing to orderbook...");
    let mut stream = client.subscribe_orderbook(vec![token_id])?;
    
    println!("✅ Subscription successful!");
    println!("");
    println!("⏱️  Waiting for messages (timeout: 30 seconds)...");
    println!("");
    
    // Wait for messages with timeout
    let timeout = tokio::time::Duration::from_secs(30);
    let mut msg_count = 0u64;
    
    loop {
        match tokio::time::timeout(timeout, stream.next()).await {
            Ok(Some(Ok(book))) => {
                msg_count += 1;
                
                println!("📨 Message #{}:", msg_count);
                println!("   Asset: {}", book.asset_id.chars().take(50).collect::<String>());
                println!("   Bids: {} levels", book.bids.len());
                println!("   Asks: {} levels", book.asks.len());
                
                if let Some(best_bid) = book.bids.first() {
                    println!("   Best Bid: ${:.4} (size: {})", best_bid.price, best_bid.size);
                }
                
                if let Some(best_ask) = book.asks.first() {
                    println!("   Best Ask: ${:.4} (size: {})", best_ask.price, best_ask.size);
                }
                
                // Calculate spread
                if let (Some(bid), Some(ask)) = (book.bids.first(), book.asks.first()) {
                    let spread = ask.price - bid.price;
                    let spread_pct = (spread / bid.price) * 100.0;
                    println!("   Spread: ${:.4} ({:.2}%)", spread, spread_pct);
                }
                
                println!("");
                
                // Stop after 5 messages (proof it works)
                if msg_count >= 5 {
                    println!("🎉 SUCCESS! Received 5 messages - WebSocket is WORKING!");
                    println!("");
                    println!("The SDK bypasses Cloudflare WAF automatically.");
                    println!("No more silent drops!");
                    break;
                }
            }
            Ok(Some(Err(e))) => {
                eprintln!("🚨 Error: {}", e);
                break;
            }
            Ok(None) => {
                eprintln!("⚠️  Stream ended unexpectedly");
                break;
            }
            Err(_) => {
                eprintln!("⏱️  Timeout - no messages received in 30 seconds");
                eprintln!("");
                eprintln!("This might mean:");
                eprintln!("  1. Token ID is expired/inactive");
                eprintln!("  2. Network connectivity issue");
                eprintln!("  3. Polymarket API is down");
                break;
            }
        }
    }
    
    if msg_count == 0 {
        println!("");
        println!("❌ FAILED: No messages received");
        std::process::exit(1);
    }
    
    Ok(())
}
