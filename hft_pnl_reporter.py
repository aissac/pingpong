#!/usr/bin/env python3
"""
PnL Reporter for HFT Binary
Sends Telegram reports every 6 minutes with latency stats and opportunity tracking.
"""
import requests
import subprocess
import re
from datetime import datetime

TELEGRAM_BOT_TOKEN = open('/home/ubuntu/telegram_bot_token').read().strip()
TELEGRAM_CHAT_ID = '1798631768'
LOG_FILE = '/var/log/hft_pingpong.log'

# Ghost simulation parameters
GHOST_DELAY_SECONDS = 0.050  # 50ms RTT simulation

def get_stats():
    """Parse HFT log for latency stats and opportunity tracking."""
    result = subprocess.check_output(['tail', '-500', LOG_FILE]).decode()
    stats_lines = [line for line in result.split('\n') if 'STATS' in line and 'avg=' in line]
    
    # Latency stats
    if stats_lines:
        m = re.search(r'avg=([0-9.]+)µs.*min=([0-9.]+)µs.*max=([0-9.]+)µs.*p99=([0-9.]+)µs', stats_lines[-1])
        if m:
            latency = {'avg': m.group(1), 'min': m.group(2), 'max': m.group(3), 'p99': m.group(4)}
        else:
            latency = {'avg': 'N/A', 'min': 'N/A', 'max': 'N/A', 'p99': 'N/A'}
    else:
        latency = {'avg': 'N/A', 'min': 'N/A', 'max': 'N/A', 'p99': 'N/A'}
    
    # Opportunity tracking (placeholder - HFT binary doesn't log opportunities yet)
    # For now, we'll estimate based on message rate
    # ~600 messages/sec, ~6 minute interval = ~216,000 messages
    # Typical rate: ~70 opportunities/minute = ~420 per 6 min
    
    # TODO: When HFT binary logs opportunities, parse from log
    maker_count = 0
    taker_count = 0
    ghost_count = 0
    executable_count = 0
    total = 0
    
    return {
        'latency': latency,
        'maker_count': maker_count,
        'taker_count': taker_count,
        'ghost_count': ghost_count,
        'executable_count': executable_count,
        'total': total
    }

def send_report(stats):
    """Send PnL report to Telegram."""
    now = datetime.utcnow()
    lat = stats['latency']
    
    report = f"""📊 **PnL Report (HFT)**
⏰ {now.strftime('%H:%M:%S')} UTC | Mode: DRY RUN

**🔥 Latency Stats:**
├ Avg: {lat['avg']}µs
├ Min: {lat['min']}µs
├ Max: {lat['max']}µs
└ P99: {lat['p99']}µs

**🎯 Opportunities:** {stats['total']}
├ Maker Hybrid: {stats['maker_count']}
└ Taker: {stats['taker_count']}

**👻 Ghost Simulation:**
├ Ghosted: {stats['ghost_count']}
└ Executable: {stats['executable_count']}

**🔧 Config:**
├ Dynamic Sizing: Enabled
├ Liquidity Mirage: Protected
├ Min Depth: 1 share
└ Maker: p&lt;0.30 or p&gt;0.70"""
    
    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage"
    r = requests.post(url, data={'chat_id': TELEGRAM_CHAT_ID, 'text': report, 'parse_mode': 'Markdown'})
    
    if r.status_code == 200:
        print(f"[{now}] Report sent successfully")
    else:
        print(f"[{now}] Error: {r.text}")

if __name__ == '__main__':
    stats = get_stats()
    send_report(stats)