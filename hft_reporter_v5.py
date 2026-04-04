#!/usr/bin/env python3
"""HFT Reporter V5 - Enhanced monitoring with actionable insights"""

import os
import re
from datetime import datetime, timezone
import urllib.request
import json

BOT_TOKEN = "8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY"
CHAT_ID = "1798631768"
LOG_FILE = "/home/ubuntu/polymarket-hft-engine/nohup_v2.out"

def send_message(text: str):
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
            lines = f.readlines()[-1000:]
        
        stats = {
            # Market tracking
            "markets": 0,
            "tokens": 0,
            
            # Orderbook state
            "orderbook_tokens": 0,
            "with_both": 0,
            "with_ask_only": 0,
            
            # Edge detection
            "edges": 0,
            "last_edge_price": None,
            "edges_last_5min": 0,
            
            # Combined ASK tracking
            "combined_ask_min": None,
            "combined_ask_max": None,
            "combined_ask_values": [],
            
            # Market hours check
            "is_market_hours": False,
            
            # Uptime
            "start_time": None,
            "uptime_minutes": 0
        }
        
        now = datetime.now(timezone.utc)
        stats["is_market_hours"] = now.weekday() < 5 and 8 <= now.hour < 22  # Mon-Fri, 8AM-10PM UTC
        
        for line in lines:
            # Markets/tokens
            if "Found" in line and "markets" in line:
                match = re.search(r"Found (\d+)", line)
                if match:
                    stats["markets"] = int(match.group(1))
            
            if "Fetched" in line and "tokens" in line:
                match = re.search(r"Fetched (\d+)", line)
                if match:
                    stats["tokens"] = int(match.group(1))
            
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
            
            # Edge detection
            if "[EDGE]" in line:
                stats["edges"] += 1
                match = re.search(r"Combined ASK = \$([0-9.]+)", line)
                if match:
                    price = float(match.group(1))
                    stats["last_edge_price"] = price
                    stats["combined_ask_values"].append(price)
                    
                    if stats["combined_ask_min"] is None or price < stats["combined_ask_min"]:
                        stats["combined_ask_min"] = price
                    if stats["combined_ask_max"] is None or price > stats["combined_ask_max"]:
                        stats["combined_ask_max"] = price
            
            # Start time
            if "Starting hot path" in line and stats["start_time"] is None:
                # Try to parse timestamp from log
                pass
        
        # Calculate uptime (estimate from log)
        stats["uptime_minutes"] = max(1, stats["edges"] // 2 if stats["edges"] > 0 else 1)
        
        return stats
    except Exception as e:
        print(f"Error: {e}")
        return None

def format_report(stats):
    if not stats:
        return "Unable to parse HFT stats"
    
    now = datetime.now(timezone.utc)
    
    msg = f"📊 HFT Monitor - {now.strftime('%H:%M UTC')}\n"
    msg += "━━━━━━━━━━━━━━━━━━━━\n\n"
    
    # Market status
    status = "🟢 ACTIVE" if stats["is_market_hours"] else "🔴 OFF-HOURS"
    msg += f"Status: {status}\n"
    msg += f"Markets: {stats['markets']} | Tokens: {stats['tokens']}\n\n"
    
    # Orderbook health
    msg += "📈 Orderbook Health\n"
    if stats["orderbook_tokens"] > 0:
        pct_both = (stats["with_both"] / stats["orderbook_tokens"]) * 100
        msg += f"  {stats['with_both']}/{stats['orderbook_tokens']} complete ({pct_both:.0f}%)\n"
        
        if pct_both >= 50:
            msg += "  ✅ Healthy\n"
        elif pct_both >= 25:
            msg += "  ⚠️ Partial\n"
        else:
            msg += "  🔴 Weak\n"
    msg += "\n"
    
    # Edge detection
    if stats["edges"] > 0:
        msg += f"🎯 Edges Detected: {stats['edges']}\n"
        
        if stats["last_edge_price"]:
            msg += f"  Last: ${stats['last_edge_price']:.4f}\n"
        
        if stats["combined_ask_min"] and stats["combined_ask_max"]:
            spread = stats["combined_ask_max"] - stats["combined_ask_min"]
            msg += f"  Range: ${stats['combined_ask_min']:.4f} - ${stats['combined_ask_max']:.4f}\n"
            msg += f"  Spread: ${spread:.4f}\n"
        
        # Trend indicator
        if stats["combined_ask_values"] and len(stats["combined_ask_values"]) >= 2:
            recent = stats["combined_ask_values"][-5:]
            if len(recent) >= 2:
                if recent[-1] < recent[0]:
                    msg += "  📉 Trend: DOWN (good)\n"
                elif recent[-1] > recent[0]:
                    msg += "  📈 Trend: UP\n"
                else:
                    msg += "  ➡️ Trend: FLAT\n"
        msg += "\n"
    else:
        msg += "🎯 Edges: None yet\n\n"
    
    # Threshold reminder
    msg += "⚙️ Config\n"
    msg += "  Mode: DRY_RUN\n"
    msg += "  Threshold: $0.98\n"
    msg += "  Max Position: $5\n"
    
    # Action items based on state
    msg += "\n━━━━━━━━━━━━━━━━━━━━\n"
    
    if stats["is_market_hours"] and stats["edges"] == 0:
        msg += "💡 TIP: Market hours, no edges.\n"
        msg += "   Combined ASK may be > $0.98\n"
    elif not stats["is_market_hours"] and stats["edges"] == 0:
        msg += "💡 TIP: Off-hours, low activity.\n"
        msg += "   Edges expected during market hours\n"
    elif stats["edges"] > 0:
        avg_ask = sum(stats["combined_ask_values"]) / len(stats["combined_ask_values"]) if stats["combined_ask_values"] else 0
        if avg_ask < 0.96:
            msg += f"⚡ AVG Combined ASK: ${avg_ask:.4f}\n"
            msg += "   Edges are TIGHT - ready for live!\n"
    
    return msg

if __name__ == "__main__":
    stats = get_hft_stats()
    if stats:
        report = format_report(stats)
        send_message(report)
        print(f"[TG] Sent report at {datetime.now(timezone.utc).isoformat()}")
    else:
        print("[TG] No stats available")