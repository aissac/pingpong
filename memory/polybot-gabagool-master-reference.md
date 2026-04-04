# Polybot Gabagool Strategy — Master Reference

## Sources
- `ent0n29/polybot` — https://github.com/ent0n29/polybot
- `docs/EXAMPLE_STRATEGY_SPEC.md` — Full strategy specification

---

## What is Gabagool Complete-Set Arbitrage?

Gabagool exploits a **pricing anomaly** in Polymarket's Up/Down binary markets. When you buy YES + NO on the same market, you always pay exactly $1.00 at settlement. But during trading, `bid_up + bid_dn` (combined cost to buy both sides) can deviate from $1.00.

**Example:**
- `bid_up = 0.52`, `bid_dn = 0.46`
- Combined cost = $0.98
- Edge = `1.00 - 0.98 = $0.02` (2% edge)
- **Buy both sides → guaranteed $0.02 profit at expiry**

---

## Market Universe

Only Up/Down binary markets for:

| Market Type | Slug Pattern | Trade Window |
|------------|-------------|--------------|
| BTC 15m | `btc-updown-15m-*` | 0-900 seconds |
| ETH 15m | `eth-updown-15m-*` | 0-900 seconds |
| BTC 1h | `bitcoin-up-or-down-*` | 0-3600 seconds |
| ETH 1h | `ethereum-up-or-down-*` | 0-3600 seconds |

---

## Core Signal: Complete-Set Edge

```
complete_set_cost = bid_up + bid_dn
complete_set_edge = 1.0 - complete_set_cost
```

**Trade condition:** `complete_set_edge >= complete-set-min-edge` (default: 0.01 = 1%)

If edge is below threshold → cancel working orders, do nothing.

---

## Pricing Algorithm (Maker Mode)

For each leg (UP and DOWN):

1. Read `bestBid`, `bestAsk`
2. Compute `mid = (bestBid + bestAsk) / 2`
3. Compute `spread = bestAsk - bestBid`
4. Compute `effectiveImproveTicks = improve-ticks + skewTicks`
5. If spread >= 0.20 (extremely wide):
   ```
   entry = mid - tickSize * max(0, improve-ticks - skewTicks)
   ```
6. Else (normal tight spread):
   ```
   entry = min(bestBid + tickSize * effectiveImproveTicks, mid)
   ```
7. Round **DOWN** to tick
8. Never cross: if `entry >= bestAsk`, set `entry = bestAsk - tickSize` (abort if < 0.01)

---

## Inventory Skew (Hedging Nudge)

Track per-market filled inventory:
- `inv_up_shares`, `inv_dn_shares`
- `imbalance = inv_up_shares - inv_dn_shares`

Compute skew ticks (linear, capped):
```
abs = |imbalance|
scale = clamp(abs / complete-set-imbalance-shares-for-max-skew, 0..1)
skew = round(scale * complete-set-max-skew-ticks)
```

Apply to legs:
- If `imbalance > 0` (too much UP): favor DOWN
  - `skewTicksDown = +skew`, `skewTicksUp = -skew`
- If `imbalance < 0` (too much DOWN): favor UP
  - `skewTicksUp = +skew`, `skewTicksDown = -skew`

**Defaults:**
- `complete-set-max-skew-ticks: 1`
- `complete-set-imbalance-shares-for-max-skew: 200`

---

## Share Sizing Table

### BTC 15m (`btc-updown-15m-*`)

| Seconds to End | Shares |
|--------------|--------|
| < 60s | 11 |
| < 180s | 13 |
| < 300s | 17 |
| < 600s | 19 |
| >= 600s | 20 |

### ETH 15m (`eth-updown-15m-*`)

| Seconds to End | Shares |
|--------------|--------|
| < 60s | 8 |
| < 180s | 10 |
| < 300s | 12 |
| < 600s | 13 |
| >= 600s | 14 |

### BTC 1h (`bitcoin-up-or-down-*`)

| Seconds to End | Shares |
|--------------|--------|
| < 60s | 9 |
| < 180s | 10 |
| < 300s | 11 |
| < 600s | 12 |
| < 900s | 14 |
| < 1200s | 15 |
| < 1800s | 17 |
| >= 1800s | 18 |

### ETH 1h (`ethereum-up-or-down-*`)

| Seconds to End | Shares |
|--------------|--------|
| < 60s | 7 |
| < 300s | 8 |
| < 600s | 9 |
| < 900s | 11 |
| < 1200s | 12 |
| < 1800s | 13 |
| >= 1800s | 14 |

---

## Bankroll Caps (Critical for $200)

**Per-order cap:**
```
max_order_bankroll_fraction * bankroll_usd
```

**Total cap:**
```
max_total_bankroll_fraction * bankroll_usd
```

**Total exposure includes:**
- Open order remaining notional
- Open positions notional
- Fills since last positions refresh

---

## Taker Top-Ups

### A) End-of-Market Top-Up
If `seconds_to_end <= complete-set-top-up-seconds-to-end` (default: 60s)
AND `abs(imbalance) >= complete-set-top-up-min-shares` (default: 10)

→ Buy lagging leg at ask for `topUpShares = abs(imbalance)`

### B) Fast Top-Up After Lead Fill
If:
- `abs(imbalance) >= complete-set-fast-top-up-min-shares` (default: 10)
- Cooldown passed since last top-up (default: 15000ms)
- Time since lead leg fill: 3-120 seconds
- `hedgedEdge = 1 - (leadFillPrice + laggingBestAsk) >= 0.01`

→ Buy lagging leg at ask for `topUpShares = abs(imbalance)`

---

## Recommended $200 Safe Settings

```yaml
hft:
  mode: LIVE
  executor:
    send-live-ack: true

hft:
  strategy:
    gabagool:
      bankroll-usd: 200
      max-order-bankroll-fraction: 0.05   # <= $10 per order
      max-total-bankroll-fraction: 0.50   # <= $100 total exposure
      refresh-millis: 750              # slower = safer
      min-replace-millis: 10000       # don't replace too fast
```

**If you need even safer:** reduce `max-total-bankroll-fraction` first.

---

## Paper vs Live Mode

| Aspect | Paper | Live |
|--------|-------|------|
| Real orders | No | Yes |
| Private key | Any | Real key required |
| API keys | Not used | `POLYMARKET_API_KEY`, `POLYMARKET_API_SECRET`, `POLYMARKET_API_PASSPHRASE` |
| `hft.mode` | `PAPER` | `LIVE` |

---

## Key Timing Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `refresh-millis` | 500 | Strategy loop interval |
| `min-replace-millis` | 5000 | Min age before replacing order |
| `min-seconds-to-end` | Config | Start trading at this time |
| `max-seconds-to-end` | Config | Stop trading at this time |
| Stale threshold | 2000ms | TOB is stale after this |

---

## Order Management

- **At most 1 working order per token** (UP and DOWN)
- If no existing order → place
- If existing order and (price or size) changed → cancel/replace only if age >= `min-replace-millis`
- Orders are GTC BUY limit orders

---

## Complete-Set Arbitrage Edge Math

```
You buy UP at bid_up
You buy DOWN at bid_dn
Combined cost = bid_up + bid_dn
At expiry, one side pays $1.00, other pays $0.00
Profit = $1.00 - (bid_up + bid_dn) = complete_set_edge
```

**Key insight:** This is **risk-free** if the edge exceeds transaction costs and you hold to expiry.

---

## Why It Works

1. Polymarket Up/Down markets always settle at exactly $1.00 for one side
2. During trading, `bid_up + bid_dn` can be < $1.00 (underpriced)
3. Buying both sides = **complete set** = guaranteed $1.00 at expiry
4. If combined cost < $1.00, you lock in profit
5. The strategy maintains inventory skew to minimize net exposure

---

## Data Requirements

- **Both legs' WebSocket top-of-book required:**
  - `bestBidPrice`, `bestAskPrice`, `updatedAt`
- **Stale handling:** If `now - updatedAt > 2000ms` → cancel orders, skip market
- **Positions refresh:** Every ~5 seconds

---

_Last updated: 2026-03-20_
