# Final Integration Architecture - Pingpong HFT Bot

## From NotebookLM (2026-03-30)

### Complete Architecture

```
┌──────────────────────────────────────────────────────────────────────────┐
│                          STARTUP SEQUENCE                                 │
│                      (STRICTLY SEQUENTIAL)                               │
└──────────────────────────────────────────────────────────────────────────┘

1. INITIALIZE L1/L2 AUTHENTICATION
   └─> REST API authentication → derive L2 credentials

2. START USER WS THREAD
   └─> Connect to `user` channel → wait for subscription confirmation

3. START EXECUTION THREAD
   └─> Spawn Tokio task → listen for edges from hot path

4. START HOT PATH (Market WS)
   └─> Only after User WS + Execution thread signal READY
   └─> Begin parsing book_snapshot and price_change events
```

---

### Thread Architecture (Sync ↔ Async Bridge)

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     HOT PATH (Sync, No Tokio)                           │
│                                                                          │
│  • Sub-microsecond latency (0.7µs)                                       │
│  • Parses WebSocket bytes with memchr                                   │
│  • Detects edge when Combined Ask < $0.98                               │
│  • Calls tx.send(OpportunitySnapshot) - NON-BLOCKING                    │
│                                                                          │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 │ crossbeam_channel::unbounded
                                 │ (or tokio::sync::mpsc::unbounded_channel)
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                   EXECUTION THREAD (Async/Tokio)                       │
│                                                                          │
│  • Waits for edges from rx.recv().await                                 │
│  • Posts Maker GTC Post-Only order via REST                             │
│  • Waits for fill_rx.recv_timeout(3 seconds)                           │
│  • On MATCHED: Fires Taker FAK order                                    │
│  • On TIMEOUT: Cancels Maker, triggers Stop-Loss                        │
│                                                                          │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                                 │ crossbeam_channel
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                   USER WS THREAD (Async/Tokio)                         │
│                                                                          │
│  • Connects to wss://ws-subscriptions-clob.polymarket.com/ws/user      │
│  • Subscribes to `orders` and `trades` channels                         │
│  • On order.status == "matched": sends FillConfirmation                │
│  • On disconnect: exponential backoff + PAUSE signal to hot path       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

### Error Handling Matrix

| Error | Action |
|-------|--------|
| **User WS Disconnects** | Exponential backoff reconnection + send PAUSE signal to hot path |
| **Maker Times Out (3s)** | DELETE /order to cancel, wait for confirmation |
| **Maker Partial Fill** | Proceed to Stop-Loss for filled portion only |
| **Taker FAK Fails** | Trigger Stop-Loss (market sell) |
| **Taker Partial Fill** | Trigger Stop-Loss for unfilled portion |

---

### Stop-Loss Logic (Market Sell)

Polymarket has no native "Market Order" - it's a limit order crossing the spread.

**How to execute:**
```rust
// Bot holds 100 YES tokens after failed Taker leg
let stop_loss = client.limit_order()
    .token_id(yes_token_id)
    .price(dec!(0.01))      // Lowest valid tick
    .size(dec!(100))
    .side(Side::Sell)        // SELL to exit position
    .time_in_force("FAK")    // Fill-And-Kill
    .build()
    .await?;

client.post_order(signed_stop_loss).await?;
```

The matching engine fills against highest resting bids until 100 shares sold.

---

### DRY RUN vs LIVE (Testnet)

**Polymarket Sandbox Environment:**
- Network: Polygon Amoy Testnet
- Chain ID: `80002`
- Faucet: Get testnet MATIC + testnet collateral

**Steps:**
1. Change RPC to Amoy testnet
2. Set chain ID to `80002`
3. Use Amoy CLOB/Gamma API endpoints
4. Run bot with real signatures (no real money at risk)
5. Validate full flow: Edge → Maker → MATCHED → Taker
6. Once validated → switch to Polygon Mainnet

---

### Implementation Checklist

#### Phase 1: Core Infrastructure
- [ ] Add `polymarket-client-sdk` with `stream` feature to Cargo.toml
- [ ] Create `src/auth.rs` - L1/L2 authentication
- [ ] Create `src/user_ws.rs` - User WebSocket thread
- [ ] Create `src/execution.rs` - Execution thread

#### Phase 2: Thread Bridges
- [ ] Create `FillConfirmation` struct
- [ ] Create `OpportunitySnapshot` struct (already exists)
- [ ] Add `crossbeam_channel::unbounded` between hot path and execution
- [ ] Add `crossbeam_channel` between execution and user_ws

#### Phase 3: Order Flow
- [ ] Implement Maker GTC Post-Only order
- [ ] Implement Taker FAK order
- [ ] Implement 3-second timeout
- [ ] Implement order cancellation (DELETE /order)

#### Phase 4: Error Handling
- [ ] User WS reconnection with exponential backoff
- [ ] PAUSE signal to hot path on WS disconnect
- [ ] Stop-Loss market sell logic
- [ ] Partial fill handling

#### Phase 5: Testing
- [ ] Test on Amoy testnet
- [ ] Validate edge detection still works
- [ ] Validate Maker → MATCHED → Taker flow
- [ ] Validate Stop-Loss on Taker failure
- [ ] Switch to Mainnet

---

### Key Points

1. **Startup is sequential** - Never start hot path before WS is ready
2. **Lock-free channels** - Use unbounded to never block
3. **3-second timeout** - Don't hang on ghost orders
4. **Stop-Loss = FAK Sell @ $0.01** - Cross spread to exit
5. **Testnet first** - Amoy testnet validates flow without real money

---

*Source: NotebookLM conversation 2026-03-30*