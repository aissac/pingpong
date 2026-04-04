#!/bin/bash

# HFT Bot Watchdog - Detects silent WebSocket death
# Checks JSONL staleness and auto-restarts bot

JSONL_FILE="/mnt/ramdisk/nohup_v2.jsonl"
TELEGRAM_BOT_TOKEN="8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY"
TELEGRAM_CHAT_ID="1798631768"
MAX_STALE_SECONDS=180
BOT_NAME="hft_pingpong_v2"

send_alert() {
    local message="$1"
    curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
        -d chat_id="${TELEGRAM_CHAT_ID}" \
        -d text="🚨 HFT CRITICAL: ${message}" > /dev/null
}

# Check if file exists
if [ ! -f "$JSONL_FILE" ]; then
    exit 0
fi

# Get file modification time
LAST_MOD=$(stat -c %Y "$JSONL_FILE")
NOW=$(date +%s)
DIFF=$((NOW - LAST_MOD))

# Check staleness
if [ "$DIFF" -gt "$MAX_STALE_SECONDS" ]; then
    send_alert "JSONL stale for ${DIFF}s. Silent death detected. Restarting bot..."
    
    # Kill bot process
    pkill -9 -f "$BOT_NAME"
    sleep 2
    
    # Restart bot
    cd /home/ubuntu/polymarket-hft-engine
    nohup ./target/release/hft_pingpong_v2 > /mnt/ramdisk/nohup_v2.out 2>&1 &
    
    sleep 5
    
    # Confirm recovery
    if pgrep -f "$BOT_NAME" > /dev/null; then
        send_alert "✅ Bot successfully restarted (PID: $(pgrep -f $BOT_NAME))."
    else
        send_alert "❌ Bot failed to restart! Manual intervention required."
    fi
fi
