# Live Trading Implementation Plan

## Overview

This document outlines the complete implementation plan for adding live trading capability to the HFT binary.

## Current State

### ✅ What's Working
- **Edge Detection**: Sub-microsecond latency (1.78µs avg)
- **Token Parsing**: Variable-length token IDs (66-78 chars)
- **Pairing**: 6/6 pairs populated (100%)
- **Transient Filter**: $0.90-$0.94 valid window, no false edges
- **Network**: 0.94ms RTT (colocated in eu-central-1)
- **Safety**: $5 max position, killswitch armed

### ❌ What's Missing
- EIP-712 signing
- Order submission to CLOB
- CTF merge for capital recycling
- Stop-loss for partial fills
- WebSocket user event monitoring

---

## Step 1: EIP-712 Signing ✅

**File**: `src/signing.rs`

**Dependencies** (add to `Cargo.toml`):
```toml
alloy-primitives = "0.8"
alloy-sol-types = "0.8"
alloy-signer = "0.2"
alloy-signer-local = "0.2"
hex = "0.4"
```

**Key Functions**:
- `init_signer(private_key_hex)` - Initialize from private key
- `sign_polymarket_order(order, signer)` - Sign EIP-712 typed data
- `create_order(...)` - Build order struct
- `get_ctf_domain()` - Polymarket EIP-712 domain

**EIP-712 Domain**:
```rust
chain_id: 137  // Polygon
verifying_contract: 0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E
name: "CTFExchange"
version: "1"
```

---

## Step 2: CLOB Order Submission ✅

**File**: `src/execution.rs`

**Dependencies**:
```toml
reqwest = { version = "0.11", features = ["json"] }
base64 = "0.21"
hmac = "0.12"
sha2 = "0.10"
```

**Key Functions**:
- `build_l2_headers(...)` - HMAC-SHA256 authentication
- `submit_order_with_backoff(...)` - POST /order with 429 handling
- `fetch_fee_rate(token_id)` - GET /fee-rate endpoint
- `execute_maker_order(...)` - Resting limit order
- `execute_taker_order(...)` - Immediate fill

**L2 Auth Headers**:
```
POLY_ADDRESS: <signer_address>
POLY_API_KEY: <api_key>
POLY_PASSPHRASE: <passphrase>
POLY_TIMESTAMP: <unix_timestamp>
POLY_SIGNATURE: <hmac-sha256>
```

**HMAC Message**: `timestamp + method + request_path + body`

---

## Step 3: CTF Merge ✅

**File**: `src/merge_worker.rs`

**Key Functions**:
- `run_merge_worker(...)` - Background task with 25 RPM limit
- `fetch_condition_id(slug)` - Get from Gamma API

**Merge Flow**:
1. Wait for both legs to reach `MINED` status
2. Build `mergePositions` calldata
3. Sign Gnosis Safe transaction
4. POST to `https://relayer.polymarket.com/submit`
5. Enforce 2.4s delay between merges (25 RPM)

**CTF Contract**: `0x4D97DCd97eC945f40cF65F87097ACe5EA0476045`
**USDC.e**: `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174`

---

## Step 4: Stop-Loss ✅

**File**: `src/stop_loss.rs`

**Key Functions**:
- `start_stop_loss_timer(...)` - 3-second tokio timer
- `execute_fak_order(...)` - Fill-And-Kill market buy
- `handle_trade_event(...)` - WebSocket event parser
- `record_hedge_pair(...)` - Track order relationships

**Stop-Loss Flow**:
1. Maker fills → start 3-second timer
2. Taker fills → decrement remaining size
3. Timer expires with remaining > 0 → FAK order at $0.99
4. Loss: ~1.44% (fees only) vs 100% directional risk

**FAK Order**:
```json
{
  "orderType": "FAK",
  "expiration": "0",
  ...
}
```

---

## Step 5: User WebSocket Monitor

**File**: `src/user_ws.rs` (TO BE CREATED)

**Endpoint**: `wss://ws-subscriptions-clob.polymarket.com/ws/user`

**Events to Handle**:
- `trade` with `status: "MATCHED"` - Start stop-loss timer
- `trade` with `status: "MINED"` - Trigger CTF merge
- `trade` with `status: "CONFIRMED"` - Update PnL

**Auth Payload**:
```json
{
  "type": "auth",
  "api_key": "...",
  "signature": "...",
  "timestamp": "...",
  "passphrase": "..."
}
```

---

## Step 6: Integration into HFT Binary

**File**: `src/bin/hft_pingpong.rs`

**Architecture**:
```
┌─────────────────────────────────────────────────────┐
│  Hot Path (sync, pinned Core 1)                     │
│  - memchr WebSocket parsing                         │
│  - Edge detection ($0.90-$0.94)                     │
│  - Push OpportunitySnapshot to channel              │
└─────────────────────────────────────────────────────┘
                         │
                         ▼ crossbeam_channel
┌─────────────────────────────────────────────────────┐
│  Background Thread (async, tokio)                   │
│  - Receive OpportunitySnapshot                      │
│  - Execute Maker order (resting limit)              │
│  - Execute Taker order (FAK)                        │
│  - Monitor User WebSocket                           │
│  - Handle stop-loss timers                          │
│  - Queue CTF merges                                 │
│  - Telegram alerts                                  │
└─────────────────────────────────────────────────────┘
```

---

## Configuration Required

### Environment Variables
```bash
# Wallet
POLYMARKET_PRIVATE_KEY=0x...  # EOA private key for signing
POLYMARKET_SAFE_ADDRESS=0x...  # Gnosis Safe proxy address

# L2 API Credentials
POLYMARKET_API_KEY=...
POLYMARKET_API_SECRET=...  # base64 encoded
POLYMARKET_PASSPHRASE=...

# Trading Config
POLYMARKET_MAX_POSITION=5000000  # $5 in micro-USDC
POLYMARKET_KILLSWITCH_DRAWDOWN=3  # -3% halt
POLYMARKET_MODE=production  # or "dry_run"
```

### Token Allowances (EOA/MetaMask)
```bash
# Approve USDC.e on Exchange contract
cast send --private-key $PK 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  "approve(address,uint256)" \
  0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E \
  115792089237316195423570985008687907853269984665640564039457584007913129639935

# Approve USDC.e on CTF Exchange
cast send --private-key $PK 0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  "approve(address,uint256)" \
  0x4D97DCd97eC945f40cF65F87097ACe5EA0476045 \
  115792089237316195423570985008687907853269984665640564039457584007913129639935

# Approve Conditional Tokens on CTF Exchange
cast send --private-key $PK 0x4D97DCd97eC945f40cF65F87097ACe5EA0476045 \
  "setApprovalForAll(address,bool)" \
  0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E \
  true
```

---

## Testing Checklist

### Dry Run Mode
- [ ] EIP-712 signing works with test key
- [ ] Order submission returns valid order_id
- [ ] User WebSocket connects and authenticates
- [ ] Stop-loss timer triggers correctly
- [ ] CTF merge queue processes without 429s
- [ ] Telegram alerts fire on events

### Production Mode
- [ ] Wallet funded with $300-500 USDC.e
- [ ] Token allowances approved
- [ ] API credentials validated
- [ ] First arbitrage executes successfully
- [ ] CTF merge recycles capital
- [ ] PnL tracking accurate

---

## Rate Limits

| Endpoint | Limit | Handling |
|----------|-------|----------|
| CLOB Orders | 3500/10sec burst | Exponential backoff |
| CLOB Sustained | 36000/10min | Queue management |
| Relayer (Merges) | 25/min | 2.4s delay between |
| User WebSocket | Persistent | Auto-reconnect |

---

## Error Handling

### 429 Rate Limit
```rust
// Exponential backoff: 100ms, 200ms, 400ms
let backoff_ms = 2u64.pow(retries) * 100;
```

### Order Rejection
- Log error with full payload
- Retry with adjusted parameters
- Alert via Telegram if persistent

### Merge Failure
- Re-queue task
- Check nonce synchronization
- Alert if nonce gap detected

---

## Monitoring Metrics

### First Hour
- Relayer RPM (watch for 429s)
- Live ghost rate vs 62% simulated
- Stop-loss slippage (should be <3%)
- Fill rate ( Maker vs Taker)

### Ongoing
- PnL per trade
- Capital velocity (merges/hour)
- Edge capture rate
- Killswitch status

---

## Rollback Plan

If issues detected:
1. Set `POLYMARKET_MODE=dry_run`
2. Restart service
3. Investigate logs
4. Fix and redeploy

---

## Next Steps

1. **Add dependencies to Cargo.toml**
2. **Create src/signing.rs** ✅ DONE
3. **Create src/execution.rs** ✅ DONE
4. **Create src/merge_worker.rs** ✅ DONE
5. **Create src/stop_loss.rs** ✅ DONE
6. **Create src/user_ws.rs** (next)
7. **Integrate into src/bin/hft_pingpong.rs**
8. **Update Cargo.toml with all deps**
9. **Test in dry_run mode**
10. **Deploy to production**

---

## Files Created This Session

```
/home/aissac/.openclaw/workspace/
├── signing.rs          # EIP-712 signing
├── execution.rs        # CLOB order submission
├── merge_worker.rs     # CTF merge with 25 RPM
├── stop_loss.rs        # 3-second stop-loss
└── LIVE_TRADING_PLAN.md # This document
```

**Next**: Create `src/user_ws.rs` for WebSocket monitoring.
