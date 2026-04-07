# AWS Deployment

**Instance:** `i-060737e3c67de6825` (pingpong)
**Region:** eu-central-1 (Frankfurt, Germany)
**Availability Zone:** eu-central-1c
**Status:** ✅ Running (since April 4, 2026)

## Instance Specs

| Spec | Value |
|------|-------|
| **Type** | c6i.large |
| **vCPU** | 2 (Intel Xeon 8375C) |
| **RAM** | 4 GB |
| **Storage** | 80 GB NVMe SSD |
| **Network** | Up to 12.5 Gbps |
| **Public IP** | 3.77.156.196 |
| **Private IP** | 172.31.2.134 |

## Cost

| Pricing Model | Cost |
|---------------|------|
| **On-Demand** | ~$2/day (~$60/month) |
| **Spot** | ~$0.60/day (~$18/month) |
| **Reserved (1yr)** | ~$35/month |

**Recommendation:** Use Spot for DRY_RUN, On-Demand for LIVE trading.

## SSH Access

```bash
ssh -i ~/.ssh/pingpong.pem ubuntu@3.77.156.196
```

## Bot Deployment

**Location:** `/home/ubuntu/polymarket-market-maker/`
**Binary:** `./target/release/arb_bot`
**PID:** 283342 (as of April 5, 4:51 PM ET)
**Uptime:** 2+ days (stable)

### Process Management

```bash
# Check status
ps aux | grep arb_bot
systemctl status pingpong  # if using systemd

# Restart bot
cd /home/ubuntu/polymarket-market-maker
./target/release/arb_bot &

# View logs
tail -f /tmp/arb_dry.log
```

## Security Configuration

| Setting | Value |
|---------|-------|
| **SSH** | Key-only (no password) |
| **Key** | `~/.ssh/pingpong.pem` |
| **Security Group** | SSH (22), All outbound |
| **Firewall** | UFW enabled, SSH only |

### Verified Clean (April 4)
- No backdoors detected
- No unauthorized users
- Clean `authorized_keys`
- No suspicious processes

## Network Performance

| Metric | Value | Notes |
|--------|-------|-------|
| **RTT to Polymarket** | ~1ms | Colocated in Frankfurt |
| **Bandwidth** | 12.5 Gbps | More than sufficient |
| **Packet Loss** | 0% | Stable connection |

**Why Frankfurt:**
- Polymarket's CLOB infrastructure is reportedly hosted in Frankfurt
- 30-80ms RTT from US East → 1ms from Frankfurt
- Critical for HFT: 50× improvement in fill rate

## Incident History

### April 1-3, 2026: Server Outage (40+ hours)

**Old Instance:** `63.179.252.138`
**Symptoms:**
- SSH connection timeout
- Ping: 100% packet loss
- Bot offline entire period

**Resolution:**
- DEE rebuilt instance via AWS console
- New instance: `i-060737e3c67de6825`
- New IP: `3.77.156.196`
- Bot redeployed April 4

**Lesson:** Set up CloudWatch alarms for instance status checks.

### March 28, 2026: Market Expiration

**Issue:** Bot discovered markets at startup, but they expired after 15 minutes.
**Root Cause:** No dynamic market discovery.
**Fix:** Event-driven architecture (April 5) — WebSocket `new_market` events.

## Monitoring

### Current Setup

| Metric | Tool | Frequency |
|--------|------|-----------|
| **CPU/Memory** | AWS CloudWatch | 5 min |
| **Disk** | `df -h` | Manual |
| **Bot Status** | `ps aux` | Manual |
| **Logs** | `/tmp/arb_dry.log` | Real-time |

### Recommended Additions

1. **CloudWatch Alarms:**
   - CPU > 80% for 5 min
   - Status check failed
   - Network packets dropped

2. **Bot Heartbeat:**
   - Telegram message every 6 minutes (already configured)
   - Alert if no message for 15 min

3. **Log Aggregation:**
   - Ship logs to CloudWatch Logs or Datadog
   - Search/triage without SSH

## Path to Production

### Current State (April 6)
- ✅ Instance stable (2+ days uptime)
- ✅ Bot running (DRY_RUN mode)
- ✅ Event-driven architecture working
- ✅ WebSocket stable (Ping/Pong keepalive)
- ✅ Dynamic market discovery (no more expired markets)
- ✅ Single trade per slot (no duplicates)

### Remaining for LIVE

| Task | Status | Owner |
|------|--------|-------|
| **Wallet funding** | ⏳ Pending | DEE |
| **USDC on Polygon** | ⏳ Pending | DEE |
| **Token allowances** | ⏳ Pending | Bot (auto) |
| **Risk parameter review** | ⏳ Pending | DEE + Pingpong |
| **Final go-live approval** | ⏳ Pending | DEE |

### Go-Live Checklist

- [ ] Wallet funded with USDC (Polygon)
- [ ] Token allowances approved (EOA signature)
- [ ] Risk parameters confirmed (position size, daily loss limit)
- [ ] Switch `DRY_RUN=false` in config
- [ ] Monitor first 10 trades manually
- [ ] Enable Telegram trade notifications

---

## Sources

- MEMORY.md (April 4-5 sessions)
- `memory/2026-04-04.md` (WebSocket fixes)
- `memory/2026-04-05.md` (bot status)
- AWS Console (instance metadata)
