#!/usr/bin/env python3
"""HFT Reporter V4 - Reads from new binary log"""

import os
import re
from datetime import datetime, timezone

BOT_TOKEN = "8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY"
CHAT_ID = "1798631768"
LOG_FILE = "/home/ubuntu/polymarket-hft-engine/nohup_v2.out"

def send_message(text: str):
    import urllib.request
    import json
    
    url = f"https://api.telegram.org/bot{BOT_TOKEN}/sendMessage"
    payload = {"chat_id": CHAT_ID, "text": text}
    
    try:
        req = urllib.request.Request(url, data=json.dumps(payload).encode(), 
                                      headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(req, timeout=10) as resp:
            return resp.status == 200
    except Exception as e:
        print(f"[TG] Error: {e}")
        return False

def get_hft_stats():
    """Parse log for HFT stats"""
    try:
        with open(LOG_FILE, "r") as f:
            lines = f.readlines()[-500:]
        
        stats = {
            "edges": 0,
            "orderbook_tokens": None,
            "with_both": None,
            "with_ask_only": None,
            "markets": 0,
            "tokens": 0,
            "mode": "DRY_RUN",
            "threshold": "$0.98",
            "last_edge": None
        }
        
        for line in lines:
            # Orderbook state
            if "[HFT] Orderbook state:" in line:
                match = re.search(r"(\d+) tokens", line)
                if match:
                    stats["orderbook_tokens"] = int(match.group(1))
            
            if "with both bid+ask" in line:
                match = re.search(r"(\d+)", line)
                if match:
                    stats["with_both"] = int(match.group(1))
            
            if "with ask only" in line:
                match = re.search(r"(\d+)", line)
                if match:
                    stats["with_ask_only"] = int(match.group(1))
            
            # Markets/tokens
            if "Found" in line and "markets" in line:
                match = re.search(r"Found (\d+)", line)
                if match:
                    stats["markets"] = int(match.group(1))
            
            if "Fetched" in line and "tokens" in line:
                match = re.search(r"Fetched (\d+)", line)
                if match:
                    stats["tokens"] = int(match.group(1))
            
            # Edge detection
            if "[EDGE]" in line:
                stats["edges"] += 1
                match = re.search(r"Combined ASK = \$([0-9.]+)", line)
                if match:
                    stats["last_edge"] = float(match.group(1))
        
        return stats
    except Exception as e:
        print(f"Error: {e}")
        return None

def format_report(stats):
    if not stats:
        return "Unable to parse HFT stats"
    
    msg = "📊 HFT Bot Status (V2)\n"
    msg += "━━━━━━━━━━━━━━━━━━━━\n"
    msg += f"🕐 {datetime.now(timezone.utc).strftime('%Y-%m-%d %H:%M:%S UTC')}\n\n"
    
    msg += "📡 Markets\n"
    msg += f"  markets: {stats['markets']}\n"
    msg += f"  tokens: {stats['tokens']}\n\n"
    
    if stats["orderbook_tokens"]:
        msg += "📊 Orderbook\n"
        msg += f"  tokens: {stats['orderbook_tokens']}\n"
        if stats["with_both"]:
            msg += f"  with bid+ask: {stats['with_both']}\n"
        if stats["with_ask_only"]:
            msg += f"  ask only: {stats['with_ask_only']}\n"
        msg += "\n"
    
    if stats["edges"] > 0:
        msg += "🎯 Edges Detected\n"
        msg += f"  count: {stats['edges']}\n"
        if stats["last_edge"]:
            msg += f"  last: ${stats['last_edge']:.4f}\n"
        msg += "\n"
    
    msg += "🛡️ Safety\n"
    msg += f"  mode: {stats['mode']}\n"
    msg += f"  threshold: {stats['threshold']}\n"
    
    return msg

if __name__ == "__main__":
    stats = get_hft_stats()
    if stats:
        report = format_report(stats)
        send_message(report)
        print(f"[TG] Sent report at {datetime.now(timezone.utc).isoformat()}")
    else:
        print("[TG] No stats available")