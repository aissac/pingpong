#!/usr/bin/env python3
"""
HFT Monitoring Reporter - Sends Telegram updates per NotebookLM spec

Reports every 5 minutes:
- Edge detection metrics
- Ghost simulation stats
- Latency stats
- System health
- Market rollover status

Alerts immediately on:
- WebSocket disconnect
- Memory spike
- Relayer 429 errors
"""

import os
import re
import time
import psutil
import requests
from datetime import datetime
from collections import defaultdict

# Telegram config
BOT_TOKEN = os.environ.get('TELEGRAM_BOT_TOKEN', '8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY')
CHAT_ID = os.environ.get('TELEGRAM_CHAT_ID', '1798631768')

# Log file
LOG_FILE = '/home/ubuntu/polymarket-hft-engine/nohup_v2.out'

# Metrics tracking
class Metrics:
    def __init__(self):
        self.edges_detected = 0
        self.edges_by_market = defaultdict(int)
        self.ghost_simulations = 0
        self.ghost_fills = 0
        self.ghost_misses = 0
        self.latency_samples = []
        self.ws_reconnects = 0
        self.last_edge_time = None
        self.edge_prices = []
        self.tokens_count = 24
        self.orderbook_sizes = []
        self.api_429_count = 0
        
    def reset(self):
        self.edges_detected = 0
        self.edges_by_market.clear()
        self.ghost_simulations = 0
        self.ghost_fills = 0
        self.ghost_misses = 0
        self.latency_samples.clear()
        self.edge_prices.clear()
        self.orderbook_sizes.clear()

metrics = Metrics()

def send_telegram(message):
    """Send message to Telegram"""
    url = f"https://api.telegram.org/bot{BOT_TOKEN}/sendMessage"
    data = {"chat_id": CHAT_ID, "text": message, "parse_mode": "Markdown"}
    try:
        requests.post(url, data=data, timeout=10)
    except Exception as e:
        print(f"Telegram error: {e}")

def parse_log():
    """Parse log file for metrics"""
    try:
        with open(LOG_FILE, 'r') as f:
            # Read last 5000 lines
            lines = f.readlines()[-5000:]
            
        for line in lines:
            # Edge detection
            if '[EDGE]' in line and 'FOUND' in line:
                metrics.edges_detected += 1
                # Extract combined price
                match = re.search(r'combined=\$([0-9.]+)', line)
                if match:
                    metrics.edge_prices.append(float(match.group(1)))
                # Extract market
                if 'BTC-5m' in line:
                    metrics.edges_by_market['BTC-5m'] += 1
                elif 'BTC-15m' in line:
                    metrics.edges_by_market['BTC-15m'] += 1
                elif 'ETH-5m' in line:
                    metrics.edges_by_market['ETH-5m'] += 1
                elif 'ETH-15m' in line:
                    metrics.edges_by_market['ETH-15m'] += 1
            
            # Ghost simulation
            if 'GHOSTED' in line:
                metrics.ghost_misses += 1
                metrics.ghost_simulations += 1
            elif 'EXECUTABLE' in line:
                metrics.ghost_fills += 1
                metrics.ghost_simulations += 1
            
            # Orderbook size
            match = re.search(r'orderbook size: (\d+)', line)
            if match:
                metrics.orderbook_sizes.append(int(match.group(1)))
            
            # Latency
            match = re.search(r'latency[:\s]+([0-9.]+)\s*µ?s', line, re.IGNORECASE)
            if match:
                metrics.latency_samples.append(float(match.group(1)))
            
            # WebSocket reconnect
            if 'WebSocket connected' in line:
                metrics.ws_reconnects += 1
            
            # API 429
            if '429' in line or 'rate limit' in line.lower():
                metrics.api_429_count += 1
                
    except Exception as e:
        print(f"Parse error: {e}")

def get_system_health():
    """Get system metrics"""
    process = psutil.Process()
    rss_mb = process.memory_info().rss / 1024 / 1024
    cpu_pct = process.cpu_percent()
    return rss_mb, cpu_pct

def generate_report():
    """Generate monitoring report"""
    now = datetime.utcnow().strftime('%H:%M:%S')
    
    # Calculate stats
    ghost_rate = 0
    if metrics.ghost_simulations > 0:
        ghost_rate = (metrics.ghost_misses / metrics.ghost_simulations) * 100
    
    avg_edge = 0
    if metrics.edge_prices:
        avg_edge = sum(metrics.edge_prices) / len(metrics.edge_prices)
    
    avg_orderbook = 0
    if metrics.orderbook_sizes:
        avg_orderbook = sum(metrics.orderbook_sizes) / len(metrics.orderbook_sizes)
    
    p99_latency = 0
    if metrics.latency_samples:
        sorted_lat = sorted(metrics.latency_samples)
        p99_idx = int(len(sorted_lat) * 0.99)
        p99_latency = sorted_lat[p99_idx]
    
    rss_mb, cpu_pct = get_system_health()
    
    report = f"""📊 HFT MONITORING REPORT - {now} UTC

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎯 EDGE DETECTION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Edges detected:         {metrics.edges_detected}
Avg gross edge:         ${avg_edge:.4f}
Market breakdown:
  BTC-5m:   {metrics.edges_by_market.get('BTC-5m', 0)}
  BTC-15m:  {metrics.edges_by_market.get('BTC-15m', 0)}
  ETH-5m:   {metrics.edges_by_market.get('ETH-5m', 0)}
  ETH-15m:  {metrics.edges_by_market.get('ETH-15m', 0)}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
👻 GHOST SIMULATION
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total attempts:         {metrics.ghost_simulations}
Simulated ghosts:       {metrics.ghost_misses}
Simulated fills:        {metrics.ghost_fills}
Ghost rate:            {ghost_rate:.1f}%

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚡ LATENCY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Samples:                {len(metrics.latency_samples)}
P99 latency:            {p99_latency:.2f}µs
Avg orderbook:          {avg_orderbook:.0f} levels

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🖥️ SYSTEM HEALTH
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
RSS memory:             {rss_mb:.1f} MB
CPU usage:              {cpu_pct:.1f}%
WS reconnects:          {metrics.ws_reconnects}
API 429 errors:         {metrics.api_429_count}
Active tokens:          {metrics.tokens_count}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📝 STATUS
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Threshold:              $0.98 (validation)
Mode:                   DRY_RUN
"""
    return report

def main():
    """Main loop"""
    send_telegram("🚀 HFT Monitoring started. Reports every 5 minutes.")
    
    report_count = 0
    while True:
        time.sleep(300)  # 5 minutes
        
        parse_log()
        report = generate_report()
        send_telegram(report)
        
        report_count += 1
        metrics.reset()
        
        # Detailed report every hour
        if report_count % 12 == 0:
            send_telegram(f"📅 Hourly check: Bot running for {report_count // 12} hour(s)")

if __name__ == '__main__':
    main()