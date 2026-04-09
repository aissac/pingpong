# 🏓 COMPLETE TRADING STRATEGIES
## Based on Profile 0xe1d6b51521bd4365769199f392f9818661bd907 Analysis

**Data Source:** 683 trades analyzed from profile_monitor.log  
**Validation:** 95% match to NotebookLM cloned profile  
**Date:** April 9, 2026

---

## 📊 STRATEGY OVERVIEW

| Slot | % of Trades | Primary Strategy | Secondary Strategy | Entry Window |
|------|-------------|------------------|-------------------|--------------|
| **5M** | 59.4% | HIGH-SIDE (≥90¢) | MID-MARKET (35-65¢) | Last 30-60s |
| **15M** | 26.9% | MID-MARKET (35-65¢) | PRE-ORDER (45¢) | Middle 60% |
| **1H** | 13.0% | HIGH-SIDE (≥85¢) | DUMP-HEDGE | Last 5-10min |

---

## 🔵 5-MINUTE SLOT STRATEGY

### **PRIMARY: HIGH-SIDE MOMENTUM** (73% of 5M trades)

**When to Enter:**
- **Timing:** Last 30-60 seconds of slot (240-300s elapsed)
- **Price:** ≥90¢ (extreme probability)
- **Side:** BUY the side at ≥90¢ (YES if yes_bid≥90, NO if no_bid≥90)

**Position Sizing:**
```
Base: 5-50 shares
Observed range: 0.6 - 130 shares (avg: 12.9)
Formula: min(50, capital × 0.5% × setup_score / price)
```

**Setup Score (0.0 - 1.0):**
```
spread_score = (1.0 - (spread_bps / 500.0)) × 0.30
depth_score = min(1.0, depth_usd / 50000) × 0.25
flow_score = min(1.0, net_buy_flow / 5000) × 0.25
time_score = (1.0 - (seconds_remaining / 300.0)) × 0.20
setup_score = spread_score + depth_score + flow_score + time_score
```

**Exit:**
- Hold until resolution (slot ends)
- Expected value: 90¢ → $1.00 = +11% gross
- Net after fees (1.80% taker): ~9% profit

**Filters:**
- ✅ Liquidity ≥ $10,000
- ✅ Spread ≤ 2% (200 bps)
- ✅ No 8%+ spike in last 3 seconds (anti-chasing)
- ✅ Probability 65%-96% (avoid extremes <65¢ or >96¢)

---

### **SECONDARY: MID-MARKET MTM** (23% of 5M trades)

**When to Enter:**
- **Timing:** 180-240s elapsed (middle 60% of slot)
- **Price:** 35-65¢ both sides (uncertain market)
- **Combined Cost:** <97¢ (arbitrage opportunity)

**Position Sizing:**
```
Both sides: YES + NO
Size: 10-80 shares per side (avg: 17)
Formula: capital × 0.5% × setup_score / combined_cost
```

**Exit:**
- Mark-to-market as probability shifts
- Exit when one side reaches ≥85¢ (+30-50% gain)
- Or hold both to resolution (one wins, one loses)

**Filters:**
- ✅ Combined cost <97¢ (guarantees profit if held)
- ✅ Liquidity ≥ $5,000 per side
- ✅ Duration ≤ 14 days (for longer markets)

---

### **TERTIARY: PRE-ORDER** (4% of 5M trades)

**When to Enter:**
- **Timing:** 90-95 seconds BEFORE slot starts
- **Price:** 40-60¢ both sides (pre-market uncertainty)
- **Trigger:** Both YES and NO available at ≤60¢

**Position Sizing:**
```
Both sides: 70 shares each (profile observed)
Formula: Fixed size for pre-order (lower risk)
```

**Exit:**
- Hold through slot start
- Exit when one side moves to ≥80¢ (+33% gain)
- Or hedge when combined cost >100¢

---

## 🟡 15-MINUTE SLOT STRATEGY

### **PRIMARY: MID-MARKET MEAN REVERSION** (75% of 15M trades)

**When to Enter:**
- **Timing:** 180-720s elapsed (middle 60% of 900s slot)
- **Price:** 35-65¢ (sweet spot 35-62¢ per NotebookLM)
- **Side:** Based on order flow direction

**Position Sizing:**
```
Base: 5-80 shares
Observed range: 4.0 - 83.2 shares (avg: 17.0)
Formula: capital × 0.5% × setup_score / price
```

**Inventory Skew Management:**
```
If YES position ≥ 30% of portfolio:
  → Lower bid and ask by 2¢ (encourage YES sells)
If NO position ≥ 30% of portfolio:
  → Raise bid and ask by 2¢ (encourage NO sells)
Max skew: 30% imbalance allowed
```

**Exit:**
- MTM exit when one side reaches ≥80¢
- Or hold to resolution if confident

**Filters:**
- ✅ Net buy flow ≥ $500 (directional confirmation)
- ✅ No 15%+ drop in last 3 seconds (dump detection)
- ✅ Correlated exposure ≤ 25% (BTC/ETH cluster limit)

---

### **SECONDARY: DUMP-HEDGE** (12% of 15M trades)

**When to Enter (LEG1):**
- **Trigger:** ≥15% price drop in 3 seconds
- **Timing:** First 120 seconds of slot
- **Side:** BUY the crashed side (contrarian)

**Position Sizing (LEG1):**
```
Size: 50% of normal position
Formula: capital × 0.25% × setup_score / price
```

**When to Enter (LEG2):**
- **Timing:** 30-90 seconds after LEG1
- **Side:** OPPOSITE of LEG1 (hedge)
- **Price:** Whatever is available

**Position Sizing (LEG2):**
```
Size: Match LEG1 dollar amount
Formula: LEG1_shares × LEG1_price / LEG2_price
```

**Exit:**
- Combined cost ≤ $0.95 (guaranteed profit)
- Or force exit after 5 minutes if hedge not filled

**Circuit Breaker:**
- ❌ Skip if combined cost > $1.20 (stop-loss cap)
- ❌ Skip if >8% move in last 3 seconds (anti-chasing)

---

### **TERTIARY: PRE-ORDER** (13% of 15M trades)

**When to Enter:**
- **Timing:** 60 seconds BEFORE slot starts
- **Price:** ≤45¢ both sides
- **Trigger:** Both YES and NO available at ≤45¢

**Position Sizing:**
```
Both sides: 70 shares each (profile observed)
Total commitment: ~$63 (45¢ × 70 × 2)
```

**Exit:**
- Hold through slot start
- Exit when one side reaches ≥75¢ (+66% gain)
- Or hedge when combined cost >100¢

---

## 🟠 1-HOUR SLOT STRATEGY

### **PRIMARY: HIGH-SIDE MOMENTUM** (60% of 1H trades)

**When to Enter:**
- **Timing:** Last 5-10 minutes of slot (3000-3600s elapsed)
- **Price:** ≥85¢ (slightly lower threshold than 5M)
- **Side:** BUY the side at ≥85¢

**Position Sizing:**
```
Base: 10-100 shares (larger due to longer duration)
Formula: capital × 1.0% × setup_score / price (2x normal allocation)
```

**Exit:**
- Hold until resolution
- Expected value: 85¢ → $1.00 = +17.6% gross
- Net after fees: ~15% profit

**Filters:**
- ✅ Liquidity ≥ $25,000 (higher for longer markets)
- ✅ No 5%+ spike in last 10 seconds
- ✅ Volume ≥ $100K in last hour

---

### **SECONDARY: DUMP-HEDGE** (30% of 1H trades)

**When to Enter (LEG1):**
- **Trigger:** ≥10% price drop in 10 seconds (slower threshold than 15M)
- **Timing:** Anytime during slot
- **Side:** BUY the crashed side

**Position Sizing (LEG1):**
```
Size: 25% of normal position
Formula: capital × 0.25% × setup_score / price
```

**When to Enter (LEG2):**
- **Timing:** 2-5 minutes after LEG1 (longer window than 15M)
- **Side:** OPPOSITE of LEG1

**Exit:**
- Combined cost ≤ $0.97 (slightly higher tolerance for 1H)
- Or force exit after 15 minutes

---

### **TERTIARY: MID-MARKET** (10% of 1H trades)

**When to Enter:**
- **Timing:** 15-45 minutes into slot (middle 50%)
- **Price:** 40-60¢ both sides
- **Combined Cost:** <95¢

**Position Sizing:**
```
Both sides: 20-50 shares each
Formula: capital × 0.5% × setup_score / combined_cost
```

**Exit:**
- MTM when one side reaches ≥75¢
- Or hold both to resolution

---

## 🛡️ RISK MANAGEMENT (ALL SLOTS)

### **Circuit Breakers**

| Trigger | Action |
|---------|--------|
| Daily loss ≥ 3% | HALT trading for 24h |
| Weekly loss ≥ 8% | HALT trading for 7 days |
| Single market loss ≥ 5% | EXIT position immediately |
| Portfolio loss ≥ 15% | EMERGENCY EXIT all positions |
| Extreme price (bid ≤2¢ or ask ≥98¢) | SKIP market |

### **Position Limits**

| Limit | Value |
|-------|-------|
| Max per trade | 10% of capital |
| Max per market | 5% of capital |
| Max correlated (BTC/ETH cluster) | 25% of capital |
| Max inventory skew | 30% imbalance |

### **Stop-Loss Rules**

| Situation | Action |
|-----------|--------|
| Combined cost > $1.20 | Forced hedge (cap loss at 20¢/share) |
| Price drops >15% in 3s | Dump-hedge LEG1 trigger |
| Price drops >30% in 10s | Emergency exit |
| Time to resolution <30s | No new entries |

---

## 📈 STRATEGY PERFORMANCE TARGETS

| Metric | Target | Profile Actual |
|--------|--------|----------------|
| Win rate | ≥75% | 78.6% ✅ |
| Avg profit per trade | ≥5¢ | ~6-8¢ ✅ |
| Max drawdown | <15% | ~12% ✅ |
| Trades per hour | 20-40 | ~28 ✅ |
| Fill rate | >35% | ~40% ✅ |

---

## 🔧 IMPLEMENTATION PRIORITY

1. ✅ **5M HIGH-SIDE** - Already working (26.1% of trades)
2. ✅ **15M MID-MARKET** - Already working (23.9% of trades)
3. ⚠️ **PRE-ORDER (5M/15M)** - Add 60-95s before slot
4. ⚠️ **DUMP-HEDGE (15M/1H)** - Add 15% drop detection
5. ⚠️ **Inventory Skew** - Add rebalancing logic
6. ⚠️ **Circuit Breakers** - Add halt conditions
7. ⚠️ **Dynamic Position Sizing** - Replace fixed size with formula

---

**Last Updated:** April 9, 2026  
**Next Review:** After 1000 trade sample
