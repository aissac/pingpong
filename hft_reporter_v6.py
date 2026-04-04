#!/usr/bin/env python3
"""HFT Reporter V6 - Professional monitoring with actionable insights"""

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
            lines = f.readlines()[-2000:]
        
        stats = {
            # Market tracking
            "markets": 0,
            "tokens": 0,
            "market_list": [],
            
            # Orderbook state
            "orderbook_tokens": 0,
            "with_both": 0,
            "with_ask_only": 0,
            "with_bid_only": 0,
            
            # Edge detection
            "edges": 0,
            "last_edge_price": None,
            "last_edge_time": None,
            "combined_ask_values": [],
            "combined_ask_min": None,
            "combined_ask_max": None,
            
            # Market context
            "is_market_hours": False,
            "session": "OFF-HOURS",
            
            # WebSocket health
            "ws_messages": 0,
            "ws_reconnects": 0,
        }
        
        now = datetime.now(timezone.utc)
        hour = now.hour
        
        # Market hours detection
        if 8 <= hour < 14:
            stats["session"] = "US PRE-MARKET"
            stats["is_market_hours"] = True
        elif 14 <= hour < 21:
            stats["session"] = "US MARKET"
            stats["is_market_hours"] = True
        elif 21 <= hour < 24 or 0 <= hour < 8:
            stats["session"] = "ASIA/EU"
            stats["is_market_hours"] = False
        
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
            
            # Market list
            if "btc-updown" in line or "eth-updown" in line:
                match = re.search(r"(btc|eth)-updown-(\d+)-(\d+)", line)
                if match:
                    stats["market_list"].append(f"{match.group(1).upper()}-{match.group(2)}")
            
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
            
            if "with bid only" in line:
                match = re.search(r"(\d+)", line)
                if match:
                    stats["with_bid_only"] = int(match.group(1))
            
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
            
            # WebSocket messages
            if "Warmed up after" in line:
                match = re.search(r"after (\d+) messages", line)
                if match:
                    stats["ws_messages"] = int(match.group(1))
            
            # Reconnects
            if "Reconnecting" in line or "WebSocket closed" in line:
                stats["ws_reconnects"] += 1
        
        return stats
    except Exception as e:
        print(f"Error: {e}")
        return None

def format_report(stats):
    if not stats:
        return "Unable to parse HFT stats"
    
    now = datetime.now(timezone.utc)
    
    # Header
    msg = f"📊 15M MARKET SUMMARY | BTC/ETH\n"
    msg += f"Time: {now.strftime('%H:%M UTC')} ({stats['session']})\n\n"
    
    # Market microstructure
    msg += "🧮 MARKET MICROSTRUCTURE:\n"
    msg += f"• Active Markets: {stats['markets']} ({stats['tokens']} tokens)\n"
    
    if stats["combined_ask_values"]:
        avg_ask = sum(stats["combined_ask_values"]) / len(stats["combined_ask_values"])
        msg += f"• Avg Combined ASK: ${avg_ask:.4f}\n"
        if stats["combined_ask_min"]:
            msg += f"  Tightest: ${stats['combined_ask_min']:.4f}\n"
    else:
        msg += "• Avg Combined ASK: --\n"
    
    # Orderbook health
    if stats["orderbook_tokens"] > 0:
        pct_both = (stats["with_both"] / stats["orderbook_tokens"]) * 100
        msg += f"• Orderbook Health: {stats['with_both']}/{stats['orderbook_tokens']} complete ({pct_both:.0f}%)\n"
        
        # Imbalance indicator
        if stats["with_ask_only"] > stats["with_bid_only"] * 2:
            msg += "• OB Imbalance: SELL-skewed (bearish pressure)\n"
        elif stats["with_bid_only"] > stats["with_ask_only"] * 2:
            msg += "• OB Imbalance: BUY-skewed (bullish pressure)\n"
        else:
            msg += "• OB Imbalance: Balanced\n"
    
    msg += "\n"
    
    # Performance
    msg += "⚡ PERFORMANCE:\n"
    msg += f"• Edges Detected: {stats['edges']}\n"
    
    if stats["combined_ask_min"] and stats["combined_ask_max"]:
        spread = stats["combined_ask_max"] - stats["combined_ask_min"]
        msg += f"• Combined ASK Range: ${stats['combined_ask_min']:.4f} - ${stats['combined_ask_max']:.4f}\n"
        msg += f"• Spread: ${spread:.4f}\n"
    
    if stats["last_edge_price"]:
        if stats["last_edge_price"] < 0.96:
            msg += "• ⚠️ EDGE BELOW $0.96!\n"
    
    msg += "\n"
    
    # System health
    msg += "🖥 SYSTEM HEALTH:\n"
    msg += f"• Mode: DRY_RUN\n"
    msg += f"• Threshold: $0.98\n"
    msg += f"• WS Reconnects: {stats['ws_reconnects']}\n"
    
    # Action items
    msg += "\n━━━━━━━━━━━━━━━━━━━━\n"
    
    if stats["is_market_hours"] and stats["edges"] == 0:
        msg += "💡 Market hours, no edges detected.\n"
        msg += "   Combined ASK may be > $0.98\n"
    elif not stats["is_market_hours"] and stats["edges"] == 0:
        msg += "💡 Off-hours, low activity expected.\n"
        msg += "   Monitor during US market hours\n"
    elif stats["edges"] > 0 and stats["combined_ask_values"]:
        avg = sum(stats["combined_ask_values"]) / len(stats["combined_ask_values"])
        if avg < 0.96:
            msg += f"⚡ ACTIVE: Avg ASK ${avg:.4f}\n"
            msg += "   Ready for live threshold ($0.94)\n"
    
    return msg

if __name__ == "__main__":
    stats = get_hft_stats()
    if stats:
        report = format_report(stats)
        send_message(report)
        print(f"[TG] Sent report at {datetime.now(timezone.utc).isoformat()}")
    else:
        print("[TG] No stats available")