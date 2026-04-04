# Live Trading Analysis - NotebookLM Insights

**Date:** 2026-03-30
**Source:** NotebookLM conversation

---

## Current Bot Performance

| Metric | Value | Assessment |
|--------|-------|------------|
| Msg Rate | 731/sec | ✅ Optimal |
| Pair Checks | 11,693/sec | ✅ Top tier |
| Hot Path Latency | 0.7µs | ✅ Elite |
| CPU Utilization | ~0.1% | ✅ Massive headroom |

**Key Insight:** CPU is idle 99.9% of time waiting for network packets. When 50 messages arrive in 1ms, bot processes entire batch before slower bots deserialize JSON.

---

## Edge Frequency Analysis

### Why 0 Edges Now vs 91 Earlier?

**Normal market microstructure - NOT a detection issue.**

Liquidity providers (market makers) widen spreads or pause quoting during:
- Macro-economic data drops
- Sharp spot price movements on Binance/Coinbase
- Volatile opening/closing seconds of prediction window

**The 91 edges occurred during a "dump"** - exactly what Dump-and-Hedge strategy exploits.

**30 seconds with 0 edges** = algorithmic market makers active, book tightly pegged to fair value.

---

## Strategy Recommendations

### Fee Structure (March 30, 2026)

| Strategy | Fee Burden | Optimal Threshold |
|----------|-----------|-------------------|
| Pure Taker | 3.60% (1.80% x 2) | $0.90-$0.92 (High Alpha) |
| Maker-Taker Hybrid | ~1.44% | $0.94 (6% gross edge) |

### Current Distribution Problem

**Current:** Clustered in $0.96-$0.98 (Low Alpha)
**Needed:** Target $0.90-$0.92 for Pure Taker, $0.94 for Maker-Taker

**Action:** Lower threshold before going live.

---

## Ghost Rate Impact (~60%)

### Pure Taker Strategy
- ✅ Protected by FAK (Fill-And-Kill)
- If liquidity vanishes, order fails gracefully
- No naked directional exposure

### Maker-Taker Hybrid
- ⚠️ **HIGH RISK** with 60% ghost rate
- Maker fills → Taker ghosts = NAKED POSITION
- **Requires:** Sub-second hardware stop-loss to dump unmatched leg

---

## Live Trading Readiness Checklist

### Before Production

| Requirement | Status | Notes |
|-------------|--------|-------|
| Directional Exposure Time | ⏳ TODO | Measure Maker→Taker MATCHED delta (must be <50ms) |
| Dynamic feeRateBps | ⏳ TODO | Don't hardcode fees - fetch from API |
| Testnet Validation | ⏳ TODO | Polygon Amoy (Chain ID 80002) |
| EIP-712 Signing | ⏳ TODO | Test signatures on sandbox |
| Order Throughput | ⏳ TODO | Test POST /order on sandbox |
| PnL Calculation | ⏳ TODO | Account for 1.80% Taker fee |

### Testnet Validation Steps

1. Switch to Polygon Amoy Testnet (Chain ID 80002)
2. Get testnet MATIC + testnet collateral
3. Run bot against sandbox liquidity
4. Post Maker order → catch WebSocket fill → fire Taker
5. Verify PnL calculation (including 1.80% fee)
6. **Only then:** Switch to Mainnet

---

## Key Metrics to Watch

### Good Signs
- ✅ Msg Rate > 500/sec
- ✅ Pair Checks > 10,000/sec
- ✅ Sub-ms latency
- ✅ Edges in High/Mid Alpha range

### Warning Signs
- ⚠️ Edges only in Low Alpha (< $0.96)
- ⚠️ Maker→Taker delta > 50ms
- ⚠️ Ghost rate > 70%
- ⚠️ Hardcoded feeRateBps

---

## Fee Math (March 30, 2026)

```
fee = 0.072 × variance¹
Peak: 1.80% @ p=0.5
Lower at extremes: ~0.35% @ p<0.30 or p>0.70

Pure Taker: 1.80% × 2 = 3.60% total
Maker-Taker: 1.80% - 0.36% (rebate) ≈ 1.44% net
```

---

## Recommended Actions

1. **Lower threshold** to $0.94 for Maker-Taker hybrid
2. **Implement stop-loss** if using Maker-Taker (60% ghost risk)
3. **Switch to testnet** before mainnet
4. **Measure Maker→Taker delta** to validate <50ms
5. **Dynamic feeRateBps** - don't hardcode

---

*Source: NotebookLM conversation 2026-03-30*