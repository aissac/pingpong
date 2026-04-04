//! Orderbook State - Fixed-Size Array for HFT
//!
//! Uses fixed-size arrays (100 elements) for bids/asks
//! Prices are in cents (0-99), allowing O(1) access to best bid/ask
//! Zero-allocation, cache-friendly structure

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// Token orderbook state using fixed-size arrays
/// Index = price in cents (0-99), Value = size
#[derive(Clone, Debug)]
pub struct TokenBookState {
    pub bids: [u64; 100],  // bids[44] = size at $0.44
    pub asks: [u64; 100],  // asks[46] = size at $0.46
}

impl Default for TokenBookState {
    fn default() -> Self {
        Self {
            bids: [0; 100],
            asks: [0; 100],
        }
    }
}

impl TokenBookState {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Get best bid (highest buy price with size > 0)
    pub fn get_best_bid(&self) -> Option<(u64, u64)> {
        // Iterate backwards from 99 to 1
        for price in (1..100).rev() {
            if self.bids[price] > 0 {
                return Some((price as u64, self.bids[price]));
            }
        }
        None
    }
    
    /// Get best ask (lowest sell price with size > 0)
    pub fn get_best_ask(&self) -> Option<(u64, u64)> {
        // Iterate forwards from 1 to 99
        for price in 1..100 {
            if self.asks[price] > 0 {
                return Some((price as u64, self.asks[price]));
            }
        }
        None
    }
    
    /// Update bid level (price in fixed-point * 1,000,000)
    pub fn update_bid(&mut self, price_u64: u64, size_u64: u64) {
        // Convert from fixed-point to cents
        let price_cents = (price_u64 / 10_000) as usize;  // e.g., 440000 -> 44
        if price_cents > 0 && price_cents < 100 {
            self.bids[price_cents] = size_u64;
        }
    }
    
    /// Update ask level (price in fixed-point * 1,000,000)
    pub fn update_ask(&mut self, price_u64: u64, size_u64: u64) {
        // Convert from fixed-point to cents
        let price_cents = (price_u64 / 10_000) as usize;  // e.g., 460000 -> 46
        if price_cents > 0 && price_cents < 100 {
            self.asks[price_cents] = size_u64;
        }
    }
    
    /// Debug print the orderbook
    pub fn debug_print(&self, token: &str) {
        if let Some((bid_price, bid_size)) = self.get_best_bid() {
            if let Some((ask_price, ask_size)) = self.get_best_ask() {
                println!("[BOOK] {} Bid: ${:.2} (size: {}) | Ask: ${:.2} (size: {})", 
                    token,
                    bid_price as f64 / 100.0,
                    bid_size,
                    ask_price as f64 / 100.0,
                    ask_size
                );
            }
        }
    }
}

/// Opportunity detected (sent to background thread)
#[derive(Clone, Debug)]
pub struct OpportunitySnapshot {
    pub yes_token_hash: u64,
    pub no_token_hash: u64,
    pub yes_best_bid: u64,   // Fixed-point * 1,000,000
    pub yes_best_ask: u64,   // Fixed-point * 1,000,000
    pub yes_ask_size: u64,   // Size at best ask
    pub no_best_bid: u64,
    pub no_best_ask: u64,
    pub no_ask_size: u64,
    pub combined_ask: u64,   // YES Ask + NO Ask (TRUE cost to arbitrage)
    pub timestamp_nanos: u64,
}

/// Fast hash for token IDs
pub fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Parse fixed-point price (multiply by 1,000,000 for 6 decimal precision)
pub fn parse_fixed_6(bytes: &[u8]) -> u64 {
    let mut result: u64 = 0;
    let mut decimal_seen = false;
    let mut decimal_places = 0u32;
    
    for &b in bytes {
        match b {
            b'.' => decimal_seen = true,
            b'0'..=b'9' => {
                result = result * 10 + (b - b'0') as u64;
                if decimal_seen { decimal_places += 1; }
            }
            _ => {}
        }
    }
    
    // Adjust to 6 decimal places
    while decimal_places < 6 {
        result *= 10;
        decimal_places += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_best_bid_ask() {
        let mut book = TokenBookState::new();
        
        // Add some bids
        book.update_bid(44_000_000, 1000);  // $0.44
        book.update_bid(45_000_000, 2000);  // $0.45
        book.update_bid(43_000_000, 500);   // $0.43
        
        // Add some asks
        book.update_ask(46_000_000, 1500);  // $0.46
        book.update_ask(47_000_000, 3000);  // $0.47
        book.update_ask(48_000_000, 2500);  // $0.48
        
        // Best bid should be $0.45 (highest)
        let (bid_price, bid_size) = book.get_best_bid().unwrap();
        assert_eq!(bid_price, 45);
        assert_eq!(bid_size, 2000);
        
        // Best ask should be $0.46 (lowest)
        let (ask_price, ask_size) = book.get_best_ask().unwrap();
        assert_eq!(ask_price, 46);
        assert_eq!(ask_size, 1500);
        
        // Combined ask should be $0.46 + $0.46 (for YES/NO pair)
        let combined_ask = ask_price * 2;  // Simplified
        println!("Combined ask: ${:.2}", combined_ask as f64 / 100.0);
    }
    
    #[test]
    fn test_zero_size_deletes_level() {
        let mut book = TokenBookState::new();
        
        // Add bid at $0.45
        book.update_bid(45_000_000, 1000);
        assert!(book.get_best_bid().is_some());
        
        // Delete by setting size to 0
        book.update_bid(45_000_000, 0);
        assert!(book.get_best_bid().is_none());
    }
}