/// Send Telegram alert on bot start
async fn send_start_alert(pairs_count: usize) {
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .unwrap_or_else(|_| "8754623467:AAHTUYGscxz1eDp2tH3olxuTXGdJHeOf0oY".to_string());
    let chat_id = std::env::var("TELEGRAM_CHAT_ID")
        .unwrap_or_else(|_| "1798631768".to_string());
    let pid = std::process::id();
    
    let msg = format!(
        "🚀 *HFT Bot STARTED*\n\nPID: `{}`\nPairs Tracked: `{}`\nMode: *LIVE ARMED*\n\nWatchdog: ✅ Active (3-min detection)",
        pid, pairs_count
    );
    let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
    
    // Fire-and-forget async task
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        let _ = client.post(&url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": msg,
                "parse_mode": "Markdown"
            }))
            .send()
            .await;
    });
}
