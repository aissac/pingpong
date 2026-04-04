#!/usr/bin/env python3
"""
Pingpong HFT Monitor - Lightweight Version
Uses tail -f approach (zero memory) + HTTP metrics endpoint (future)

Key fixes:
1. Seeks to end of file on startup (doesn't read 8GB into memory)
2. Only processes new lines as they arrive
3. Telegram alerts for critical events only
"""

import os
import json
import time
import requests
from datetime import datetime

CONFIG_FILE = '/home/ubuntu/polymarket-hft-engine/config/pingpong.json'
LOG_FILE = '/home/ubuntu/polymarket-hft-engine/nohup_v2.out'
STATE_FILE = '/home/ubuntu/polymarket-hft-engine/config/monitor_state.json'

# Alert thresholds
ALERT_EDGES_PER_MIN = 1000  # Alert if edges/minute drops below this

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

def tail_file(filepath):
    """
    Generator that yields new lines as they arrive.
    Like `tail -f` - starts at end of file, zero memory usage.
    """
    with open(filepath, 'r') as f:
        # Seek to end immediately - don't read existing content
        f.seek(0, os.SEEK_END)
        
        while True:
            line = f.readline()
            if not line:
                time.sleep(0.1)  # Brief sleep to avoid CPU spin
                continue
            yield line

def main():
    config = load_config()
    
    # Load state for tracking
    state = {
        'last_heartbeat': None,
        'edges_since_heartbeat': 0,
        'errors_since_heartbeat': 0,
        'last_alert_time': 0
    }
    
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE) as f:
                state.update(json.load(f))
        except:
            pass
    
    print(f"[Monitor] Starting lightweight tail on {LOG_FILE}")
    print(f"[Monitor] Alerts for: errors, edge rate drops")
    
    heartbeat_count = 0
    
    for line in tail_file(LOG_FILE):
        line = line.strip()
        
        # Parse heartbeat lines (1 per second)
        if line.startswith('[HB]'):
            heartbeat_count += 1
            state['last_heartbeat'] = time.time()
            
            # Every 5 heartbeats (5 seconds), check status
            if heartbeat_count % 5 == 0:
                parts = line.split()
                # Extract edges/s from heartbeat
                try:
                    edges_idx = parts.index('edges/s:') + 1
                    edges_per_sec = int(parts[edges_idx].rstrip(','))
                    
                    # Alert if edge rate drops below threshold
                    if edges_per_sec < ALERT_EDGES_PER_MIN // 60:
                        now = time.time()
                        if now - state['last_alert_time'] > 300:  # Max 1 alert per 5 min
                            send_telegram(f"⚠️ Edge rate dropped: {edges_per_sec}/sec", config)
                            state['last_alert_time'] = now
                except:
                    pass
        
        # Parse edge detections (count for summary)
        elif '[EDGE]' in line:
            state['edges_since_heartbeat'] += 1
        
        # Parse errors (immediate alert)
        elif 'ERROR' in line or 'panic' in line.lower() or 'fail' in line.lower():
            state['errors_since_heartbeat'] += 1
            now = time.time()
            if now - state['last_alert_time'] > 60:  # Max 1 error alert per minute
                send_telegram(f"🚨 Error in log: {line[:100]}", config)
                state['last_alert_time'] = now
        
        # Save state periodically (every 100 lines)
        if heartbeat_count % 100 == 0:
            with open(STATE_FILE, 'w') as f:
                json.dump(state, f)

if __name__ == '__main__':
    main()