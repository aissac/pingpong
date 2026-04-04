# Path to Production - NotebookLM Roadmap

## ✅ COMPLETED

1. **Sub-microsecond latency** - 0.7µs avg, <5µs p99 orderbook parsing
2. **Token hash mismatch FIXED** - bytes vs str hashing consistency
3. **Stateful WebSocket parsing** - asset_id at TOP LEVEL only
4. **Edge detection WORKING** - detecting $0.92-$0.97 combined ASK in real-time
5. **DRY RUN mode** - paper trading, no real orders
6. **Ghost simulation** - tracking ~60% ghost rate, ~35% executable
7. **Dynamic token discovery** - Gamma API fetching BTC/ETH 5m/15m markets
8. **Rollover engine** - auto subscribe/unsubscribe markets
9. **GitHub repo** - https://github.com/aissac/polymarket-hft-engine

---

## 🔴 REMAINING FOR LIVE TRADING

### 1. Order Submission (CLOB API)

**What:** Submit orders to `POST /order` REST endpoint

**How:**
- Integrate `polymarket-client-sdk` or `rs-clob-client` Rust crates
- Order payload must include: `tokenID`, `price`, `size`, `side`, `expiration`
- **FAK Orders:** Use "Fill and Kill" time-in-force for Taker leg
  - Fills as many shares as possible immediately
  - Cancels rest (prevents resting on book if liquidity vanishes)

### 2. EIP-712 Signing & Authentication

**Two-tier auth required:**

**L2 Authentication (API Key):**
- HTTP headers: `POLY_ADDRESS`, `POLY_SIGNATURE`, `POLY_TIMESTAMP`, `POLY_API_KEY`, `POLY_PASSPHRASE`
- `POLY_SIGNATURE` = HMAC-SHA256 signature using API secret

**L1 Authentication (EIP-712):**
- Order payload must be signed using EIP-712 with private key
- Authorizes Exchange contract to execute trade without custody

### 3. Wallet Setup & Allowances

**Network:** Polygon

**Currency:** USDC.e (for buys), conditional tokens (for sells)

**Contract Allowances (if using EOA wallet):**
1. Main Exchange: `0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E`
2. Neg Risk Exchange: `0xC5d563A36AE78145C45a50134d48A1215220f80a`
3. Neg Risk Adapter: `0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296`

**Builder Program (Gasless):**
- Use Gnosis Safe proxy wallet (SignatureType 2)
- All operations 100% gas-free via Polymarket relayer

### 4. Maker vs Taker Strategy

**Key insight:** Polymarket removed 500ms delay on taker orders for crypto markets (Feb 2026)

**However:** New March 30 fee structure = up to 1.80% taker fee per leg (3.60% total)

**Optimal Strategy: Hybrid Maker-Taker**
1. Rest `Post-Only` Maker order on thick side → earn 0.36% maker rebate
2. On fill confirmation → instantly cross thin side with `FAK` Taker order

### 5. Fill Confirmation

**Do NOT poll REST API** - too slow

**Solution:** Open second WebSocket connection to `user` channel

**Auth:** Use API credentials to subscribe to:
- `order` events: live, canceled, matched
- `trade` events: matched, mined, confirmed

**Flow:**
1. Maker order fills → `matched` status received
2. Background thread triggers Taker hot path immediately

### 6. Risk Management

**Critical safeguards:**

1. **Stop-Loss / Naked Maker Dump:**
   - If Maker fills but Taker fails → naked directional position
   - Implement 3-second timeout
   - Auto-fire market sell to dump unmatched leg

2. **Inventory Skew Limits:**
   - Hard-cap net exposure
   - If too many YES tokens → adjust quoting to sell YES, buy NO

3. **Rate Limits:**
   - CLOB trading endpoint: 3,500 requests per 10-second burst
   - Handle HTTP 429 with exponential backoff
   - Avoid IP ban

### 7. Colocation

**AWS Frankfurt (`eu-central-1`) is OPTIMAL**

- Polymarket matching engine hosted in Europe
- Network RTT: 30-80ms range
- Combined with 0.7µs processing = competitive advantage

### 8. Other Critical Components

**Dynamic Fee Appending:**
- MUST include `feeRateBps` field in signed order object
- If missing/wrong → order rejected

**CTF Settlement Engine:**
- Background cron for post-market settlement
- Call `redeemPositions` on CTF contract to burn winning tokens
- Claim USDC payout

**Merge Operations:**
- If holding 1 YES + 1 NO for same market → don't wait for resolution
- Call `Merge` on CTF contract anytime
- Burn pair → instantly unlock $1.00 USDC

---

## 📋 CHECKLIST SUMMARY

| Task | Status | Priority |
|------|--------|----------|
| Order submission (POST /order) | ❌ TODO | HIGH |
| EIP-712 signing | ❌ TODO | HIGH |
| L2 API authentication | ❌ TODO | HIGH |
| Wallet + USDC on Polygon | ❌ TODO | HIGH |
| Contract allowances | ❌ TODO | HIGH |
| User WebSocket channel | ❌ TODO | HIGH |
| Fill confirmation handler | ❌ TODO | HIGH |
| FAK Taker orders | ❌ TODO | HIGH |
| Stop-loss / naked dump | ❌ TODO | MEDIUM |
| Inventory skew limits | ❌ TODO | MEDIUM |
| Rate limit handling | ❌ TODO | MEDIUM |
| Dynamic feeRateBps | ❌ TODO | MEDIUM |
| CTF settlement cron | ❌ TODO | LOW |
| Merge operations | ❌ TODO | LOW |

---

## 🎯 NEXT STEPS

1. **Get API credentials** from Polymarket (L2 auth)
2. **Fund wallet** with USDC on Polygon
3. **Set allowances** for 3 contracts
4. **Integrate `rs-clob-client`** for order submission
5. **Implement EIP-712 signing** (use `alloy` crate)
6. **Open User WebSocket** for fill confirmations
7. **Test with small sizes** (1 share)
8. **Scale up** after validation

---

*Source: NotebookLM conversation 2026-03-30*