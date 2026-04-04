## 🔴 CRITICAL BUG: WebSocket Disconnecting

### The Problem

You were RIGHT to be skeptical. The orderbook state isn't frozen - **the binary keeps crashing and restarting**.

**Evidence:**
```
[HFT] Processed 546390 messages in 1742.26859975s
```

The binary crashed after **29 minutes** (546,390 messages), then restarted fresh.

**Orderbook state changed:**
- Before crash: 7 with both bid+ask
- After restart: 4 with both bid+ask

### Root Cause

The WebSocket connection is closing after ~29 minutes. When `read()` returns `Ok(0)`, the hot path exits.

**Code path:**
```rust
let n = match ws_stream.read(&mut buffer[total_bytes..]) {
    Ok(0) => break,  // WebSocket closed - exits
    Ok(n) => n,
    Err(_) => break, // Error - exits
};
```

### Why This Matters

1. Every restart loses accumulated orderbook state
2. 29 minutes of orderbook updates = gone
3. Fresh restart = incomplete orderbooks (4 vs 7)
4. No edges detected because state keeps resetting

### Solution

Need to add WebSocket reconnection logic. When the connection closes, the hot path should reconnect instead of exiting.

### Temporary Fix

Restart the binary periodically (every 20 minutes) to keep orderbooks fresh, until reconnection is implemented.