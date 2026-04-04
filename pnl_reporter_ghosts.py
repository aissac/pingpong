#!/usr/bin/env python3
"""
PnL Reporter - Combined from HFT (latency) + Old Binary (opportunities + ghosts)
Parse ACTUAL ghost rates from ghost_simulator output.
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
    """Parse old binary log for REALISTIC opportunities and ACTUAL ghost rates."""
    try:
        result = subprocess.check_output(['tail', '-1000', OLD_LOG]).decode()
        
        # Parse ghost simulation results
        ghosted = len(re.findall(r'GHOSTED:', result))
        executable = len(re.findall(r'EXECUTABLE:', result))
        partial = len(re.findall(r'PARTIAL:', result))
        total_simulated = ghosted + executable + partial
        
        ghost_rate = (ghosted / total_simulated * 100) if total_simulated > 0 else 0
        
        # Parse realistic opportunities (edge 6-50%)
        pattern = r'YES: \$([0-9.]+) \+ NO: \$([0-9.]+) = \$([0-9.]+)'
        matches = re.findall(pattern, result)
        
        realistic_opps = []
        for yes, no, combined in matches:
            combined_val = float(combined)
            yes_val = float(yes)
            no_val = float(no)
            
            # Filter realistic opportunities
            if combined_val < 0.90 or combined_val > 0.98:
                continue
            
            if yes_val < 0.01 or no_val < 0.01:
                continue
            
            if combined_val > 0:
                edge = (1.00 - combined_val) / combined_val * 100
            else:
                continue
            
            if edge < 6 or edge > 50:
                continue
            
            realistic_opps.append({'combined': combined_val, 'edge': edge})
        
        opportunities = len(realistic_opps)
        avg_edge = sum(o['edge'] for o in realistic_opps) / opportunities if opportunities > 0 else 0
        
        # Executable after ghost filtering
        executable_rate = (executable / total_simulated * 100) if total_simulated > 0 else 65
        executable_opps = int(opportunities * executable_rate / 100)
        
        return {
            'total': len(matches),
            'opportunities': opportunities,
            'avg_edge': avg_edge,
            'ghosted': ghosted,
            'executable_sim': executable,
            'partial': partial,
            'ghost_rate': ghost_rate,
            'executable_opps': executable_opps
        }
    except Exception as e:
        return {'total': 0, 'opportunities': 0, 'avg_edge': 0, 'ghosted': 0, 'executable_sim': 0, 'partial': 0, 'ghost_rate': 0, 'executable_opps': 0}

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

👻 Ghost Simulation:
├ Ghosted: {opp['ghosted']}
├ Executable: {opp['executable_sim']}
├ Partial: {opp['partial']}
├ Ghost rate: {opp['ghost_rate']:.1f}%
└ Executable opps: {opp['executable_opps']}

📈 Markets: BTC/ETH 5m & 15m Up/Down"""
    
    url = f'https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendMessage'
    requests.post(url, data={'chat_id': TELEGRAM_CHAT_ID, 'text': report, 'parse_mode': 'Markdown'})
    print(f'[{now}] Report sent successfully')

if __name__ == '__main__':
    send_report()