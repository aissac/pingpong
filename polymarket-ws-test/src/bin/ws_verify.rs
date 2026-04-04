// WebSocket Verification Test - Step 1 of Phase 3
// This test validates correct subscription format with live tokens

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;

/// Token info from Gamma API
#[derive(Debug, Deserialize)]
struct TokenInfo {
    token_id: String,
    question: String,
    active: bool,
}

/// WebSocket subscription message (CORRECT FORMAT - uses "assets" NOT "assets_ids")
#[derive(Serialize)]
struct WsSubscription {
    assets: Vec<String>,  // CRITICAL: Must be "assets", not "assets_ids"
    #[serde(rename = "type")]
    msg_type: String,
}

/// Orderbook update from Polymarket
#[derive(Debug, Deserialize)]
struct OrderbookMessage {
    #[serde(rename = "type")]
    msg_type: Option<String>,
    asset_id: Option<String>,
    market_slug: Option<String>,
    #[serde(rename = "asset_ids")]
    asset_ids: Option<Vec<String>>,
    // Orderbook fields
    hash: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "marketID")]
    market_id: Option<String>,
    // Expected event types: "book" (initial snapshot) or "price_change" (updates)
    event: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Phase 3 Step 1: WebSocket Verification\n");
    
    // Step 1: Fetch live tokens from Gamma API
    println!("📡 Step 1: Fetching live tokens from Gamma API...");
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .timeout(Duration::from_secs(10))
        .build()?;
    
    let response = client
        .get("https://gamma-api.polymarket.com/markets?active=true&limit=5")
        .send()
        .await?;
    
    let tokens: Vec<TokenInfo> = response.json().await?;
    println!("✅ Fetched {} active markets", tokens.len());
    
    if tokens.is_empty() {
        return Err("No active tokens found".into());
    }
    
    // Pick first 2 active tokens for testing
    let test_tokens: Vec<String> = tokens.iter()
        .filter(|t| t.active)
        .take(2)
        .map(|t| t.token_id.clone())
        .collect();
    
    println!("\n📊 Test tokens:");
    for (i, token) in tokens.iter().take(2).enumerate() {
        println!("   {}. {} (ID: {}...)", i + 1, 
            token.question.chars().take(50).collect::<String>(),
            &token.token_id[..20]);
    }
    println!("   Token IDs: {:?}", test_tokens);
    
    // Step 2: Connect to WebSocket
    println!("\n🔌 Step 2: Connecting to Polymarket WebSocket...");
    let ws_url = "wss://ws-subscriptions-clob.polymarket.com/ws/market";
    
    // WAF bypass headers
    let request = tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(ws_url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
        .header("Origin", "https://polymarket.com")
        .header("Host", "ws-subscriptions-clob.polymarket.com")
        .body(())?;
    
    let (ws_stream, response) = connect_async(request).await?;
    println!("✅ Connected! Status: {}", response.status());
    
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Step 3: Subscribe with CORRECT format (uses "assets" key)
    println!("\n📝 Step 3: Subscribing with assets format...");
    let subscription = WsSubscription {
        assets: test_tokens.clone(),  // Key is "assets" not "assets_ids"
        msg_type: "market".to_string(),
    };
    
    let subscribe_msg = serde_json::to_string(&subscription)?;
    println!("   Sending: {}", subscribe_msg);
    
    ws_sender.send(Message::Text(subscribe_msg)).await?;
    println!("✅ Subscription sent!");
    
    // Step 4: Listen for orderbook updates
    println!("\n📥 Step 4: Listening for orderbook updates (30 seconds)...\n");
    
    let mut message_count = 0;
    let mut orderbook_count = 0;
    let timeout = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(timeout);
    
    loop {
        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        message_count += 1;
                        
                        // Parse message
                        if let Ok(parsed) = serde_json::from_str::<OrderbookMessage>(&text) {
                            println!("📨 Message {}: type={:?}, event={:?}", 
                                message_count, parsed.msg_type, parsed.event);
                            
                            // Check if it's an orderbook update
                            if parsed.msg_type.as_deref() == Some("market") {
                                orderbook_count += 1;
                                match parsed.event.as_deref() {
                                    Some("book") => println!("   ✅ INITIAL SNAPSHOT"),
                                    Some("price_change") => println!("   ✅ PRICE UPDATE"),
                                    _ => println!("   ✅ MARKET EVENT"),
                                }
                                println!("   Asset: {:?}", parsed.asset_id);
                                println!("   Market: {:?}", parsed.market_slug);
                                println!("   Hash: {:?}", parsed.hash);
                            }
                        } else {
                            println!("📨 Message {}: {}", message_count, 
                                text.chars().take(100).collect::<String>());
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        println!("🏓 Ping received, sending pong");
                        let _ = ws_sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        println!("⚠️ Server closed connection");
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        println!("❌ Error: {}", e);
                        break;
                    }
                    None => break,
                }
            }
            _ = &mut timeout => {
                println!("\n⏰ Timeout reached");
                break;
            }
        }
    }
    
    // Summary
    println!("\n📊 VERIFICATION RESULTS:");
    println!("   Messages received: {}", message_count);
    println!("   Orderbook updates: {}", orderbook_count);
    
    if orderbook_count > 0 {
        println!("\n✅ SUCCESS: WebSocket subscription working!");
        println!("   - Correct format: {{\"assets\": [...], \"type\": \"market\"}}");
        println!("   - Key must be \"assets\" NOT \"assets_ids\"");
        println!("   - Receiving market events (book/price_change)");
        println!("   - Ready to integrate into main bot");
    } else if message_count > 0 {
        println!("\n⚠️ WARNING: Connected and receiving messages, but no market events");
        println!("   - Check token IDs are valid ERC1155 token IDs");
        println!("   - Ensure tokens are active markets");
        println!("   - Try different tokens from Gamma API");
    } else {
        println!("\n❌ FAILED: No messages received");
        println!("   - Check network connectivity");
        println!("   - Check WebSocket endpoint");
        println!("   - Verify WAF bypass headers");
    }
    
    Ok(())
}