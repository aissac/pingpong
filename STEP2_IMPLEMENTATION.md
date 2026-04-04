# Step 2 Implementation Guide - Authentication & Execution

## From NotebookLM (2026-03-30)

### 1. Minimum Size Filter

**Dynamic filter based on target trade size:**
```rust
const TARGET_SHARES: u64 = 100; // 100 shares per leg

// In hot path, AFTER tuple extraction, BEFORE math:
if yes_ask_size < TARGET_SHARES || no_ask_size < TARGET_SHARES {
    return; // Fast path rejection - insufficient liquidity
}
```

**Why dynamic?** Share prices fluctuate. 100 shares at $0.10 = $10, at $0.90 = $90.

---

### 2. Authentication Setup

**Two-Level Auth Model:**

| Level | Type | Purpose | Storage |
|-------|------|---------|---------|
| L1 | EIP-712 | Prove wallet ownership | Private key |
| L2 | HMAC | Sign API requests | apiKey, secret, passphrase |

**API Endpoints:**
- Create keys: `POST https://clob.polymarket.com/auth/api-key`
- Fetch keys: `GET https://clob.polymarket.com/auth/derive-api-key`

**Rust Crates:**
```toml
[dependencies]
polymarket-client-sdk = "0.1"
alloy = { version = "0.1", features = ["signers", "signer-local"] }
```

**Code:**
```rust
use alloy::signers::LocalSigner;

let signer = LocalSigner::from_str(&private_key)?
    .with_chain_id(Some(137)); // Polygon chain ID
    
let client = Client::new("https://clob.polymarket.com", Config::default())?
    .authentication_builder(&signer)
    .authenticate().await?; // Handles L1 EIP-712 and gets L2 HMAC keys
```

---

### 3. User WebSocket

**Endpoint:** Connect using `ApiCreds` from L2 setup

**Events to Monitor:**

| Event Type | Status | Meaning |
|------------|--------|---------|
| `order` | `live` | Order resting on book |
| `order` | `matched` | Order matched (not yet on-chain) |
| `trade` | `MATCHED` | Sent to relayer for on-chain |
| `trade` | `MINED` | Transaction mined |
| `trade` | `CONFIRMED` | Finality reached |

**Trigger:** `trade` event with status `MATCHED`
- Don't wait for `CONFIRMED` - too slow for HFT
- `MATCHED` = matching engine paired order, sent to relayer

---

### 4. Order Flow Sequence

```
┌─────────────────────────────────────────────────────────────┐
│                    EDGE DETECTED                             │
│              Combined Ask = $0.95 (5% edge)                  │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              CHECK: Minimum Depth ≥ TARGET_SHARES           │
│              yes_ask_size >= 100 && no_ask_size >= 100      │
└────────────────────────┬────────────────────────────────────┘
                         │ PASS
                         ▼
┌─────────────────────────────────────────────────────────────┐
│           STEP 1: POST MAKER ORDER (GTC Post-Only)          │
│           Thick side (e.g., BUY NO @ $0.4750)               │
│           postOnly: true                                     │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│              WAIT: User WebSocket trade event               │
│              Status = MATCHED                                │
│              (Sent to relayer for on-chain)                 │
└────────────────────────┬────────────────────────────────────┘
                         │ MATCHED
                         ▼
┌─────────────────────────────────────────────────────────────┐
│           STEP 2: POST TAKER ORDER (FAK)                    │
│           Thin side (e.g., BUY YES @ $0.4750)               │
│           Fill-And-Kill: instant fill + cancel rest         │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
            ┌────────────┴────────────┐
            │                         │
            ▼                         ▼
┌───────────────────┐      ┌───────────────────────────────┐
│  FULL FILL       │      │  PARTIAL FILL or FAIL          │
│  ✅ Complete!    │      │  ⚠️ Start 3-sec stop-loss      │
│                   │      │  Market sell unmatched shares  │
└───────────────────┘      └───────────────────────────────┘
```

---

### 5. Order Types

**GTC (Good Till Cancelled) Post-Only:**
```json
{
  "tokenID": "...",
  "price": "0.4750",
  "size": "100",
  "side": "BUY",
  "expiration": "2026-03-30T12:00:00Z",
  "type": "GTC",
  "postOnly": true
}
```

**FAK (Fill And Kill):**
```json
{
  "tokenID": "...",
  "price": "0.4750",
  "size": "100",
  "side": "BUY",
  "type": "FAK"
}
```

---

### 6. Hardware Stop-Loss

If Maker fills but Taker fails:
1. Start 3-second timer
2. If no full fill by timeout → market sell unmatched Maker leg
3. Prevents directional exposure

---

## Implementation Order

1. [ ] Add minimum size filter to hot path
2. [ ] Add `polymarket-client-sdk` dependency
3. [ ] Implement L1 EIP-712 signing
4. [ ] Get L2 HMAC credentials
5. [ ] Open User WebSocket connection
6. [ ] Implement Maker order submission
7. [ ] Implement trade event handler
8. [ ] Implement Taker FAK order
9. [ ] Implement stop-loss timer

---

*Source: NotebookLM conversation 2026-03-30*