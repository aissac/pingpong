# User WebSocket Implementation - Rust

## From NotebookLM (2026-03-30)

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    HOT PATH (Sync)                           │
│         Detects edge, sends OpportunitySnapshot              │
│         Sub-microsecond latency (0.7µs)                      │
└────────────────────────┬────────────────────────────────────┘
                         │ crossbeam_channel
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              EXECUTION THREAD (Async/Tokio)                 │
│         1. Receives edge detection                           │
│         2. Posts Maker GTC Post-Only order                   │
│         3. Waits for fill_rx channel                         │
│         4. Fires Taker FAK order on MATCHED                  │
└────────────────────────┬────────────────────────────────────┘
                         │ crossbeam_channel
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              USER WS THREAD (Async/Tokio)                   │
│         Listens for order/trade events                       │
│         Sends FillConfirmation on MATCHED status            │
└─────────────────────────────────────────────────────────────┘
```

### 1. WebSocket Endpoint

**URL:** `wss://ws-subscriptions-clob.polymarket.com/ws/user`

SDK constant: `CLOB_WSS_BASE`

### 2. Authentication

Use the same L2 credentials from REST client:
- `apiKey`
- `secret`
- `passphrase`

SDK handles HMAC-SHA256 signing automatically.

### 3. Implementation Code

```rust
use polymarket_client_sdk::stream::{WssUserClient, WssUserEvent};
use polymarket_client_sdk::clob::Client;
use crossbeam_channel::Sender;
use tokio::task;
use futures_util::StreamExt;

// Bridge message from WS thread to execution thread
pub struct FillConfirmation {
    pub order_id: String,
    pub status: String,
}

pub async fn spawn_user_ws_monitor(
    rest_client: &Client,
    fill_tx: Sender<FillConfirmation>
) {
    // 1. Get L2 credentials from REST client
    let creds = rest_client.credentials().clone();

    task::spawn(async move {
        println!("🎧 [USER WS] Connecting to personal trade stream...");

        // 2. Initialize authenticated User WebSocket Client
        let mut user_ws = match WssUserClient::new(creds, None).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("🚨 [USER WS] Connection failed: {}", e);
                return;
            }
        };

        // 3. Subscribe to orders and trades
        if let Err(e) = user_ws.subscribe_orders().await {
            eprintln!("🚨 [USER WS] Failed to subscribe to orders: {}", e);
        }
        if let Err(e) = user_ws.subscribe_trades().await {
            eprintln!("🚨 [USER WS] Failed to subscribe to trades: {}", e);
        }

        println!("✅ [USER WS] Successfully subscribed to live fill events.");

        // 4. Listen for incoming events
        while let Some(msg) = user_ws.next().await {
            match msg {
                Ok(WssUserEvent::Order(order_msg)) => {
                    if order_msg.status == "matched" {
                        println!("⚡ [USER WS] MAKER ORDER MATCHED! ID: {}", order_msg.id);

                        // Send confirmation to Execution Thread
                        let _ = fill_tx.send(FillConfirmation {
                            order_id: order_msg.id,
                            status: order_msg.status,
                        });
                    }
                },
                Ok(WssUserEvent::Trade(trade_msg)) => {
                    // Alternative: monitor trade status == "matched"
                    if trade_msg.status == "matched" {
                        let _ = fill_tx.send(FillConfirmation {
                            order_id: trade_msg.order_id,
                            status: trade_msg.status,
                        });
                    }
                },
                Err(e) => {
                    eprintln!("⚠️ [USER WS] Stream error: {}", e);
                    // Add reconnection logic here
                },
                _ => {}
            }
        }
    });
}
```

### 4. Execution Thread Logic

```rust
// After posting Maker order
let maker_response = client.post_order(signed_maker_order).await?;
let expected_id = maker_response.order_id;

// Wait for MATCHED confirmation with 3-second timeout
if let Ok(fill) = fill_rx.recv_timeout(std::time::Duration::from_millis(3000)) {
    if fill.order_id == expected_id {
        // FIRE TAKER FAK ORDER IMMEDIATELY!
        let taker_response = client.post_order(signed_taker_order).await?;
        println!("🎯 ARBITRAGE EXECUTED COMPLETELY!");
    }
} else {
    // Timeout - Maker order didn't fill
    // Execute cancel_order(expected_id) or Stop-Loss market dump
    println!("⚠️ TIMEOUT - Executing stop-loss");
}
```

### 5. Event Types

**Order Events:**
- `live` - Order resting on book
- `matched` - Order matched by engine (TRIGGER THIS!)
- `canceled` - Order canceled

**Trade Events:**
- `matched` - Sent to relayer for on-chain
- `mined` - Transaction mined
- `confirmed` - Finality reached (too slow, don't wait!)

### 6. Key Points

1. **User WS runs in separate thread** - Never block hot path
2. **Only wait for `matched`** - Don't wait for `mined`/`confirmed`
3. **3-second timeout** - Prevent hanging on ghost orders
4. **crossbeam_channel bridge** - Sync hot path ↔ async WS thread
5. **Reconnection logic** - Handle stream drops gracefully

---

## Implementation Checklist

1. [ ] Add `polymarket-client-sdk` dependency with `stream` feature
2. [ ] Create `src/user_ws.rs` with `spawn_user_ws_monitor()`
3. [ ] Create `FillConfirmation` struct
4. [ ] Add `fill_rx` channel to background thread
5. [ ] Integrate with Maker → Taker flow

---

*Source: NotebookLM conversation 2026-03-30*