//! SDK WebSocket Integration for HFT Bot
//! 
//! Replaces raw tungstenite with official polymarket-client-sdk
//! Handles Cloudflare WAF bypass automatically
//! 
//! Architecture:
//! - Async tokio thread manages SDK WebSocket
//! - Sync crossbeam channel feeds hot path
//! - Zero-allocation hot path preserved

use crossbeam_channel::{Sender as SyncSender, TrySendError};
use tokio::sync::watch;
use tokio::time::Duration;
use futures_util::StreamExt;
use rust_decimal::prelude::ToPrimitive;

/// Internal orderbook snapshot for hot path
/// Pure f64, no allocations, no Decimals
#[derive(Debug, Clone)]
pub struct OrderbookSnapshot {
    pub asset_id: String,
    pub best_bid_price: f64,
    pub best_bid_size: f64,
    pub best_ask_price: f64,
    pub best_ask_size: f64,
    pub timestamp: u64,
}

/// Convert SDK BookUpdate to internal snapshot
/// Does heavy Decimal parsing in background thread
pub fn convert_to_internal(update: &polymarket_client_sdk::clob::ws::types::response::BookUpdate) -> Option<OrderbookSnapshot> {
    // Need both bids and asks for arbitrage
    let best_bid = update.bids.first()?;
    let best_ask = update.asks.first()?;

    Some(OrderbookSnapshot {
        asset_id: update.asset_id.clone(),
        best_bid_price: best_bid.price.to_f64().unwrap_or(0.0),
        best_bid_size:  best_bid.size.to_f64().unwrap_or(0.0),
        best_ask_price: best_ask.price.to_f64().unwrap_or(0.0),
        best_ask_size:  best_ask.size.to_f64().unwrap_or(0.0),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
    })
}

/// Async SDK WebSocket manager
/// Runs in background tokio thread
pub async fn run_sdk_websocket(
    mut active_tokens_rx: watch::Receiver<Vec<String>>,
    hot_path_tx: SyncSender<OrderbookSnapshot>,
) {
    // Initialize official SDK client (handles Cloudflare evasion)
    let client = polymarket_client_sdk::clob::ws::Client::default();

    loop {
        let current_tokens = active_tokens_rx.borrow().clone();
        
        if current_tokens.is_empty() {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        eprintln!("🔗 [SDK] Connecting to {} tokens...", current_tokens.len());

        match client.subscribe_orderbook(current_tokens) {
            Ok(stream) => {
                eprintln!("✅ [SDK] Stream connected!");

                let mut pinned_stream = Box::pin(stream);

                loop {
                    tokio::select! {
                        // Rollover found new markets
                        Ok(_) = active_tokens_rx.changed() => {
                            eprintln!("🔄 [SDK] Rollover update, rebuilding stream...");
                            break;
                        }

                        // Orderbook update from Polymarket
                        msg = pinned_stream.next() => {
                            match msg {
                                Some(Ok(book)) => {
                                    if let Some(snapshot) = convert_to_internal(&book) {
                                        if let Err(TrySendError::Disconnected(_)) = hot_path_tx.try_send(snapshot) {
                                            eprintln!("🚨 [FATAL] Hot path disconnected");
                                            return;
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    eprintln!("❌ [SDK] Error: {:?}. Reconnecting...", e);
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    break;
                                }
                                None => {
                                    eprintln!("⚠️ [SDK] Stream ended. Reconnecting...");
                                    tokio::time::sleep(Duration::from_secs(2)).await;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ [SDK] Subscribe failed: {:?}. Retrying...", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}
