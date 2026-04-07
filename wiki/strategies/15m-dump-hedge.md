# 15M DUMP-HEDGE Strategy

**Type:** Statistical Arbitrage (Two-Leg Hedge)
**Markets:** BTC & ETH 15-minute Up/Down slots
**Trigger:** Dump signal (rapid price drop ≥15% in 3s)

## Strategy Logic

```
LEG1: Detect dump signal → Buy depressed side (YES or NO)
LEG2: Immediately hedge opposite side → Lock in combined edge

IF (combined_cost ≤ $0.94):
    → Positive EV after rebate
    → Execute both legs
```

## Why Two Legs?

**Single-leg risk:**
- Buy YES @ 40¢ → Market crashes to 10¢ → -75% loss
- Unhedged directional exposure

**Two-leg hedge:**
- LEG1: Buy YES @ 40¢ (depressed)
- LEG2: Buy NO @ 50¢ (opposite side)
- Combined: 90¢ total → Guaranteed $1.00 payout
- **Profit:** $0.10 - fees + rebate

## Fee Math (March 30, 2026+)

| Component | Value | Notes |
|-----------|-------|-------|
| **Max Taker Fee** | 1.80% | At p=0.50 (peak variance) |
| **Maker Rebate** | 0.36% | 20% of taker fee |
| **Net Fee Burden** | 1.44% | Taker - rebate |
| **Ghost Drag** | ~2.14% | 62% ghost rate × 50ms RTT |
| **Total Cost** | ~3.58% | Fee + ghost drag |

**Edge threshold:** $0.94 combined cost
- Payout: $1.00
- Profit: $0.06 - fees ≈ $0.02-0.03 net

## Execution Flow

```
1. Monitor for dump signal (≥15% price drop in 3s)
2. LEG1: Buy depressed side (YES or NO)
   - Record shares, price, timestamp
3. LEG2: Buy opposite side (hedge)
   - Match LEG1 share count
   - Must fill within max_wait_secs (default 600s)
4. Track combined cost
5. Hold until resolution
```

## Dynamic Position Sizing

**Seamless slot transition fix (April 5):**
```rust
// Reset LEG state when entering new slot
if let Some(prev_slot) = p.slot_traded {
    if prev_slot != current_slot {
        p.leg1_side = None;
        p.leg1_price = None;
        p.leg1_time = None;
        p.leg2_filled = false;
        p.leg1_shares = None;
    }
}
```

**Why:** Prevents stale LEG1 from previous slot causing wrong hedge calculations.

## Recent Trades (April 6, 2026)

| Time (ET) | Market | Leg | Side | Price | Combined | Status |
|-----------|--------|-----|------|-------|----------|--------|
| 04:48:12 | ETH-15m-1775450700 | LEG1 | YES | 32¢ | - | ✅ Filled |
| 04:48:18 | BTC-15m-1775450700 | LEG1 | YES | 45¢ | - | ✅ Filled |
| 04:48:38 | BTC-15m-1775450700 | LEG2 | NO | 50¢ | 95¢ | ✅ Filled |
| 04:48:43 | ETH-15m-1775450700 | LEG2 | NO | 63¢ | 95¢ | ✅ Filled |
| 05:00:00 | BTC-15m-1775451600 | LEG1 | YES | 49¢ | - | ✅ Filled |
| 05:00:03 | ETH-15m-1775451600 | LEG1 | YES | 50¢ | - | ✅ Filled |
| 05:00:09 | ETH-15m-1775451600 | LEG2 | NO | 45¢ | 95¢ | ✅ Filled |
| 05:01:13 | BTC-15m-1775451600 | LEG2 | NO | 46¢ | 95¢ | ✅ Filled |

## Risk Controls

| Control | Value | Purpose |
|---------|-------|---------|
| **Max wait (LEG2)** | 600s (10min) | Force exit if hedge doesn't fill |
| **Combined threshold** | $0.94 | Minimum EV edge |
| **Share matching** | LEG2 = LEG1 | Delta-neutral hedge |
| **Slot isolation** | Reset on new slot | Prevent stale state |

## Ghost Simulation Results

**90-second sample (March 28):**
- **Total signals:** 161
- **👻 Ghosted:** 100 (62.1%) — liquidity vanished after 50ms
- **✅ Executable:** 56 (34.8%) — real opportunities
- **⚠️ Partial:** 5 (3.1%) — some liquidity remained

**Implication:** Expected live fill rate ~35%, not 100% from dry run.

## Code Location

- **Strategy logic:** `src/strategy/dump_hedge.rs`
- **Dump detection:** `src/dump_detector.rs` (15% drop in 3s)
- **LEG state:** `state.rs` (leg1_side, leg1_price, leg2_filled)
- **Ghost simulator:** `src/ghost_simulator.rs`

---

## Sources

- MEMORY.md (2026-04-05 session logs)
- `/tmp/arb_dry.log` (April 6 trade logs)
- NotebookLM: Ghost simulation analysis (March 28)
- GitHub commit `dee52d8` (seamless slot transitions)
