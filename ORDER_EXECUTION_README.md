# Order Execution Module - Usage Guide

## Quick Start

### Paper Trading (Default)
```bash
# Just run - DRY_RUN=true by default
./run.sh
```

### Live Trading
1. Create `.env` file with your credentials:
```env
POLYMARKET_PRIVATE_KEY=0x...your_private_key
POLYMARKET_FUNDER_ADDRESS=0x...your_proxy_wallet_address
DRY_RUN=false
MAX_DAILY_LOSS_USD=100.0
ORDER_TIMEOUT_SECS=60
TARGET_WALLET=0xe1d6b51521bd4365769199f392f9818661bd907
COPY_RATIO=0.10
MAX_TRADE_CAP_USD=500.0
```

2. Run:
```bash
./run.sh
```

## Safety Features

### 1. **Dry-Run Default**
- `DRY_RUN=true` by default - no real orders submitted
- Must explicitly set `DRY_RUN=false` for live trading

### 2. **Circuit Breaker**
- `MAX_DAILY_LOSS_USD=100.0` - stops trading after losing $100
- Halted state requires manual reset (restart or midnight auto-reset)

### 3. **Order Timeout**
- Orders unfilled after 60 seconds are cancelled
- Prevents stale orders from lingering

### 4. **Minimum Trade Size**
- Trades < $10 (after copy ratio) are skipped
- Avoids dust positions and high fee impact

## Architecture

```
copy_trader.rs          -> Whale polling engine (1-second loop)
    |
    v
CopyTradeAction         -> Crossbeam channel (hot path)
    |
    v
order_executor.rs       -> Order submission / paper trading
    |
    +-- DRY_RUN=true    -> Paper trade simulation
    |
    +-- DRY_RUN=false   -> EIP-712 signed order -> CLOB API
```

## Files Created/Modified

### New Files
- `src/order_executor.rs` - Order submission module with circuit breaker
- `run.sh` - Convenience run script
- `copytrader.service` - systemd unit file
- `.env.example` - Template for configuration

### Modified Files
- `src/main.rs` - Integrated order executor, replaced paper-only loop
- `src/lib.rs` - Added `pub mod order_executor;`

## Testing

### Test Signing (Live)
```bash
# Set up credentials in .env
source .env
cargo test -- --test-threads=1
```

### Test Paper Trading
```bash
# No credentials needed
./run.sh
# Watch for "paper_trade" JSON logs
```

### Check Logs
```bash
tail -f /tmp/copytrader.log | jq .
```

## API Endpoints Used

- **Order Submission**: `POST https://clob.polymarket.com/order`
- **Order Status**: `GET https://clob.polymarket.com/orders?id={order_id}`
- **Wallet Orders**: `GET https://clob.polymarket.com/orders?maker={wallet}`
- **Whale Trades**: `GET https://data-api.polymarket.com/trades?user={target_wallet}`

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DRY_RUN` | `true` | Paper trading mode |
| `MAX_DAILY_LOSS_USD` | `100.0` | Circuit breaker threshold |
| `ORDER_TIMEOUT_SECS` | `60` | Cancel unfilled orders after |
| `TARGET_WALLET` | `0xe1d6b...` | Whale to copy |
| `COPY_RATIO` | `0.10` | Copy 10% of whale size |
| `MAX_TRADE_CAP_USD` | `500.0` | Max per-trade size |
| `POLYMARKET_PRIVATE_KEY` | - | Your private key (live mode) |
| `POLYMARKET_FUNDER_ADDRESS` | - | Your proxy wallet address |

## Monitoring

### Daily Stats
The executor tracks:
- Total trades
- Win rate
- Cumulative PnL
- Daily loss (for circuit breaker)

### systemd Service
```bash
sudo cp copytrader.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable copytrader
sudo systemctl start copytrader
sudo journalctl -u copytrader -f
```

## Next Steps

1. **Add fill tracking** - Poll order status and update positions
2. **Add position tracking** - Track open positions per market
3. **Add PnL calculation** - Real-time PnL from fills
4. **Add Telegram alerts** - Notify on trades/circuit breaker triggers
5. **Add stop-loss** - Auto-close losing positions