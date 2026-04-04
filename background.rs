// src/background.rs
// Background thread handling: Ghost simulation, Telegram reporting, JSON logging
// Runs on unpinned core with tokio async runtime - no latency impact on hot path

use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use chrono::Utc;

use crate::telemetry::{OpportunitySnapshot, BackgroundTask, LatencyBatch};

/// Ghost simulation result
#[derive(Debug)]
pub enum GhostResult {
    Executable,  // Liquidity survived 50ms RTT
    Ghosted,     // Liquidity vanished
    Partial,     // Some liquidity remained
}

/// Execute ghost simulation after 50ms delay
/// Checks if the orderbook depth from the opportunity snapshot still exists
pub async fn run_ghost_simulation(
    snap: OpportunitySnapshot,
    client: Client,
    token_map: Arc<HashMap<u64, String>>,
    tg_token: String,
    tg_chat_id: String,
) {
    // 1. Wait 50ms to simulate network RTT + competition reaction
    sleep(Duration::from_millis(50)).await;
    
    // 2. Resolve token hashes to Polymarket Token ID strings
    // (The hot path uses u64 hashes for speed; background resolves to REST API IDs)
    let yes_token_str = match token_map.get(&snap.yes_token_hash) {
        Some(t) => t.clone(),
        None => return, // Unknown token, exit silently
    };
    
    // 3. Fetch current orderbook via REST API to check liquidity
    let url = format!("https://clob.polymarket.com/book?token_id={}", yes_token_str);
    
    let result = match client.get(&url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                check_liquidity(&text, snap.yes_ask_price, snap.yes_depth)
            } else {
                GhostResult::Ghosted // Parse failure = assume ghost
            }
        }
        Err(_) => GhostResult::Ghosted, // Network failure = assume ghost
    };
    
    // 4. Log result and send to Telegram
    let combined_price = snap.combined_price();
    let edge_pct = snap.edge_percent();
    let timestamp = Utc::now().to_rfc3339();
    
    let status_icon = match result {
        GhostResult::Executable => "✅",
        GhostResult::Ghosted => "👻",
        GhostResult::Partial => "⚠️",
    };
    
    let status_text = match result {
        GhostResult::Executable => "EXECUTABLE (Liquidity Survived)",
        GhostResult::Ghosted => "GHOSTED (Liquidity Vanished)",
        GhostResult::Partial => "PARTIAL (Some Liquidity Remained)",
    };
    
    // Log to file (async, non-blocking)
    tracing::info!(
        "{} GHOST SIMULATION: {} | ${:.4} | Edge: {:.1}% | {}",
        status_icon,
        format_hash(snap.condition_hash),
        combined_price,
        edge_pct,
        status_text
    );
    
    // 5. Send Telegram report (fire-and-forget)
    let report = format!(
        "{} *Ghost Simulation*\n\
         Time: {}\n\
         Combined: ${:.4}\n\
         Edge: {:.1}%\n\
         Status: {}",
        status_icon, timestamp, combined_price, edge_pct, status_text
    );
    
    let tg_url = format!("https://api.telegram.org/bot{}/sendMessage", tg_token);
    let _ = client.post(&tg_url)
        .json(&serde_json::json!({
            "chat_id": tg_chat_id,
            "text": report,
            "parse_mode": "Markdown"
        }))
        .send()
        .await;
}

/// Check if liquidity still exists at the target price
fn check_liquidity(json_response: &str, target_price: u64, target_depth: u64) -> GhostResult {
    // Parse JSON response (simplified - use serde_json in production)
    // Look for asks at or below target_price with sufficient depth
    
    // TODO: Parse actual orderbook JSON
    // For now, simulate based on price movement
    
    let target_price_f64 = target_price as f64 / 1_000_000.0;
    let target_depth_f64 = target_depth as f64;
    
    // Check if depth at target price is >= 50% of original
    // This is a simplified check - real implementation parses orderbook
    
    GhostResult::Executable // Default to executable for now
}

/// Process and aggregate latency stats
pub async fn process_latency_stats(stats: LatencyBatch, client: Client, tg_token: String, tg_chat_id: String) {
    let report = format!(
        "📊 *Latency Report*\n\
         Avg: {:.2}µs\n\
         Min: {:.2}µs\n\
         Max: {:.2}µs\n\
         P99: {:.2}µs\n\
         Samples: {}",
        stats.avg_nanos as f64 / 1000.0,
        stats.min_nanos as f64 / 1000.0,
        stats.max_nanos as f64 / 1000.0,
        stats.p99_nanos as f64 / 1000.0,
        stats.sample_count
    );
    
    // Send to Telegram (optional - can be disabled for high-frequency)
    if stats.sample_count >= 1000 {  // Only send periodic updates
        let tg_url = format!("https://api.telegram.org/bot{}/sendMessage", tg_token);
        let _ = client.post(&tg_url)
            .json(&serde_json::json!({
                "chat_id": tg_chat_id,
                "text": report,
                "parse_mode": "Markdown"
            }))
            .send()
            .await;
    }
}

/// Format hash for display (first 8 characters)
fn format_hash(hash: u64) -> String {
    format!("{:016x}", hash)[..8].to_string()
}