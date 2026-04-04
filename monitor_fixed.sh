#!/bin/bash

# HFT Bot 24-Hour Monitoring Script
# Checks every 5 minutes, alerts on issues

TELEGRAM_BOT_TOKEN="8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY"
TELEGRAM_CHAT_ID="1798631768"
JSONL_FILE="/mnt/ramdisk/nohup_v2.jsonl"
BOT_OUTPUT="/mnt/ramdisk/nohup_v2.out"
LOG_FILE="/home/ubuntu/logs/hft_monitor.log"
HOURLY_LOG="/home/ubuntu/logs/hft_hourly.log"
MAX_STALE_SECONDS=180

send_alert() {
    local message="$1"
    curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
        -d chat_id="${TELEGRAM_CHAT_ID}" \
        -d text="HFT MONITOR ALERT: ${message}" > /dev/null
    echo "[$(date -u)] ALERT: $message" >> "$LOG_FILE"
}

log_hourly() {
    local message="$1"
    echo "[$(date -u)] $message" >> "$HOURLY_LOG"
}

# Initialize
mkdir -p /home/ubuntu/logs
touch "$LOG_FILE" "$HOURLY_LOG"

echo "[$(date -u)] Monitor started" >> "$LOG_FILE"

# Track previous counts
PREV_JSONL=0
PREV_RECONNECTS=0
PREV_TIMEOUTS=0
HOUR_COUNT=0

while true; do
    # Check if bot is running
    BOT_PID=$(pgrep -f hft_pingpong_v2 | head -1)
    if [ -z "$BOT_PID" ]; then
        send_alert "Bot process NOT RUNNING! Manual intervention required."
        sleep 300
        continue
    fi
    
    # Check JSONL freshness
    if [ -f "$JSONL_FILE" ]; then
        LAST_MOD=$(stat -c %Y "$JSONL_FILE")
        NOW=$(date +%s)
        DIFF=$((NOW - LAST_MOD))
        CURRENT_JSONL=$(wc -l < "$JSONL_FILE")
    else
        DIFF=9999
        CURRENT_JSONL=0
    fi
    
    # Check reconnects (handle empty grep output)
    RECONNECT_OUTPUT=$(grep -c 'Reconnect attempt' "$BOT_OUTPUT" 2>/dev/null || true)
    if [ -z "$RECONNECT_OUTPUT" ]; then
        CURRENT_RECONNECTS=0
    else
        CURRENT_RECONNECTS=$RECONNECT_OUTPUT
    fi
    
    # Check TCP timeouts (handle empty grep output)
    TIMEOUT_OUTPUT=$(grep -c 'TCP TIMEOUT' "$BOT_OUTPUT" 2>/dev/null || true)
    if [ -z "$TIMEOUT_OUTPUT" ]; then
        CURRENT_TIMEOUTS=0
    else
        CURRENT_TIMEOUTS=$TIMEOUT_OUTPUT
    fi
    
    # Calculate deltas
    JSONL_DELTA=$((CURRENT_JSONL - PREV_JSONL))
    RECONNECT_DELTA=$((CURRENT_RECONNECTS - PREV_RECONNECTS))
    TIMEOUT_DELTA=$((CURRENT_TIMEOUTS - PREV_TIMEOUTS))
    
    # Alert on JSONL staleness
    if [ "$DIFF" -gt "$MAX_STALE_SECONDS" ]; then
        send_alert "JSONL stale for ${DIFF} seconds. Logger may be dead."
    fi
    
    # Alert on new reconnects
    if [ "$RECONNECT_DELTA" -gt 0 ] && [ "$PREV_RECONNECTS" -gt 0 ]; then
        send_alert "Bot reconnected ${RECONNECT_DELTA} times. Check connection stability."
    fi
    
    # Alert on TCP timeouts
    if [ "$TIMEOUT_DELTA" -gt 0 ]; then
        send_alert "TCP timeout detected ${TIMEOUT_DELTA} times. Ping/pong may have failed."
    fi
    
    # Log status
    echo "[$(date -u)] JSONL: ${CURRENT_JSONL} (+${JSONL_DELTA}) | Reconnects: ${CURRENT_RECONNECTS} (+${RECONNECT_DELTA}) | Timeouts: ${CURRENT_TIMEOUTS} (+${TIMEOUT_DELTA}) | Age: ${DIFF}s" >> "$LOG_FILE"
    
    # Hourly summary
    HOUR_COUNT=$((HOUR_COUNT + 1))
    if [ "$HOUR_COUNT" -ge 12 ]; then
        log_hourly "=== HOURLY SUMMARY ==="
        log_hourly "JSONL: ${PREV_JSONL} to ${CURRENT_JSONL} (+$((CURRENT_JSONL - PREV_JSONL)) lines)"
        log_hourly "Reconnects this hour: $((CURRENT_RECONNECTS - PREV_RECONNECTS))"
        log_hourly "Timeouts this hour: $((CURRENT_TIMEOUTS - PREV_TIMEOUTS))"
        log_hourly "Bot PID: $BOT_PID"
        log_hourly ""
        HOUR_COUNT=0
    fi
    
    # Update previous values
    PREV_JSONL=$CURRENT_JSONL
    PREV_RECONNECTS=$CURRENT_RECONNECTS
    PREV_TIMEOUTS=$CURRENT_TIMEOUTS
    
    sleep 300
done
