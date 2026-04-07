# System Overview

**Bot Name:** Pingpong (formerly Gabagool)
**Language:** Rust 2021
**Deployment:** AWS c6i.large (Frankfurt, eu-central-1c)
**Status:** Running (DRY_RUN mode)

## Core Architecture

### Event-Driven Design (April 2026)

The bot uses a fully event-driven architecture with **no polling** for market discovery:

```
┌─────────────────────────────────────────────────────────────┐
│                     WebSocket Stream                         │
│  - price_change events (real-time orderbook updates)        │
│  - new_market events (instant market discovery)             │
│  - market_resolved events (auto-cleanup)                    │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                  Dynamic Subscription Manager                │
│  - Subscribe/unsubscribe tokens on-the-fly                  │
│  - No WebSocket reconnect required                          │
│  - REST fallback for token discovery                        │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Strategy Engine                            │
│  - 5M HIGH-SIDE (extreme probability scalping)              │
│  - 15M DUMP-HEDGE (arbitrage with hedge)                    │
│  - Seamless slot transitions (LEG state reset)              │
└─────────────────────────────────────────────────────────────┘
```

## Key Components

| Component | File | Purpose |
|-----------|------|---------|
| **Main Binary** | `bin/arb_bot.rs` | Entry point, WebSocket loop, token fetching |
| **Hot Path** | `src/hft_hot_path.rs` | Sub-microsecond message processing |
| **Orderbook** | `src/orderbook.rs` | Local orderbook management |
| **Strategies** | `src/strategy/` | HIGH-SIDE, DUMP-HEDGE implementations |
| **Market Rollover** | `src/market_rollover.rs` | Dynamic subscription at slot boundaries |
| **Ghost Simulator** | `src/ghost_simulator.rs` | 50ms RTT simulation (62% ghost rate) |

## Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| **Processing Latency** | 0.7µs average | Matches polyfill-rs benchmarks |
| **Network RTT** | ~1ms (Frankfurt) | Colocated with Polymarket infra |
| **Message Throughput** | 600+ msg/sec | Sustained during high volatility |
| **Ghost Rate** | 62% | Liquidity vanishes after 50ms RTT |

## Execution Mode

**Current:** DRY_RUN (no real orders)
- Strategies fire correctly
- Logs simulated trades
- Waits for go-live signal

**Path to LIVE:**
1. ✅ Volume validation complete
2. ✅ Event-driven architecture stable
3. ⏳ Wallet funding & token allowances
4. ⏳ Final risk parameter review

## Markets Tracked

- **BTC Up/Down:** 5m & 15m slots
- **ETH Up/Down:** 5m & 15m slots
- **Total Tokens:** 16 (8 markets × 2 outcomes)
- **Excluded:** SOL, XRP (lower liquidity)

---

## Sources

- MEMORY.md (session logs 2026-03-27 to 2026-04-05)
- `bin/arb_bot.rs` (event-driven implementation)
- GitHub commit `e4c8ebd` (Bug 1-5 fixes)
