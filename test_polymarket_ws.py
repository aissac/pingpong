#!/usr/bin/env python3
"""
Polymarket WebSocket Test Script
Tests if we can receive market data from Polymarket CLOB WebSocket
"""

import websocket
import json
import time
import sys

# Use a valid 77-digit clobTokenId from an active BTC market
TARGET_TOKEN_ID = "61747813023874742608857566552896587988293341311109753724364483362037124305138"

messages_received = 0
first_message_time = None

def on_message(ws, message):
    global messages_received, first_message_time
    
    if first_message_time is None:
        first_message_time = time.time()
    
    messages_received += 1
    
    # Print first few messages in full, then just counts
    if messages_received <= 3:
        print(f"\n🟢 [{time.strftime('%X')}] MESSAGE #{messages_received}:")
        try:
            data = json.loads(message)
            print(f"  Type: {data.get('type', 'unknown')}")
            if 'book_snapshot' in data:
                print(f"  Book snapshot with {len(data.get('book_snapshot', []))} levels")
            elif 'price_change' in data:
                print(f"  Price change: {data.get('price_change')}")
            else:
                print(f"  Data: {str(message)[:200]}...")
        except:
            print(f"  Raw: {str(message)[:200]}...")
    elif messages_received == 4:
        print(f"\n🟢 [{time.strftime('%X')}] MESSAGE #4+ (suppressing details)...")
        print(f"  ... (receiving messages successfully)")

def on_error(ws, error):
    print(f"🚨 ERROR: {error}")

def on_close(ws, close_status_code, close_msg):
    print(f"\n⚠️ DISCONNECTED: Status {close_status_code} - {close_msg}")

def on_open(ws):
    print("✅ HTTP 101 Upgrade Successful! Socket Open.")
    
    # The exact payload required by the CLOB WebSocket
    payload = {
        "assets": [TARGET_TOKEN_ID],
        "type": "market"
    }
    
    print(f"📡 Sending subscription payload: {json.dumps(payload)}")
    ws.send(json.dumps(payload))
    print("📡 Subscription sent - waiting for initial_dump...")

if __name__ == "__main__":
    print("=" * 60)
    print("POLYMARKET WEBSOCKET TEST")
    print("=" * 60)
    print(f"Target Token: {TARGET_TOKEN_ID[:50]}...")
    print(f"Endpoint: wss://ws-subscriptions-clob.polymarket.com/ws/market")
    print("")
    
    # Enable trace to see PING/PONG frames
    websocket.enableTrace(False)  # Set to True for debug
    
    ws = websocket.WebSocketApp(
        "wss://ws-subscriptions-clob.polymarket.com/ws/market",
        on_open=on_open,
        on_message=on_message,
        on_error=on_error,
        on_close=on_close,
        header=[
            "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            "Origin: https://polymarket.com"
        ]
    )
    
    print("🔄 Starting WebSocket loop (ping_interval=20, ping_timeout=10)...")
    print("⏱️  Running for 60 seconds...")
    print("")
    
    # CRITICAL: ping_interval keeps Cloudflare from dropping us
    ws.run_forever(ping_interval=20, ping_timeout=10)
    
    # Summary
    print("")
    print("=" * 60)
    print("TEST SUMMARY")
    print("=" * 60)
    
    if messages_received > 0:
        elapsed = time.time() - first_message_time if first_message_time else 0
        print(f"✅ SUCCESS! Received {messages_received} messages")
        print(f"   First message arrived at T+{elapsed:.2f}s")
        print(f"   WebSocket is WORKING!")
        sys.exit(0)
    else:
        print(f"❌ FAILED: Zero messages received")
        print(f"   Possible causes:")
        print(f"   1. Token ID is expired/invalid")
        print(f"   2. Cloudflare WAF blocking (check TLS fingerprint)")
        print(f"   3. Network/firewall issues")
        sys.exit(1)
