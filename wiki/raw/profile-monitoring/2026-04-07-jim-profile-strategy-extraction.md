# Profile Strategy Extraction Session - April 7, 2026

**Source:** Telegram session with Jim (@jim_09, user ID: 1047985696)
**Collected:** 2026-04-07
**Published:** Unknown
**Session ID:** 94c04a27-2515-4d54-8c34-2e0fc301f405

---

## Monitoring Setup

**Profile:** `0xe1d6b51521bd4365769199f392f9818661bd907`
**Location:** AWS Frankfurt (3.77.156.196)
**Started:** 2026-04-07 12:01 UTC
**Runtime:** 2+ hours
**Total Trades:** 295+

---

## Strategy Modules Detected

### PRE-ORDER MODULE (14% of trades)
- **Timing:** -4897s to -91s before slot (avg -1894s)
- **Price:** 48-55¢ (avg 52¢)
- **Size:** 3.8 to 387 shares (avg 71.2)
- **Behavior:** Places orders MUCH earlier than expected config

### MID-MARKET MAKER (75% of trades)
- **5M trades:** 82% during slot
  - LOW (1-30¢): 17%
  - MID (31-70¢): 34%
  - HIGH (71-99¢): 47%
- **15M trades:** 88% during slot
  - LOW (1-30¢): 18%
  - MID (31-70¢): 31%
  - HIGH (71-99¢): 50%

### LOW-SIDE LOTTERY (8% of trades)
- **Prices:** 2¢, 5¢, 10¢
- **Behavior:** Flash crash captures

### HIGH-SIDE MOMENTUM (26% of trades)
- **Timing:** All late-slot (>200s)
- **Avg size:** 9.3 shares
- **Price:** 71-99¢

---

## Size Analysis

| Slot | Min | Max | Mean | Median |
|------|-----|-----|------|--------|
| 5M | 1.0 | 54.7 | 12.6 | 6.9 |
| 15M | 1.0 | 387 | 32.9 | 6.5 |

---

## Asset Preferences

| Asset | Trades | Avg Size |
|-------|--------|----------|
| BTC | 18 | 34.8 |
| ETH | 13 | 11.1 |
| XRP | 6 | 11.1 |
| SOL | 5 | 15.4 |

---

## Key Findings

1. **Pre-orders placed EARLIER than config** - config says 60s, observed -91s to -4897s
2. **5M trades cluster at 180-240s** - not spread throughout slot
3. **HIGH price trades = separate strategy** - late-slot momentum plays
4. **Size bimodal** - many small (1-5), some very large (50+)

---

## Confirmed Parameters

- Pre-order price: 52¢ (observed) vs 45¢ (config)
- MID-slot dominance: 75%+ of trades
- LATE timeout: 2% (exit ~240s for 5M)
- Danger exit: Not observed yet

---

## NotebookLM Query Results

### Monitoring Duration
- **Goal:** 100-200 completed trades across multiple market regimes
- **Current status:** 295 trades (SUFFICIENT!)
- **Market conditions needed:** Calm, trending, headline shocks

### Implementation Blueprint

**Core Modules:**
- REST API + WebSocket clients
- Order Signer (EIP-712)
- Trading Logic Director (central orchestrator)
- Inventory & Risk Manager

**Pre-Order CTF Arb:**
- Place limit BUY on BOTH Up and Down at combined < $1.00
- If both fill, call CTF merge function
- Redeems to 1.00 USDC instantly

**Mid-Market Maker:**
- Anchor quotes to book edges
- Inventory-based skew

**Low-Side Lottery:**
- Entry: 1¢, 2¢, 3¢ resting orders
- Exit: 50% at 10¢, 50% at 15¢

### Expected ROI

| Strategy | ROI | Win Rate |
|----------|-----|----------|
| Pre-Order Arb | 2-4% per trade | 75%+ |
| Mid-Market Maker | ~0.4% spread | 55%+ |
| Low-Side Lottery | Asymmetric | Low % |

---

## Bot Deployment Status

**AWS Frankfurt:** Running
**Mode:** DRY_RUN=true
**Messages processed:** 317,000+
**Markets:** 12 (BTC/ETH 5m + 15m)
**Log:** `/tmp/paper_trade.log`

### Strategies Enabled
- ✅ PRE_ORDER
- ✅ HIGH_SIDE
- ✅ LOTTERY
- ❌ MID (Maker) - needs <10ms latency, AWS has ~80ms

---

## GitHub Commits

- `b38bda3` - Add PRE_ORDER and LOTTERY strategies
- `5a51723` - Fix WebSocket message logging
