# Strategy Clone: Profile 0xe1d6b51521bd4365769199f392f9818661bd907

## ⚠️ Security Warning

NotebookLM flagged this profile as potentially malicious bait. The stats ($42.2K biggest win, 9,714 predictions) may be wash-traded to lure users into malicious bot repos. **Do not** download any code associated with this profile. Use the STRATEGY patterns only.

---

## Profile Stats (March 2026)

| Metric | Value |
|--------|-------|
| Positions Value | $34.6K |
| Biggest Win | $42.2K |
| Predictions | 9,714 |
| Avg Trades/Day | ~300+ |
| Win Rate | Unknown |

---

## Strategy Architecture

### 1. Hybrid Maker-Taker

**Maker Side:**
- Place passive limit orders at 0.5%-2% spread from mid
- Earn maker rebates (0.36% of taker fee = 20% rebate)
- Adjust quotes based on inventory skew

**Taker Side:**
- Execute aggressively on dump events (15%+ price crash)
- Hedge opposite side when combined cost ≤ $0.95

### 2. Position Sizing Formula

```
position_size = capital × 0.5% × setup_score

setup_score = 0.30 × spread_tightness
            + 0.25 × orderbook_depth
            + 0.25 × order_flow_momentum
            + 0.20 × time_to_resolution_factor
```

**Hard caps:**
- Max 5-10% of capital per trade
- Max 25% in correlated markets

### 3. Market Selection (7 Filters)

```rust
fn market_passes_filters(market: &Market) -> bool {
    market.liquidity >= 10_000_usd &&
    market.depth_within_1_percent() &&
    market.leading_probability >= 0.65 &&
    market.leading_probability <= 0.96 &&
    market.spread_bps <= 200 &&
    market.days_to_resolution <= 14 &&
    !market.has_recent_spike(0.08) &&
    market.order_flow_imbalance >= 500_usd &&
    portfolio.correlated_exposure(market) <= 0.25 * capital
}
```

### 4. Inventory Skew Management

```rust
// Calculate skew
fn calculate_skew(yes_shares: i64, no_shares: i64, max_capacity: i64) -> f64 {
    (yes_shares - no_shares) as f64 / max_capacity as f64
}

// Adjust quotes based on skew
fn adjust_quotes(bid: u64, ask: u64, skew: f64) -> (u64, u64) {
    let skew_limit = 0.30;
    
    if skew >= skew_limit {
        // Long YES: Lower both to sell faster
        (bid.saturating_sub(2), ask.saturating_sub(2))
    } else if skew <= -skew_limit {
        // Short YES: Raise both to buy YES faster
        (bid.saturating_add(2), ask.saturating_add(2))
    } else {
        (bid, ask)
    }
}
```

### 5. Edge Detection (Dump-and-Hedge)

```rust
const DUMP_THRESHOLD: f64 = 0.15; // 15% drop
const DUMP_WINDOW_SECS: u64 = 3;
const HEDGE_THRESHOLD_CENTS: u64 = 95;

fn detect_dump(current_ask: u64, previous_ask: u64, time_delta_ms: u64) -> bool {
    let drop_pct = (previous_ask - current_ask) as f64 / previous_ask as f64;
    drop_pct >= DUMP_THRESHOLD && time_delta_ms <= DUMP_WINDOW_SECS * 1000
}

fn should_hedge(leg1_price: u64, opposite_ask: u64) -> bool {
    leg1_price + opposite_ask <= HEDGE_THRESHOLD_CENTS
}
```

### 6. Circuit Breakers

```rust
enum BreakerState {
    Active,
    HaltedExtremePrices,    // Bid ≤ 2¢ or Ask ≥ 98¢
    HaltedMaxPosition,      // Position > max_position_value
    HaltedDrawdown,        // Daily loss ≥ 3%
    HaltedWeeklyLoss,      // Weekly loss ≥ 8%
    HaltedSingleMarket,    // Single market loss ≥ 5%
    HaltedPortfolioLoss,   // Total portfolio loss ≥ 15%
}

fn evaluate_circuit_breakers(
    bid_cents: u64,
    ask_cents: u64,
    position_value: f64,
    daily_pnl: f64,
    weekly_pnl: f64,
    market_pnl: HashMap<MarketId, f64>,
    max_position: f64,
    starting_capital: f64,
) -> BreakerState {
    // 1. Extreme prices
    if bid_cents <= 2 || ask_cents >= 98 {
        return BreakerState::HaltedExtremePrices;
    }
    
    // 2. Max position
    if position_value > max_position {
        return BreakerState::HaltedMaxPosition;
    }
    
    // 3. Daily drawdown
    if daily_pnl / starting_capital <= -0.03 {
        return BreakerState::HaltedDrawdown;
    }
    
    // 4. Weekly drawdown
    if weekly_pnl / starting_capital <= -0.08 {
        return BreakerState::HaltedWeeklyLoss;
    }
    
    // 5. Single market loss
    for (_, pnl) in market_pnl {
        if pnl / starting_capital <= -0.05 {
            return BreakerState::HaltedSingleMarket;
        }
    }
    
    BreakerState::Active
}
```

---

## Integration into Pingpong

### Current Implementation Status

| Component | Status | File |
|-----------|--------|------|
| Inventory skew | ✅ Implemented | `quote_manager.rs` |
| Edge detection | ✅ Partial | `market_state.rs` |
| Circuit breakers | ⏳ Partial | Need state machine |
| Position sizing | ❌ Missing | Need setup_score |
| Market selection | ❌ Missing | Need filter pipeline |
| Maker spreads | ❌ Missing | Need tiered quotes |

### Priority Implementation Order

1. **Circuit Breaker State Machine** - Critical for safety
2. **Drawdown Tracking** - Required for circuit breakers
3. **Market Selection Pipeline** - Filter bad markets
4. **Setup Score Calculation** - Dynamic position sizing
5. **Maker Spread Tiers** - Passive liquidity provision
6. **Dump Detection** - Taker edge capture

---

## Risk Management

### Stop-Loss Rules

| Trigger | Action |
|---------|--------|
| Daily loss ≥ 3% | Halt all trading |
| Weekly loss ≥ 8% | Halt all trading |
| Single market loss ≥ 5% | Exit position, halt that market |
| Portfolio loss ≥ 15% | Emergency exit all positions |

### Time-Based Stops (Hedge Execution)

```rust
const MAX_WAIT_FOR_HEDGE_SECS: u64 = 300; // 5 minutes

// If hedge not filled within 5 min, force market exit
if time_since_leg1 > MAX_WAIT_FOR_HEDGE_SECS {
    execute_stop_loss(market_id);
}
```

---

## Expected Performance

Based on strategy analysis:

| Metric | Estimate |
|--------|----------|
| Win Rate | 65-75% (on convergence plays) |
| Avg Edge | 2-5% per trade |
| Max Drawdown | 8% (with circuit breakers) |
| Trades/Day | 50-300 (market dependent) |
| Monthly Volume | $50K-$200K |

---

## Next Session Tasks

1. Implement circuit breaker state machine
2. Add drawdown tracking (daily/weekly)
3. Build market filter pipeline
4. Add setup_score to hot path
5. Implement maker spread tiers
6. Add dump detection to edge detection