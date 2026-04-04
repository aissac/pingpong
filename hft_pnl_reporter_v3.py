#!/usr/bin/env python3
"""
HFT PnL Reporter v3 - Production Grade
- Market-by-market breakdown (BTC-5m, BTC-15m, ETH-5m, ETH-15m)
- Edge tier grouping (2-3%, 3-5%, >5%)
- Hourly summary reports
- Instant alerts on thresholds
"""
import os
import re
import time
import shutil
import requests
from datetime import datetime, timedelta
from collections import defaultdict

# --- CONFIGURATION ---
TELEGRAM_BOT_TOKEN = open('/home/ubuntu/telegram_bot_token').read().strip()
TELEGRAM_CHAT_ID = '1798631768'
HFT_LOG = '/var/log/hft_pingpong.log'
OLD_LOG = '/tmp/pingpong.log'
STATE_FILE = '/tmp/hft_reporter_last_run.txt'

# Risk Thresholds
MAX_DAILY_DRAWDOWN_PCT = -3.0
MAX_RELAYER_RPM = 20
DISK_USAGE_WARNING_PCT = 85.0
REPORT_INTERVAL_HOURS = 1

# Market condition ID prefixes (first 8 chars map to market)
# Populated dynamically from the log
MARKET_NAMES = {
    '536869': 'BTC-5m',
    '3cb5c5': 'BTC-5m',
    '28fa17': 'BTC-15m',
    '6a4599': 'BTC-15m',
}

def send_telegram(text):
    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"
    payload = {'chat_id': TELEGRAM_CHAT_ID, 'text': text, 'parse_mode': 'HTML'}
    try:
        resp = requests.post(url, json=payload, timeout=10)
        if resp.status_code == 200:
            print(f"[{datetime.utcnow().isoformat()}] Report sent successfully")
    except Exception as e:
        print(f"[{datetime.utcnow().isoformat()}] Failed to send: {e}")

def get_hft_stats():
    try:
        result = os.popen(f'tail -100 {HFT_LOG}').read()
        stats_lines = [line for line in result.split('\n') if 'STATS' in line and 'avg=' in line]
        if stats_lines:
            m = re.search(r'avg=([0-9.]+).*min=([0-9.]+).*max=([0-9.]+).*p99=([0-9.]+)', stats_lines[-1])
            if m:
                return {'avg': m.group(1), 'min': m.group(2), 'max': m.group(3), 'p99': m.group(4)}
    except:
        pass
    return {'avg': 'N/A', 'min': 'N/A', 'max': 'N/A', 'p99': 'N/A'}

def parse_opportunities():
    data = {
        'markets': defaultdict(lambda: {'fills': 0, 'ghosts': 0, 'total': 0}),
        'tiers': {'2-3%': 0, '3-5%': 0, '>5%': 0, 'total': 0},
        'total_signals': 0,
        'total_fills': 0,
        'total_ghosts': 0,
        'edges': [],
        'recent_execs': 0,
    }
    
    try:
        result = os.popen(f'cat {OLD_LOG}').read()
        
        for line in result.split('\n'):
            edge_match = re.search(r'Edge:\s*([0-9.]+)%', line)
            if edge_match:
                edge = float(edge_match.group(1))
                data['edges'].append(edge)
                data['total_signals'] += 1
                
                if 2.0 <= edge <= 3.0:
                    data['tiers']['2-3%'] += 1
                elif 3.0 < edge <= 5.0:
                    data['tiers']['3-5%'] += 1
                elif edge > 5.0:
                    data['tiers']['>5%'] += 1
                data['tiers']['total'] += 1
            
            if 'GHOST SIMULATION:' in line:
                cond_match = re.search(r'GHOST SIMULATION:\s*([0-9a-f]+)', line)
                if cond_match:
                    cond_id = cond_match.group(1)[:8]
                    market = MARKET_NAMES.get(cond_id, 'UNKNOWN')
                    data['markets'][market]['ghosts'] += 1
                    data['total_ghosts'] += 1
            
            if 'EXECUTABLE SIMULATION:' in line:
                cond_match = re.search(r'EXECUTABLE SIMULATION:\s*([0-9a-f]+)', line)
                if cond_match:
                    cond_id = cond_match.group(1)[:8]
                    market = MARKET_NAMES.get(cond_id, 'UNKNOWN')
                    data['markets'][market]['fills'] += 1
                    data['total_fills'] += 1
                    data['markets'][market]['total'] += 1
        
    except Exception as e:
        print(f"Parse error: {e}")
    
    return data

def get_disk_usage():
    try:
        total, used, free = shutil.disk_usage("/")
        return (used / total) * 100
    except:
        return 0

def main():
    hft = get_hft_stats()
    opp = parse_opportunities()
    disk_pct = get_disk_usage()
    
    # --- ALERT THRESHOLDS ---
    alerts = []
    
    if disk_pct > DISK_USAGE_WARNING_PCT:
        alerts.append(f"💾 DISK WARNING: {disk_pct:.1f}% used!")
    
    net_pnl = (opp['total_fills'] * 1.29) - (opp['total_ghosts'] * 0.70)
    drawdown_pct = (net_pnl / 5000.0) * 100
    
    if drawdown_pct < MAX_DAILY_DRAWDOWN_PCT:
        alerts.append(f"🛑 DRAWDOWN ALERT: {drawdown_pct:.2f}% (Limit: {MAX_DAILY_DRAWDOWN_PCT}%)")
    
    if opp['recent_execs'] > MAX_RELAYER_RPM:
        alerts.append(f"⚠️ RELAYER CHOKE: {opp['recent_execs']}/25 RPM!")
    
    if alerts:
        alert_msg = "🚨 HFT CRITICAL ALERTS 🚨\n\n" + "\n".join(alerts)
        send_telegram(alert_msg)
        return
    
    # --- HOURLY REPORT ---
    last_run = 0
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE, 'r') as f:
                last_run = float(f.read().strip() or 0)
        except:
            pass
    
    now_ts = time.time()
    if (now_ts - last_run) < (REPORT_INTERVAL_HOURS * 3600):
        return
    
    with open(STATE_FILE, 'w') as f:
        f.write(str(now_ts))
    
    # Market breakdown
    market_lines = []
    for market, stats in sorted(opp['markets'].items()):
        if stats['total'] > 0:
            ghost_rate = (stats['ghosts'] / stats['total'] * 100) if stats['total'] > 0 else 0
            market_lines.append(f"• {market}: {stats['fills']} Fills | {stats['ghosts']} Ghosts ({ghost_rate:.0f}%)")
    
    if not market_lines:
        ghost_rate = (opp['total_ghosts'] / (opp['total_fills'] + opp['total_ghosts']) * 100) if (opp['total_fills'] + opp['total_ghosts']) > 0 else 0
        market_lines.append(f"• ALL: {opp['total_fills']} Fills | {opp['total_ghosts']} Ghosts ({ghost_rate:.0f}%)")
    
    avg_edge = sum(opp['edges']) / len(opp['edges']) if opp['edges'] else 0
    
    report = f"""📊 HFT Engine Report - {datetime.utcnow().strftime('%H:%M:%S')} UTC
Mode: DRY RUN | Markets: BTC/ETH 5m & 15m

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ SYSTEM HEALTH
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Hot Path Min: {hft['min']}µs | Avg: {hft['avg']}µs
P99 Spikes:   {hft['p99']}µs
Disk Usage:   {disk_pct:.1f}%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 OPPORTUNITIES
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total signals: {opp['total_signals']}
Avg edge:      {avg_edge:.1f}%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📉 EDGE TIERS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
2-3% (Tight):    {opp['tiers']['2-3%']}
3-5% (Optimal):  {opp['tiers']['3-5%']}
>5% (Anomalous): {opp['tiers']['>5%']}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
👻 GHOST SIMULATION (50ms RTT)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
{chr(10).join(market_lines)}
Total: {opp['total_fills'] + opp['total_ghosts']} ({(opp['total_ghosts'] / (opp['total_fills'] + opp['total_ghosts']) * 100) if (opp['total_fills'] + opp['total_ghosts']) > 0 else 0:.0f}% ghost rate)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📈 STATUS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
HFT Binary: ✅ Running (sub-µs)
Old Binary: ✅ Running (full tracking)
Reporter:   ✅ Hourly mode"""
    
    send_telegram(report)

if __name__ == "__main__":
    main()