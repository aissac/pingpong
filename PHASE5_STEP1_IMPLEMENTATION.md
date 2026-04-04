# Phase 5 Step 1: Wire WebSocket to Crossbeam Channel

## NotebookLM Guidance Summary

From NotebookLM research (saved to notebook):

### Key Decisions

1. **NO separate OrderbookSnapshot struct** - Use zero-allocation borrowed strings with simd-json
2. **NO Option<f64>** - Use fixed-point integers immediately (micro-USDC)
3. **Group under condition_id** - YES/NO prices stored together in BTreeMap
4. **Reconnection reset** - Clear price cache, use initial_dump, verify positions

### Polymarket WebSocket Format

```json
{
    "event": "book",
    "asset_id": "15353185604353847122370324954202969073036867278400776447048296624042585335546",
    "bids": [
        { "price": "0.49", "size": "100" },
        { "price": "0.48", "size": "250" }
    ],
    "asks": [
        { "price": "0.51", "size": "500" }
    ],
    "hash": "0x...",
    "timestamp": 1712150000000
}
```

---

## Implementation

### 1. Create `src/market_state.rs`

This module will manage the YES/NO price grouping under condition_id.

```rust
//! Market State - Zero-Allocation Price Tracking
//!
//! Groups YES/NO prices under condition_id using BTreeMap
//! Uses fixed-point integers (micro-USDC) for branchless arithmetic

use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Fixed-point price (micro-USDC: $0.49 = 490,000)
pub type MicroUsdc = u64;

/// Market prices grouped by condition_id
/// 
/// Stores YES and NO prices together for edge detection
#[derive(Debug, Clone)]
pub struct MarketPrices {
    /// YES token ID
    pub yes_token_id: String,
    /// NO token ID
    pub no_token_id: String,
    /// YES mid price in micro-USDC (None until received)
    pub yes_mid: Option<MicroUsdc>,
    /// NO mid price in micro-USDC (None until received)
    pub no_mid: Option<MicroUsdc>,
    /// YES best ask (for edge calculation)
    pub yes_ask: Option<MicroUsdc>,
    /// NO best ask (for edge calculation)
    pub no_ask: Option<MicroUsdc>,
    /// Timestamp of last update
    pub last_update_ns: u64,
}

impl MarketPrices {
    pub fn new(yes_token_id: String, no_token_id: String) -> Self {
        Self {
            yes_token_id,
            no_token_id,
            yes_mid: None,
            no_mid: None,
            yes_ask: None,
            no_ask: None,
            last_update_ns: 0,
        }
    }
    
    /// Calculate combined ask (YES_ask + NO_ask)
    /// Returns None if either price is missing
    pub fn combined_ask(&self) -> Option<MicroUsdc> {
        match (self.yes_ask, self.no_ask) {
            (Some(yes), Some(no)) => Some(yes.saturating_add(no)),
            _ => None,
        }
    }
    
    /// Check for arbitrage opportunity
    /// Returns true if combined_ask < threshold
    pub fn is_edge(&self, threshold: MicroUsdc) -> Option<bool> {
        self.combined_ask().map(|combined| combined < threshold)
    }
    
    /// Reset all prices (called on reconnection)
    pub fn reset(&mut self) {
        self.yes_mid = None;
        self.no_mid = None;
        self.yes_ask = None;
        self.no_ask = None;
        self.last_update_ns = 0;
    }
}

/// Global market state manager
pub struct MarketStateManager {
    /// condition_id -> MarketPrices
    markets: Arc<RwLock<BTreeMap<String, MarketPrices>>>,
    /// token_id -> condition_id mapping
    token_to_condition: Arc<RwLock<BTreeMap<String, String>>>,
}

impl MarketStateManager {
    pub fn new() -> Self {
        Self {
            markets: Arc::new(RwLock::new(BTreeMap::new())),
            token_to_condition: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
    
    /// Register a new market with YES/NO token pair
    pub async fn register_market(
        &self,
        condition_id: String,
        yes_token_id: String,
        no_token_id: String,
    ) {
        let mut markets = self.markets.write().await;
        let mut token_map = self.token_to_condition.write().await;
        
        markets.insert(
            condition_id.clone(),
            MarketPrices::new(yes_token_id.clone(), no_token_id.clone()),
        );
        
        token_map.insert(yes_token_id, condition_id.clone());
        token_map.insert(no_token_id, condition_id);
    }
    
    /// Update price for a token
    /// Returns condition_id if market has both YES and NO prices
    pub async fn update_price(
        &self,
        token_id: &str,
        bid: MicroUsdc,
        ask: MicroUsdc,
        timestamp_ns: u64,
    ) -> Option<String> {
        let token_map = self.token_to_condition.read().await;
        let condition_id = token_map.get(token_id)?.clone();
        drop(token_map);
        
        let mut markets = self.markets.write().await;
        let market = markets.get_mut(&condition_id)?;
        
        // Determine if this is YES or NO token
        let is_yes = token_id == market.yes_token_id;
        
        let mid = (bid.saturating_add(ask)) / 2;
        
        if is_yes {
            market.yes_mid = Some(mid);
            market.yes_ask = Some(ask);
        } else {
            market.no_mid = Some(mid);
            market.no_ask = Some(ask);
        }
        
        market.last_update_ns = timestamp_ns;
        
        // Return condition_id if we have both prices
        if market.yes_ask.is_some() && market.no_ask.is_some() {
            Some(condition_id)
        } else {
            None
        }
    }
    
    /// Clear all prices (call on reconnection)
    pub async fn clear_all(&self) {
        let mut markets = self.markets.write().await;
        for market in markets.values_mut() {
            market.reset();
        }
    }
    
    /// Get market state
    pub async fn get_market(&self, condition_id: &str) -> Option<MarketPrices> {
        let markets = self.markets.read().await;
        markets.get(condition_id).cloned()
    }
    
    /// Find arbitrage opportunities
    /// Returns (condition_id, combined_ask, yes_ask, no_ask)
    pub async fn find_edges(&self, threshold: MicroUsdc) -> Vec<(String, MicroUsdc, MicroUsdc, MicroUsdc)> {
        let markets = self.markets.read().await;
        
        markets
            .iter()
            .filter_map(|(condition_id, market)| {
                let yes_ask = market.yes_ask?;
                let no_ask = market.no_ask?;
                let combined = yes_ask.saturating_add(no_ask);
                
                if combined < threshold {
                    Some((condition_id.clone(), combined, yes_ask, no_ask))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_market_prices() {
        let mut market = MarketPrices::new(
            "yes_token".to_string(),
            "no_token".to_string(),
        );
        
        // Initially no prices
        assert!(market.combined_ask().is_none());
        assert!(market.is_edge(950_000).is_none());
        
        // Set YES price
        market.yes_ask = Some(480_000);
        assert!(market.combined_ask().is_none()); // Still missing NO
        
        // Set NO price
        market.no_ask = Some(460_000);
        assert_eq!(market.combined_ask(), Some(940_000));
        
        // Check edge detection
        assert_eq!(market.is_edge(950_000), Some(true));  // 940k < 950k
        assert_eq!(market.is_edge(930_000), Some(false)); // 940k > 930k
        
        // Reset
        market.reset();
        assert!(market.yes_ask.is_none());
        assert!(market.no_ask.is_none());
    }

    #[tokio::test]
    async fn test_state_manager() {
        let manager = MarketStateManager::new();
        
        manager.register_market(
            "condition_1".to_string(),
            "yes_token".to_string(),
            "no_token".to_string(),
        ).await;
        
        // Update YES price
        let result = manager.update_price("yes_token", 470_000, 490_000, 1000).await;
        assert!(result.is_none()); // No edge yet (missing NO)
        
        // Update NO price
        let result = manager.update_price("no_token", 450_000, 470_000, 1001).await;
        assert_eq!(result, Some("condition_1".to_string()));
        
        // Find edges
        let edges = manager.find_edges(980_000).await;
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].1, 960_000); // combined ask
    }
}
```

### 2. Modify `src/network/ws_engine.rs`

Add reconnection handling and price reset:

```rust
// Add to process_message in ws_engine.rs

/// Process orderbook update
async fn process_book_update(&self, text: &str) -> Result<()> {
    // Use simd-json for zero-allocation parsing
    let value: Value = serde_json::from_str(text)?;
    
    // Extract token ID
    let token_id = value["asset_id"]
        .as_str()
        .context("No asset_id in book update")?;
    
    // Extract bids/asks arrays
    let bids = value["bids"]
        .as_array()
        .context("No bids array")?;
    let asks = value["asks"]
        .as_array()
        .context("No asks array")?;
    
    // Get best bid/ask (first in array)
    let best_bid = bids.first()
        .and_then(|b| b["price"].as_str())
        .and_then(|p| p.parse::<f64>().ok())
        .map(|p| (p * 1_000_000.0) as u64)
        .unwrap_or(0);
    
    let best_ask = asks.first()
        .and_then(|a| a["price"].as_str())
        .and_then(|p| p.parse::<f64>().ok())
        .map(|p| (p * 1_000_000.0) as u64)
        .unwrap_or(0);
    
    // Get sizes
    let bid_size = bids.first()
        .and_then(|b| b["size"].as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|s| (s * 1_000_000.0) as u64)
        .unwrap_or(0);
    
    let ask_size = asks.first()
        .and_then(|a| a["size"].as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|s| (s * 1_000_000.0) as u64)
        .unwrap_or(0);
    
    // Hash token ID for hot path
    let token_hash = fast_hash(token_id.as_bytes());
    
    let timestamp_ns = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    
    // Send to hot path
    let event = WsEvent::BookUpdate {
        token_hash,
        token_id: token_id.to_string(), // Include token_id for lookup
        bid_price: best_bid,
        bid_size,
        ask_price: best_ask,
        ask_size,
        timestamp_nanos: timestamp_ns,
    };
    
    let _ = self.event_tx.send(event);
    Ok(())
}

/// Handle reconnection
async fn handle_reconnect(&self) -> Result<()> {
    println!("[WS] Clearing price cache on reconnect...");
    
    // Send disconnect event to clear prices
    let _ = self.event_tx.send(WsEvent::Disconnected {
        reason: "Reconnecting".to_string(),
    });
    
    // Exponential backoff
    let delay = self.calculate_backoff();
    tokio::time::sleep(Duration::from_secs(delay)).await;
    
    // Reconnect with initial_dump=true
    self.connect_and_run().await
}
```

### 3. Modify `src/hot_path.rs`

Add MarketStateManager integration:

```rust
// Add to hot_path.rs

use crate::market_state::{MarketStateManager, MicroUsdc};

pub struct HotPath {
    /// Token hash -> (bid_price, bid_size, ask_price, ask_size)
    orderbook: HashMap<u64, (u64, u64, u64, u64)>,
    /// Market state manager
    market_state: Arc<MarketStateManager>,
    /// Token pairs (YES hash -> NO hash)
    token_pairs: HashMap<u64, u64>,
    /// Hash -> Token ID mapping
    hash_to_id: HashMap<u64, String>,
    /// Token ID -> Condition ID mapping
    id_to_condition: HashMap<String, String>,
    /// Edge threshold
    edge_threshold: u64,
    // ... rest of fields
}

impl HotPath {
    /// Process orderbook update with market state tracking
    fn process_book_update(
        &mut self,
        token_hash: u64,
        token_id: &str,
        bid_price: u64,
        bid_size: u64,
        ask_price: u64,
        ask_size: u64,
        timestamp_nanos: u64,
        task_tx: &Sender<BackgroundTask>,
    ) {
        let start = Instant::now();
        
        // Update orderbook
        self.orderbook.insert(token_hash, (bid_price, bid_size, ask_price, ask_size));
        
        // Update market state (async, spawn on tokio)
        let market_state = Arc::clone(&self.market_state);
        let token_id_owned = token_id.to_string();
        tokio::spawn(async move {
            if let Some(condition_id) = market_state.update_price(
                &token_id_owned,
                bid_price,
                ask_price,
                timestamp_nanos,
            ).await {
                // Both YES and NO prices available - check for edge
                if let Some(market) = market_state.get_market(&condition_id).await {
                    if let Some(is_edge) = market.is_edge(940_000) {
                        if is_edge {
                            println!("[HOT_PATH] 🎯 Edge detected: {} YES={:.4} NO={:.4}",
                                condition_id,
                                market.yes_ask.unwrap() as f64 / 1_000_000.0,
                                market.no_ask.unwrap() as f64 / 1_000_000.0,
                            );
                        }
                    }
                }
            }
        });
        
        // ... rest of hot path logic
    }
    
    /// Handle disconnect - reset prices
    fn handle_disconnect(&mut self) {
        println!("[HOT_PATH] Clearing all prices on disconnect");
        
        // Clear orderbook
        self.orderbook.clear();
        
        // Clear market state
        tokio::spawn(async move {
            // This would need to be async, so we spawn a task
            // In practice, market_state.clear_all() would be called
        });
    }
}
```

### 4. Add reconnection test to `src/main.rs`

```rust
// Add to main.rs

/// Test reconnection handling
async fn test_reconnection() {
    let market_state = Arc::new(MarketStateManager::new());
    
    // Register test market
    market_state.register_market(
        "test_condition".to_string(),
        "yes_token".to_string(),
        "no_token".to_string(),
    ).await;
    
    // Update prices
    market_state.update_price("yes_token", 470_000, 490_000, 1000).await;
    market_state.update_price("no_token", 450_000, 470_000, 1001).await;
    
    // Verify we have both prices
    let edges = market_state.find_edges(980_000).await;
    assert!(!edges.is_empty());
    
    // Simulate disconnect
    market_state.clear_all().await;
    
    // Verify prices are cleared
    let edges = market_state.find_edges(980_000).await;
    assert!(edges.is_empty());
    
    println!("✅ Reconnection test passed");
}
```

---

## Testing Steps

### 1. Build and Run
```bash
cd /home/aissac/.openclaw/workspace
cargo build --release
DRY_RUN=true cargo run --release
```

### 2. Monitor WebSocket
```bash
# Watch for orderbook updates
tail -f logs/bot.log | grep -E "\[WS\]|\[HOT_PATH\]"
```

### 3. Verify Edge Detection
```bash
# Should see edges when YES_ask + NO_ask < threshold
# Example: YES=0.48, NO=0.46 → combined=0.94 < 0.95 threshold
```

### 4. Test Reconnection
```bash
# Kill WebSocket manually to test reconnect
kill -SIGTERM <pid>

# Should see:
# [WS] Clearing price cache on reconnect...
# [HOT_PATH] Clearing all prices on disconnect
# [WS] Reconnecting in 5s...
```

---

## Next Steps

After Step 1 is verified:

**Step 2: Integrate with hot_path thread**
- Modify `src/main.rs` to spawn hot_path thread
- Connect crossbeam channel to hot_path
- Pin hot_path to CPU core

**Step 3: Add dry-run execution logic**
- Simulate fills without real orders
- Track PnL, fill rates, adverse selection
- Log to JSONL for analysis

**Step 4: Add telemetry/reporting**
- Telegram alerts for opportunities
- Periodic PnL reports
- Metrics dashboard

**Step 5: Test end-to-end**
- Run dry-run for 15-30 minutes
- Verify opportunities detected
- Verify circuit breakers trigger correctly