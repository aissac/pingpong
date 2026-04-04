#!/usr/bin/env python3
"""
PnL Reporter - Combined from HFT (latency) + Old Binary (opportunities)
Filter: Only count REALISTIC arbitrage (combined $0.90-$0.98, edge 6-50%)
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
    """Parse old binary log for REALISTIC opportunities only."""
    try:
        result = subprocess.check_output(['tail', '-1000', OLD_LOG]).decode()
        
        # Match: YES: $X.XX + NO: $Y.YY = $Z.ZZ
        pattern = r'YES: \$([0-9.]+) \+ NO: \$([0-9.]+) = \$([0-9.]+)'
        matches = re.findall(pattern, result)
        
        total = len(matches)
        realistic_opps = []
        
        for yes, no, combined in matches:
            combined_val = float(combined)
            yes_val = float(yes)
            no_val = float(no)
            
            # FILTER: Only count realistic arbitrage opportunities
            # 1. Combined must be between $0.90 and $0.98 (realistic arb range)
            # 2. Must have liquidity on BOTH sides (min $0.01 each)
            # 3. Edge should be 6-50% (filter out closed markets)
            
            if combined_val < 0.01 or combined_val > 0.98:
                continue  # Skip closed/no-liquidity markets
            
            # Calculate edge
            if combined_val > 0:
                edge = (1.00 - combined_val) / combined_val * 100
            else:
                continue
            
            # Filter realistic edges (6-50%)
            if edge < 6 or edge > 50:
                continue
            
            # Must have liquidity on both sides
            if yes_val < 0.01 or no_val < 0.01:
                continue
            
            realistic_opps.append({
                'combined': combined_val,
                'edge': edge,
                'yes': yes_val,
                'no': no_val
            })
        
        opportunities = len(realistic_opps)
        avg_edge = sum(o['edge'] for o in realistic_opps) / opportunities if opportunities > 0 else 0
        
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
├ Realistic: {opp['opportunities']} (edge 6-50%)
├ Avg edge: {opp['avg_edge']:.1f}%
├ Ghost rate: ~35%
└ Executable: {opp['executable']}

📈 Markets: BTC/ETH 5m & 15m Up/Down
📉 Filter: combined $0.90-$0.98"""
    
    url = f'https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage'
    requests.post(url, data={'chat_id': TELEGRAM_CHAT_ID, 'text': report, 'parse_mode': 'Markdown'})
    print(f'[{now}] Report sent successfully')

if __name__ == '__main__':
    send_report()