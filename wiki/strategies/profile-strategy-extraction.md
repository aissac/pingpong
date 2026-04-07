# Profile Strategy Extraction

**Updated:** 2026-04-07
**Sources:** Jim (@jim_09); NotebookLM Analysis
**Raw:** [2026-04-07-jim-profile-strategy-extraction.md](../../raw/profile-monitoring/2026-04-07-jim-profile-strategy-extraction.md)

---

## Overview

Reverse-engineering trading strategies from on-chain profile `0xe1d6b51521bd4365769199f392f9818661bd907` via continuous AWS monitoring.

**Monitoring Duration:** 2+ hours
**Sample Size:** 295+ trades (statistically sufficient)
**Markets:** BTC/ETH 5-minute and 15-minute Up/Down

---

## Four Strategy Modules Detected

### 1. PRE-ORDER CTF Arbitrage (14% of trades)

**Concept:** Buy both YES and NO below $1.00, merge via CTF contract, redeem for $1.00 risk-free.

| Parameter | Observed | Config |
|-----------|----------|--------|
| Timing | -91s to -4897s before slot | 60s |
| Price | 48-55¢ (avg 52¢) | 45¢ |
| Size | 3.8-387 shares (avg 71.2) | 70 shares |

**Key Finding:** Pre-orders placed MUCH earlier than expected — possibly for different slot windows or early positioning.

**Implementation:**
```rust
// Place limit BUY on BOTH outcomes
if yes_ask + no_ask < 1.00 {
    place_buy_yes(price = 0.52);
    place_buy_no(price = 0.52);
    // If both fill → call merge() → redeem $1.00
}
```

---

### 2. MID-MARKET MAKER (75% of trades)

**Concept:** Spread capture via dynamic quoting with inventory skew.

**5-Minute Slots:**
| Price Zone | % of Trades |
|------------|-------------|
| LOW (1-30¢) | 17% |
| MID (31-70¢) | 34% |
| HIGH (71-99¢) | 47% |

**15-Minute Slots:**
| Price Zone | % of Trades |
|------------|-------------|
| LOW (1-30¢) | 18% |
| MID (31-70¢) | 31% |
| HIGH (71-99¢) | 50% |

**Timing Cluster:** 82% of 5M trades at 180-240s (NOT spread throughout slot)

**Implementation:**
```rust
// Anchor quotes to book edges
bid = best_bid * 0.90;  // 10% below
ask = best_ask * 1.10;  // 10% above

// Inventory skew
if yes_inventory > threshold {
    ask *= 0.98;  // Lower ask to sell YES faster
    bid *= 0.98;  // Discourage buying more YES
}
```

---

### 3. LOW-SIDE LOTTERY (8% of trades)

**Concept:** Resting orders at extreme lows to catch flash crashes.

| Price | Frequency |
|-------|-----------|
| 2¢ | 1 trade |
| 5¢ | 1 trade |
| 10¢ | 1 trade |

**Exit Strategy:**
- Take-profit: 50% at 10¢, 50% at 15¢
- Cancel unfilled before market close

**ROI:** Asymmetric — risk 1-3¢ to gain 97-99¢

---

### 4. HIGH-SIDE MOMENTUM (26% of trades)

**Concept:** Late-slot momentum bets on high-certainty outcomes.

| Metric | Value |
|--------|-------|
| Price Range | 71-99¢ |
| Timing | All >200s into slot |
| Avg Size | 9.3 shares |

**Trigger:** ≥90¢ in last 30s of slot (consensus forming)

---

## Size Analysis

**5-Minute Slots:**
- Min: 1.0 shares
- Max: 54.7 shares
- Mean: 12.6 shares
- Median: 6.9 shares
- **Distribution:** Bimodal (many small, some large)

**15-Minute Slots:**
- Min: 1.0 shares
- Max: 387 shares (outlier)
- Mean: 32.9 shares
- Median: 6.5 shares

---

## Asset Preferences

| Asset | Trades | Avg Size | Price Range |
|-------|--------|----------|-------------|
| BTC | 18 | 34.8 | 5¢ - 98¢ |
| ETH | 13 | 11.1 | 2¢ - 98¢ |
| XRP | 6 | 11.1 | 47¢ - 67¢ |
| SOL | 5 | 15.4 | 11¢ - 95¢ |

**Observation:** BTC gets largest sizes (34.8 avg vs 11-15 for others)

---

## Expected ROI (NotebookLM)

| Strategy | ROI per Trade | Win Rate |
|----------|---------------|----------|
| PRE-ORDER Arb | 2-4% | 75%+ |
| MID-Market Maker | ~0.4% (spread) | 55%+ |
| LOW-Side Lottery | Asymmetric | Low % |

**Risk Parameters:**
- Max per-trade: $10 USDC
- Max Loss Exposure: 5% per market
- Danger price floor: 15¢
- Timeout: Exit unmatched before volatile final minutes

---

## Implementation Status

### AWS Frankfurt Bot (3.77.156.196)

| Strategy | Status | Latency Requirement |
|----------|--------|---------------------|
| PRE_ORDER | ✅ Enabled | 80ms OK |
| HIGH_SIDE | ✅ Enabled | 80ms OK |
| LOTTERY | ✅ Enabled | 80ms OK |
| MID (Maker) | ❌ Disabled | Needs <10ms |

**Mode:** DRY_RUN (paper trading)
**Messages Processed:** 317,000+
**Markets:** 12 (BTC/ETH 5m + 15m)

### GitHub Commits

- `b38bda3` - Add PRE_ORDER and LOTTERY strategies
- `5a51723` - Fix WebSocket message logging

---

## Key Insights

1. **Pre-order timing anomaly** — Observed -4897s to -91s vs config 60s. May indicate multi-slot positioning or early liquidity grabs.

2. **MID-slot clustering** — 75%+ of trades at 180-240s (5M) and 300+s (15M). Suggests weighted regression needing N ticks for confirmation.

3. **HIGH-SIDE is separate strategy** — 26% of trades at 71-99¢, all late-slot. Not part of MID-market maker.

4. **Size bimodality** — Many small trades (1-5 shares) + occasional large (50-387 shares). Suggests confidence-scaling or multi-wallet architecture.

5. **MID (Maker) requires colocation** — 80ms AWS RTT too slow for spread capture. Needs NYCServers (<10ms) for adverse selection protection.

---

## See Also

- [[5m-high-side]] — HIGH-SIDE momentum strategy details
- [[15m-dump-hedge]] — Dump-and-hedge arbitrage
- [[latency]] — Latency requirements for MID strategy
- [[aws-deployment]] — AWS Frankfurt infrastructure
