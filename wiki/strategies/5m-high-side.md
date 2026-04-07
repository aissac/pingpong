# 5M HIGH-SIDE Strategy

**Type:** Market Maker Scalping
**Markets:** BTC & ETH 5-minute Up/Down slots
**Trigger:** Extreme probability (≥90¢ or ≤10¢)

## Strategy Logic

```
IF (YES_ask ≥ 90¢ OR NO_ask ≥ 90¢):
    → Buy the 90¢ side as MAKER
    → Collect 20% rebate (0.36% on crypto)
    → Hold until resolution or adverse move
```

## Why 90¢?

| Probability | Taker Fee | Maker Rebate | Net Edge |
|-------------|-----------|--------------|----------|
| 90¢ (90%) | 1.80% | -0.36% (you GET paid) | +2.16% |
| 80¢ (80%) | 1.44% | -0.29% | +1.73% |
| 70¢ (70%) | 1.08% | -0.22% | +1.30% |

**At 90¢:**
- Polymarket fee formula: `fee = 0.25 × variance²`
- Variance at 90¢ = 0.18 (low)
- Fee ≈ 0.35% (vs 1.80% peak at 50¢)
- Maker rebate: 20% of taker fee = 0.07%
- **Net: You profit from spread + rebate**

## Execution Flow

```
1. Detect extreme price (YES ≥ 90¢ or NO ≥ 90¢)
2. Check slot timing (must be within last 30s of 5m slot)
3. Verify not already traded this slot (slot_traded check)
4. Place MAKER order (post-only, earn rebate)
5. Monitor for fill
6. Hold until resolution OR exit on adverse move
```

## Recent Trades (April 6, 2026)

| Time (ET) | Market | Side | Price | Remaining | Status |
|-----------|--------|------|-------|-----------|--------|
| 04:49:31 | BTC-5m-1775450700 | YES | 90¢ | 29s | ✅ Filled |
| 04:49:44 | ETH-5m-1775450700 | NO | 90¢ | 16s | ✅ Filled |
| 04:54:45 | ETH-5m-1775451000 | YES | 90¢ | 15s | ✅ Filled |
| 04:54:55 | BTC-5m-1775451000 | YES | 90¢ | 5s | ✅ Filled |
| 04:59:30 | BTC-5m-1775451300 | NO | 90¢ | 30s | ⏭️ Skipped (already traded) |
| 04:59:30 | ETH-5m-1775451300 | NO | 95¢ | 30s | ⏭️ Skipped (already traded) |

## Risk Controls

| Control | Value | Purpose |
|---------|-------|---------|
| **Max per slot** | 1 trade | Prevent overtrading same market |
| **Slot timing** | Last 30s | Minimize exposure time |
| **Position limit** | 30 shares | Cap single-trade risk |
| **Daily loss** | 3% | Circuit breaker halt |

## Performance (Dry Run)

- **Win rate:** 78.6% (March 18 session)
- **Avg profit:** +$306.34/day
- **Slots traded:** 28
- **Strategy:** BTC-focused (ETH secondary)

## Code Location

- **Strategy logic:** `src/strategy/high_side.rs`
- **Trigger detection:** `bin/arb_bot.rs` (check_strategies loop)
- **Slot tracking:** `state.rs` (slot_traded field)

---

## Sources

- MEMORY.md (2026-04-05 session logs)
- `/tmp/arb_dry.log` (April 6 trade logs)
- NotebookLM: Fee formula analysis (March 28)
