# HEARTBEAT.md

## Detailed PnL Report - Every 6 Minutes

### How It Works

1. Cron runs `/tmp/pnl_telegram.py` every 6 minutes
2. Report includes:
   - Opportunity count (Maker Hybrid vs Taker)
   - **Ghost Simulation stats** (Ghosted/Executable/Partial)
   - Last opportunity details
   - Config summary
3. Report sent to DEE via Telegram automatically

### Ghost Simulation Stats

**What it shows:**
- Total signals tracked
- Ghosted % (liquidity vanished after 50ms RTT)
- Executable % (real opportunities)
- Partial % (some liquidity remained)

**Current results (validated):**
- ~58% ghost rate
- ~40% executable rate
- Validates NotebookLM's fill rate prediction

### What To Do

If `/tmp/pnl_ready.txt` exists (legacy check):
1. Read `/tmp/pnl_detailed.txt`
2. Send content to DEE via Telegram
3. Remove `/tmp/pnl_ready.txt`

### Markets Tracked

- BTC-5m, BTC-15m
- ETH-5m, ETH-15m
- (SOL and XRP excluded)
