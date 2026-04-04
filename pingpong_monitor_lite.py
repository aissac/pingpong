#!/usr/bin/env python3
"""
Pingpong HFT Monitor - Lightweight Version
Tracks state via tail -f instead of reading entire log
"""

import os
import json
import time
import requests
from datetime import datetime

CONFIG_FILE = '/home/ubuntu/polymarket-hft-engine/config/pingpong.json'
LOG_FILE = '/home/ubuntu/polymarket-hft-engine/nohup_v2.out'
STATE_FILE = '/home/ubuntu/polymarket-hft-engine/config/monitor_state.json'

def load_config():
    config = {'telegram_token': '', 'telegram_chat_id': ''}
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE) as f:
                data = json.load(f)
                config['telegram_token'] = data.get('telegram_token', '')
                config['telegram_chat_id'] = data.get('telegram_chat_id', '')
        except:
            pass
    return config

def send_telegram(text, config):
    if not config['telegram_token'] or not config['telegram_chat_id']:
        return False
    url = f"https://api.telegram.org/bot{config['telegram_token']}/sendMessage"
    try:
        r = requests.post(url, json={'chat_id': config['telegram_chat_id'], 'text': text, 'parse_mode': 'Markdown'}, timeout=10)
        return r.status_code == 200
    except:
        return False

def main():
    config = load_config()
    
    # Load state
    state = {'last_edges': 0, 'last_check': time.time()}
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE) as f:
                state.update(json.load(f))
        except:
            pass
    
    # Read ONLY last 100 lines (not entire log!)
    try:
        with open(LOG_FILE, 'rb') as f:
            f.seek(0, 2)
            size = f.tell()
            # Read last 50KB max
            f.seek(max(0, size - 50000))
            content = f.read().decode('utf-8', errors='ignore')
            lines = content.split('\n')[-100:]
    except Exception as e:
        print(f"Error: {e}")
        return
    
    # Count edges in last 100 lines
    edge_count = sum(1 for line in lines if '[EDGE]' in line)
    now = time.time()
    elapsed = now - state['last_check']
    
    # Calculate rate (edges per minute)
    new_edges = edge_count  # Just count recent edges
    rate = int(new_edges / elapsed * 60) if elapsed > 0 else 0
    
    # Get pair_checks from last line
    pair_checks = 0
    for line in reversed(lines):
        if 'pair_checks=' in line:
            try:
                pair_checks = int(line.split('pair_checks=')[1].split()[0])
            except:
                pass
            break
    
    # Generate report
    now_str = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
    report = f"""```
[🏓 PINGPONG HFT - Status]
Time: {now_str}

=== STATUS ===
Recent Edges: {new_edges}
Edge Rate: ~{rate}/min
Pair Checks: {pair_checks:,}

=== SYSTEM ===
Mode: DRY_RUN
Threshold: $0.94
```"""
    
    print(report)
    send_telegram(report, config)
    
    # Save state
    state['last_edges'] = edge_count
    state['last_check'] = now
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f)

if __name__ == '__main__':
    main()
