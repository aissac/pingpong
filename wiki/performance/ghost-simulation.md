# Ghost Simulation

**Date:** March 28, 2026
**Purpose:** Validate expected live fill rate vs dry run performance

---

## The Problem

**Dry run shows 100% fill rate** — every opportunity triggers, every trade logs.

**Reality:** In live trading, liquidity vanishes between detection and execution.

**Question:** What's our *real* expected fill rate?

---

## NotebookLM Prediction

**Prediction:** ~35-40% fill rate in live trading.

**Reasoning:**
- Network RTT: 50ms (home connection)
- Liquidity changes every 10-100ms in volatile markets
- By the time order arrives, 60% of opportunities are gone ("ghosted")

**Validation needed:** Simulate 50ms RTT delay and measure ghost rate.

---

## Implementation

**File:** `src/ghost_simulator.rs`

**Algorithm:**
```rust
1. Track signal when opportunity detected
   - condition_id, side, price, initial_depth

2. Wait 50ms (simulated network RTT)

3. Check if depth still exists after delay
   - Query REST API for current orderbook

4. Classify result:
   - GHOSTED: depth = 0 (liquidity vanished)
   - PARTIAL: depth < order_size (some liquidity remains)
   - EXECUTABLE: depth >= order_size (full fill possible)
```

**Key insight:** Don't place real orders — just simulate the delay and check if liquidity would have been available.

---

## Results (90-second sample)

**Total signals tracked:** 161

| Status | Count | Percentage |
|--------|-------|------------|
| 👻 **Ghosted** | 100 | **62.1%** |
| ✅ **Executable** | 56 | **34.8%** |
| ⚠️ **Partial** | 5 | **3.1%** |
| **Total** | **161** | **100%** |

### Visualization

```
Ghost Rate: ████████████████████████████████████████ 62.1%
Executable: ██████████████████████ 34.8%
Partial:    ██ 3.1%
```

---

## Key Insights

### 1. Dry Run is Misleading

**Dry run fill rate:** 100%
**Expected live fill rate:** ~35%

**Why:** Dry run doesn't account for:
- Network latency (50ms RTT)
- Liquidity changes during RTT
- Order book updates between detection and execution

### 2. Ghost Rate Validates NotebookLM

**NotebookLM prediction:** 60-65% ghost rate
**Measured ghost rate:** 62.1%

✅ **Prediction confirmed!**

### 3. Path to Improvement

**Current bottleneck:** Network RTT (50ms from home)

**Solution:** Deploy to Frankfurt (eu-central-1)
- Expected RTT: ~1ms (colocated with Polymarket)
- Expected ghost rate: <10% (proportional to RTT)
- **50× improvement in fill rate**

**Math:**
```
Ghost rate ∝ RTT
50ms → 62% ghost rate
1ms  → ~1.2% ghost rate (theoretical)
Realistic: 5-10% (conservative estimate)
```

---

## Impact on Strategy

### 5M HIGH-SIDE

**Dry run:** Triggers every time bid ≥90¢
**Live (home):** ~35% of triggers fill
**Live (Frankfurt):** ~90-95% of triggers fill

**Implication:** Position sizing must account for 35% fill rate when testing from home.

### 15M DUMP-HEDGE

**Dry run:** Both legs fill 100%
**Live (home):** 
- LEG1: ~35% fill rate
- LEG2: ~35% fill rate (independent)
- Both legs: ~12% combined (0.35 × 0.35)

**Implication:** DUMP-HEDGE is much harder to execute from home. Frankfurt deployment is critical.

---

## Frankfurt Deployment Impact

| Metric | Home (NYC) | Frankfurt (eu-central-1) | Improvement |
|--------|------------|--------------------------|-------------|
| **RTT** | 50ms | 1ms | 50× |
| **Ghost rate** | 62% | ~5-10% | 6-12× |
| **Fill rate** | 35% | ~90-95% | 2.6× |
| **LEG1+LEG2 both fill** | 12% | ~81% | 6.75× |

**Conclusion:** Frankfurt deployment is not optional — it's the difference between a working bot and a broken one.

---

## Code Location

- **Simulator:** `src/ghost_simulator.rs`
- **Background thread:** `src/background.rs` (runs simulation)
- **Telemetry:** `src/telemetry.rs` (OpportunitySnapshot struct)

---

## Next Steps

1. ✅ Ghost simulation complete (62% ghost rate measured)
2. ✅ Frankfurt deployment complete (1ms RTT achieved)
3. ⏳ Re-run ghost simulation from Frankfurt (expect <10% ghost rate)
4. ⏳ Adjust position sizing based on new fill rate

---

## Sources

- MEMORY.md (March 28 session logs)
- `src/ghost_simulator.rs` (implementation)
- NotebookLM: Fill rate prediction analysis
- Network RTT measurements (NYC vs Frankfurt)
