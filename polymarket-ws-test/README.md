# Phase 3 WebSocket Integration Guide

## Step 1: Verify WebSocket Subscription ✅

**CRITICAL FIX:** Use `"assets"` NOT `"assets_ids"`

```json
{
  "assets": ["<TOKEN_ID_1>", "<TOKEN_ID_2>"],
  "type": "market"
}
```

### Test Script Ready

File: `src/bin/ws_verify.rs`

This script:
1. Fetches live tokens from Gamma API (`https://gamma-api.polymarket.com/markets?active=true&limit=5`)
2. Connects to `wss://ws-subscriptions-clob.polymarket.com/ws/market`
3. Subscribes with **correct** format: `{"assets": [...], "type": "market"}`
4. Listens for 30 seconds and validates:
   - Initial snapshots (`"event": "book"`)
   - Price updates (`"event": "price_change"`)

### Run on AWS

```bash
# SSH into AWS
ssh ubuntu@100.64.0.2

# Navigate to project
cd /home/ubuntu/polymarket-market-maker

# Copy test file from this directory
# (If needed: scp -r ~/workspace/polymarket-ws-test ubuntu@100.64.0.2:/tmp/)

# Build and run
cargo build --release --bin ws_verify
./target/release/ws_verify
```

### Expected Output

```
🚀 Phase 3 Step 1: WebSocket Verification

📡 Step 1: Fetching live tokens from Gamma API...
✅ Fetched 5 active markets

📊 Test tokens:
   1. Will Bitcoin reach $100k by... (ID: 1535318560435384...)
   2. Will ETH flip BTC in 2026? (ID: 8923640981726341...)

🔌 Step 2: Connecting to Polymarket WebSocket...
✅ Connected! Status: 101 Switching Protocols

📝 Step 3: Subscribing with assets format...
   Sending: {"assets":["1535318560435384...","8923640981726341..."],"type":"market"}
✅ Subscription sent!

📥 Step 4: Listening for orderbook updates (30 seconds)...

📨 Message 1: type=Some("market"), event=Some("book")
   ✅ INITIAL SNAPSHOT
   Asset: Some("1535318560435384...")
   Market: Some("bitcoin-100k")

📨 Message 2: type=Some("market"), event=Some("price_change")
   ✅ PRICE UPDATE
   Asset: Some("1535318560435384...")

📊 VERIFICATION RESULTS:
   Messages received: 15
   Orderbook updates: 15

✅ SUCCESS: WebSocket subscription working!
   - Correct format: {"assets": [...], "type": "market"}
   - Key must be "assets" NOT "assets_ids"
   - Receiving market events (book/price_change)
   - Ready to integrate into main bot
```

### Key Learnings from NotebookLM

1. **Field Name Matters:** Use `"assets"` not `"assets_ids"` - server will drop connection otherwise
2. **Token IDs:** Must use ERC1155 token IDs (long numeric strings), not `condition_id` (hex strings)
3. **Event Types:** Expect `"event": "book"` (initial snapshot) and `"event": "price_change"` (updates)
4. **WAF Bypass:** Headers required: `User-Agent: Mozilla/5.0` and `Origin: https://polymarket.com`

## Next Steps

Once Step 1 passes:

1. ✅ **Step 1:** WebSocket subscription verified
2. ⏳ **Step 2:** Integrate token discovery into `main.rs`
3. ⏳ **Step 3:** Wire WebSocket → crossbeam channel → hot_path
4. ⏳ **Step 4:** Test end-to-end with discovered tokens
5. ⏳ **Step 5:** Verify orderbook updates flowing

## Integration into Main Bot

After verification, we'll integrate:

```rust
// src/network/ws_engine.rs
pub async fn run_websocket(
    token_ids: Vec<String>,
    event_tx: crossbeam::Sender<EngineEvent>,
) -> Result<(), Error> {
    // Connect with WAF bypass headers
    let request = Request::builder()
        .uri("wss://ws-subscriptions-clob.polymarket.com/ws/market")
        .header("User-Agent", "Mozilla/5.0...")
        .header("Origin", "https://polymarket.com")
        .body(())?;
    
    let (ws_stream, _) = connect_async(request).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    // Subscribe with CORRECT format
    let subscription = json!({
        "assets": token_ids,  // CRITICAL: "assets" not "assets_ids"
        "type": "market"
    });
    ws_sender.send(Message::Text(subscription.to_string())).await?;
    
    // Stream events to hot path
    while let Some(Ok(msg)) = ws_receiver.next().await {
        if let Message::Text(text) = msg {
            // Parse and send to hot_path via channel
            if let Ok(event) = parse_orderbook_update(&text) {
                event_tx.send(event)?;
            }
        }
    }
    Ok(())
}
```

## NotebookLM Session

Keep NotebookLM in loop: `notebooklm ask "Step 1 complete, ready for Step 2..." --notebook 857bce48-57e6-4ee7-a621-6fe8a588a239`