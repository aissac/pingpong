# Top-of-Book Market Maker - Implementation Plan

## Strategy Overview

**Pivot from:** Pre-Order Merge Trap (100% ghosts in calm markets)  
**Pivot to:** Top-of-Book Market Making (constant fills, rebate farming)

---

## Core Mechanics

| Parameter | Value |
|-----------|-------|
| **Markets** | 1-4 simultaneous (BTC/ETH 15m) |
| **Spread** | 0.49/0.51 (1-tick inside best bid/ask) |
| **Refresh Rate** | 1000ms (1 second) |
| **Max Inventory** | 1,000 shares ($500) per side |
| **Skew Threshold** | 300 shares ($150) imbalance |
| **Maker Rebate** | 0.36% per fill |
| **Net per 100-share round-trip** | +$2.36 ($2.00 spread + $0.36 rebates) |
| **Expected Hourly** | $23-35/market (10-15 round trips/hr) |
| **Capital Required** | $1,500 min, $10,000 optimal |

---

## PnL Math

**Per 100-share round-trip:**
- Buy 100 @ $0.49 = $49.00
- Sell 100 @ $0.51 = $51.00
- Spread capture: +$2.00
- Maker rebates: $49×0.36% + $51×0.36% = +$0.36
- **Net: +$2.36 per 100 shares**

**Hourly projection (1 market):**
- 10-15 round trips × $2.36 = $23.60-35.40/hr
- 4 markets = $94-142/hr
- Daily (24h) = $2,256-3,408

---

## Inventory Skew Control

```
If YES_inventory > NO_inventory + 300 shares:
  - Drop YES_Ask to 0.48 (aggressive sell)
  - Drop YES_Bid to 0.47 (stop buying)

If NO_inventory > YES_inventory + 300 shares:
  - Drop NO_Ask to 0.48 (aggressive sell)
  - Drop NO_Bid to 0.47 (stop buying)

If YES >= 500 AND NO >= 500:
  - CTF merge 500 pairs → $500 USDC (free collateral release)
```

---

## Risk Management

| Risk | Mitigation |
|------|------------|
| **Toxic flow** | Spot delta >0.15% in 30s → CancelAll + 60s cooldown |
| **Adverse selection** | >500 shares one-sided in <2s → 15s halt |
| **Rate limit** | Token bucket: 50 burst, 5/s sustained (3000/10min) |
| **Max exposure** | $1,000/market × 4 markets = $4,000 total |
| **Stop-loss** | Volatility halt, not fixed USD loss |

---

## Implementation Steps

### Step 1: Create Modules (DONE)
- `src/market_maker.rs` - Core MM state, quotes, inventory
- `src/pnl_tracker.rs` - Rebate tracking, PnL calculation
- `src/lib.rs` - Export new modules

### Step 2: Create Main Binary
- `src/bin/market_maker.rs` - Main entry point
- WebSocket integration (existing price_cache)
- CLOB API integration (place, cancel, post-only)
- 1-second refresh loop

### Step 3: Dry-Run (48 hours)
- Validate skew response (mock 500 YES injection → quotes adjust in 1s)
- Validate rate limiting (≤1 cancel/replace per second per market)
- Validate WAF bypass (User-Agent + Origin headers)

### Step 4: Live Deployment
- Fund wallet: $5,000 USDC + 5 POL (gas)
- Deploy to 1 market first (BTC 15m)
- Scale to 4 markets after 7 days

---

## Files to Create

1. `src/market_maker.rs` - Core MM logic
2. `src/pnl_tracker.rs` - PnL tracking
3. `src/bin/market_maker.rs` - Main binary
4. `src/order_manager.rs` - CLOB order placement
5. `src/inventory_tracker.rs` - YES/NO balance tracking

---

## NotebookLM Validation

**Date:** 2026-04-02  
**Session:** clear-slug  
**Verdict:** ✅ VALIDATED

> "This architecture leverages the new March 30, 2026 fee structure to capture both the bid-ask spread and the lucrative LP rebates, while actively managing the directional inventory risk that destroys naive AMMs."

**Key insight:** Pre-Order Merge Trap requires PANIC (rare). Top-of-Book MM works in CALM markets (constant fills).

---

## Next Actions

1. ✅ Create market_maker.rs
2. ✅ Create pnl_tracker.rs
3. ⏳ Create market_maker binary
4. ⏳ 48-hour dry-run
5. ⏳ Live deployment ($5,000 USDC)

---

**Status:** MODULES CREATED, BINARY PENDING
