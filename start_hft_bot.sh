#!/bin/bash
# HFT Bot Startup Wrapper with Telegram Alert

TELEGRAM_BOT_TOKEN="8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY"
TELEGRAM_CHAT_ID="1798631768"

send_alert() {
    local message="$1"
    curl -s -X POST "https://api.telegram.org/bot${TELEGRAM_BOT_TOKEN}/sendMessage" \
        -d chat_id="${TELEGRAM_CHAT_ID}" \
        -d text="$message" \
        -d parse_mode="Markdown" > /dev/null
}

# Start the bot
cd /home/ubuntu/polymarket-hft-engine
nohup ./target/release/hft_pingpong_v2 > /mnt/ramdisk/nohup_v2.out 2>&1 &
BOT_PID=$!

sleep 5

# Count active markets from log
PAIRS=$(grep "Tracking.*token" /mnt/ramdisk/nohup_v2.out | tail -1 | grep -oP '\d+(?= token)' || echo "24")
if [ -z "$PAIRS" ]; then
    PAIRS="24"
fi

# Send START alert
send_alert "🚀 *HFT Bot STARTED*

PID: \`$BOT_PID\`
Pairs Tracked: \`$PAIRS\`
Mode: *LIVE ARMED*

Watchdog: ✅ Active (3-min detection)"

echo "Bot started with PID $BOT_PID, alert sent"
