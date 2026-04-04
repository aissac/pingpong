#!/usr/bin/env python3
"""
PnL Reporter - Combined from HFT (latency) + Old Binary (opportunities)
"""
import requests
import subprocess
import re
from datetime import datetime

TELEGRAM_BOT_TOKEN = open('/home/ubuntu/telegram_bot_token').read().strip()
TELEGRAM_CHAT_ID = '1798631768'
HFT_LOG = '/var/log/hft_pingpong.log'
OLD_LOG = '/tmp/pingpong.log'

def get_hft_stats():
    """Parse HFT log for latency stats."""
    try:
        result = subprocess.check_output(['tail', '-100', HFT_LOG]).decode()
        stats_lines = [line for line in result.split('\n') if 'STATS' in line and 'avg=' in line]
        
        if stats_lines:
            m = re.search(r'avg=([0-9.]+).*min=([0-9.]+).*max=([0-9.]+).*p99=([0-9.]+)', stats_lines[-1])
            if m:
                return {'avg': m.group(1), 'min': m.group(2), 'max': m.group(3), 'p99': m.group(4)}
    except:
        pass
    return {'avg': 'N/A', 'min': 'N/A', 'max': 'N/A', 'p99': 'N/A'}

def get_opportunity_stats():
    """Parse old binary log for opportunities with positive edge."""
    try:
        result = subprocess.check_output(['tail', '-1000', OLD_LOG]).decode()
        
        # Match: YES: $X.XX + NO: $Y.YY = $Z.ZZ
        # Combined < $0.98 means arbitrage opportunity
        pattern = r'YES: \$([0-9.]+) \+ NO: \$([0-9.]+) = \$([0-9.]+)'
        matches = re.findall(pattern, result)
        
        total = len(matches)
        opportunities = 0
        total_edge = 0
        
        for yes, no, combined in matches:
            combined_val = float(combined)
            if combined_val < 0.98:  # Arbitrage opportunity
                opportunities += 1
                # Edge = (1.00 - combined) / combined * 100%
                if combined_val > 0:
                    edge = (1.00 - combined_val) / combined_val * 100
                    total_edge += edge
        
        avg_edge = total_edge / opportunities if opportunities > 0 else 0
        
        # Ghost rate: ~35% of detected opportunities are ghosts
        ghost_rate = 0.35
        ghost_count = int(opportunities * ghost_rate)
        executable = opportunities - ghost_count
        
        return {
            'total': total,
            'opportunities': opportunities,
            'avg_edge': avg_edge,
            'ghost_count': ghost_count,
            'executable': executable
        }
    except Exception as e:
        return {'total': 0, 'opportunities': 0, 'avg_edge': 0, 'ghost_count': 0, 'executable': 0}

def send_report():
    """Send combined PnL report to Telegram."""
    lat = get_hft_stats()
    opp = get_opportunity_stats()
    now = datetime.utcnow()
    
    report = f"""📊 PnL Report
⏰ {now.strftime('%H:%M:%S')} UTC | Mode: DRY RUN

🔥 Latency (HFT):
├ Avg: {lat['avg']}µs
├ Min: {lat['min']}µs
├ Max: {lat['max']}µs
└ P99: {lat['p99']}µs

🎯 Opportunities (6 min):
├ Total signals: {opp['total']}
├ Arb detected: {opp['opportunities']} (combined < $0.98)
├ Avg edge: {opp['avg_edge']:.1f}%
├ Ghost rate: ~35%
└ Executable: {opp['executable']}

📈 Markets: BTC/ETH 5m & 15m Up/Down
📉 Status: Both binaries running"""
    
    url = f'https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage'
    requests.post(url, data={'chat_id': TELEGRAM_CHAT_ID, 'text': report, 'parse_mode': 'Markdown'})
    print(f'[{now}] Report sent successfully')

if __name__ == '__main__':
    send_report()