# AWS Bot Running Status — April 7, 2026

**Updated:** 2026-04-07  
**Sources:** [AWS Deployment Log](../../raw/april-7-bot/2026-04-07-aws-bot-deployment.md)

---

## Current Status

| Metric | Value |
|--------|-------|
| **Instance** | AWS EC2 Frankfurt |
| **IP** | 3.77.156.196 |
| **Status** | ✅ Running |
| **Mode** | DRY_RUN (paper trading) |
| **Uptime** | Since April 7, ~15:00 UTC |
| **Messages** | 317,000+ processed |

---

## Deployment Timeline

| Time (UTC) | Event |
|------------|-------|
| 15:09 | Code changes: WebSocket logging + Ping/Pong |
| 15:11 | Git commit `5a51723`, pushed to main |
| 15:11 | SSH to AWS, git pull, cargo build |
| 15:12 | Bot restarted in DRY_RUN mode |
| 15:18 | Confirmed 317K+ messages processed |
| 15:21 | Memory updated, monitoring active |

---

## Configuration

**Binary:** `./target/release/arb_bot`  
**Log:** `/tmp/paper_trade.log`  
**Markets:** 12 (BTC/ETH 5m + 15m up/down)

### Strategies Enabled

| Strategy | Status | Reason |
|----------|--------|--------|
| PRE_ORDER | ✅ | Risk-free arb, 80ms OK |
| HIGH_SIDE | ✅ | Late-slot momentum, 80ms OK |
| LOTTERY | ✅ | Flash crash, 80ms OK |
| MID (Maker) | ❌ | Needs <10ms, AWS has ~80ms |

---

## Performance

**Message Rate:** ~600-800 msg/sec  
**WebSocket:** Stable, no disconnects  
**Latency:** ~80ms RTT (Frankfurt → Polymarket)

### Log Output Sample

```
🏓 BTC/ETH Up/Down Bot - Seamless Slot Transitions
📊 5M High-Side | 15M Dump-Hedge | 1H Pre-Limit
📊 Dry run: true
📡 Received 317000 messages
```

---

## Monitoring Commands

```bash
# Live log tail
ssh -i ~/.ssh/pingpong.pem ubuntu@3.77.156.196 'tail -f /tmp/paper_trade.log'

# Check process status
ssh -i ~/.ssh/pingpong.pem ubuntu@3.77.156.196 'ps aux | grep arb_bot'

# Message count
ssh -i ~/.ssh/pingpong.pem ubuntu@3.77.156.196 'grep "Received" /tmp/paper_trade.log | tail -1'
```

---

## Known Limitations

1. **Latency:** ~80ms insufficient for MID (Maker) strategy
   - Solution: Migrate to NYCServers for <10ms

2. **Market Discovery:** Bot waits for boundary + 5s
   - May miss early PRE_ORDER opportunities
   - Consider pre-subscribing to known tokens

3. **No Fill Confirmation:** DRY_RUN mode doesn't receive fills
   - Strategy state may drift from actual positions
   - Need live testing to validate

---

## Next Steps

1. **Monitor for triggers:**
   - PRE_ORDER: 95s before slot
   - HIGH_SIDE: ≥90¢ in last 30s
   - LOTTERY: 15% flash crash

2. **If profitable in dry run:**
   - Enable live trading (`DRY_RUN=false`)
   - Fund wallet with USDC on Polygon
   - Set token allowances

3. **If adverse selection on MID:**
   - Migrate to NYCServers colocation
   - Target <10ms latency

---

## Incident History

| Date | Issue | Resolution |
|------|-------|------------|
| April 1-6 | Instance unreachable (72+ hours) | Manual recovery |
| April 7 | Bot redeployed | Stable since |

---

## See Also

- [AWS Deployment Specs](./aws-deployment.md)
- [Profile Strategy Extraction](../strategies/profile-strategy-extraction.md)
- [Latency Benchmarks](../performance/latency.md)

---

## Raw Data

- [Deployment Log](../../raw/april-7-bot/2026-04-07-aws-bot-deployment.md)
