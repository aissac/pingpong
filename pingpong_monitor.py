#!/usr/bin/env python3
"""
Pingpong HFT Monitor - Quant-Grade Telegram Reports
"""

import os
import re
import json
import time
import requests
from datetime import datetime
from collections import defaultdict

# Config file (permanent location)
CONFIG_FILE = '/home/ubuntu/polymarket-hft-engine/config/pingpong.json'
LOG_FILE = '/home/ubuntu/polymarket-hft-engine/nohup_v2.out'
STATE_FILE = '/home/ubuntu/polymarket-hft-engine/config/monitor_state.json'

def load_config():
    """Load config from file."""
    config = {'telegram_token': '', 'telegram_chat_id': ''}
    
    if os.path.exists(CONFIG_FILE):
        try:
            with open(CONFIG_FILE) as f:
                data = json.load(f)
                config['telegram_token'] = data.get('telegram_token', '')
                config['telegram_chat_id'] = data.get('telegram_chat_id', '')
        except Exception as e:
            print(f"Error loading config: {e}")
    
    return config

def parse_log(log_content):
    """Parse log and extract metrics."""
    messages = 0
    pair_checks = 0
    edges = []
    edge_counts = defaultdict(int)
    edge_total = 0
    
    # Find latest message count
    msg_match = re.search(r'Received (\d+) messages', log_content)
    if msg_match:
        messages = int(msg_match.group(1))
    
    # Find latest pair_checks
    pc_match = re.search(r'pair_checks=(\d+)', log_content)
    if pc_match:
        pair_checks = int(pc_match.group(1))
    
    # Find edges in range $0.90-$0.98
    edge_pattern = r'Combined ASK = \$([0-9.]+) \(YES=([0-9.]+)¢, NO=([0-9.]+)¢\)'
    for match in re.finditer(edge_pattern, log_content):
        try:
            combined = float(match.group(1))
            yes_price = float(match.group(2))
            no_price = float(match.group(3))
            
            if 0.90 <= combined <= 0.98:
                edges.append({'combined': combined, 'yes': yes_price, 'no': no_price})
                if combined <= 0.92:
                    edge_counts['$0.90-0.92'] += 1
                elif combined <= 0.95:
                    edge_counts['$0.93-0.95'] += 1
                else:
                    edge_counts['$0.96-0.98'] += 1
        except:
            continue
    
    # Count total EDGE DETECTED messages
    edge_total = len(re.findall(r'EDGE DETECTED', log_content))
    
    return {
        'messages': messages,
        'pair_checks': pair_checks,
        'edges': edges[-20:],
        'edge_counts': dict(edge_counts),
        'total_edges': edge_total
    }

def generate_report(metrics, state):
    """Generate quant-grade report."""
    now = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
    
    # Calculate rates
    msg_rate = 0
    pair_rate = 0
    edge_rate = 0
    if state.get('last_messages', 0) > 0:
        elapsed = time.time() - state.get('last_timestamp', time.time())
        if elapsed > 0:
            msg_rate = int((metrics['messages'] - state['last_messages']) / elapsed)
            pair_rate = int((metrics['pair_checks'] - state['last_pair_checks']) / elapsed)
            edge_rate = int((metrics['total_edges'] - state.get('last_edges', 0)) / elapsed)
    
    # Count new edges in this period
    new_edges = metrics['total_edges'] - state.get('last_edges', 0)
    
    report = f"""```
[ 🏓 PINGPONG HFT ENGINE ]
MODE: DRY_RUN | TIME: {now}

=== ⚡ SYSTEM PERFORMANCE ===
Msg Rate    : {msg_rate:,}/sec
Pair Checks : {metrics['pair_checks']:,} ({pair_rate:,}/sec)
Total Edges : {metrics['total_edges']:,} ({edge_rate:,}/sec)

=== 🎯 EDGE DISTRIBUTION (ASK < $0.98) ===
"""
    
    for bucket in ['$0.90-0.92', '$0.93-0.95', '$0.96-0.98']:
        count = metrics['edge_counts'].get(bucket, 0)
        bar = '█' * min(count // 1000, 10) + ('▏' if count > 10000 else '')
        label = '[High α]' if bucket == '$0.90-0.92' else '[Mid α]' if bucket == '$0.93-0.95' else '[Low α]'
        report += f"{bucket:12} : {bar:10} ({count:>5}) {label}\n"
    
    report += "\n=== 💧 LATEST EDGES ===\n"
    report += "YES(¢)  NO(¢)   Combined\n"
    report += "------  ------  --------\n"
    
    for edge in metrics['edges'][-5:]:
        report += f"{edge['yes']:>6.1f}  {edge['no']:>6.1f}  ${edge['combined']:>7.4f}\n"
    
    ghost_rate = 60
    report += f"\n=== 📊 ESTIMATED FILL RATE ===\n"
    report += f"Ghost Rate: ~{ghost_rate}% | Executable: ~{100-ghost_rate}%\n"
    report += f"Real Opps: ~{int(metrics['total_edges'] * (100-ghost_rate) / 100):,}\n"
    
    # Add new edges this period
    if new_edges > 0:
        report += f"\n🔥 New Edges (last 3min): {new_edges:,}\n"
    
    report += "```"
    
    return report

def send_telegram(report, config):
    """Send via Telegram."""
    if not config['telegram_token'] or not config['telegram_chat_id']:
        print("ERROR: Missing Telegram credentials")
        return False
    
    url = f"https://api.telegram.org/bot{config['telegram_token']}/sendMessage"
    payload = {
        'chat_id': config['telegram_chat_id'],
        'text': report,
        'parse_mode': 'Markdown'
    }
    
    try:
        r = requests.post(url, json=payload, timeout=10)
        if r.status_code == 200:
            print("✅ Report sent to Telegram")
            return True
        else:
            print(f"❌ Telegram error: {r.status_code} {r.text}")
            return False
    except Exception as e:
        print(f"❌ Error: {e}")
        return False

def main():
    # Load config
    config = load_config()
    
    # Load state
    state = {'last_messages': 0, 'last_pair_checks': 0, 'last_edges': 0, 'last_timestamp': time.time()}
    if os.path.exists(STATE_FILE):
        try:
            with open(STATE_FILE) as f:
                state.update(json.load(f))
        except:
            pass
    
    # Read log (last 500KB for better edge capture)
    try:
        with open(LOG_FILE, 'rb') as f:
            f.seek(0, 2)
            size = f.tell()
            f.seek(max(0, size - 500000))
            content = f.read().decode('utf-8', errors='ignore')
    except Exception as e:
        print(f"❌ Error reading log: {e}")
        return
    
    # Parse and send
    metrics = parse_log(content)
    report = generate_report(metrics, state)
    print(report)
    
    send_telegram(report, config)
    
    # Save state
    state['last_messages'] = metrics['messages']
    state['last_pair_checks'] = metrics['pair_checks']
    state['last_edges'] = metrics['total_edges']
    state['last_timestamp'] = time.time()
    with open(STATE_FILE, 'w') as f:
        json.dump(state, f)

if __name__ == '__main__':
    main()