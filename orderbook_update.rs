//! OrderBookUpdate - Zero-allocation HFT struct
//! 
//! Uses u64 fixed-point and hashed IDs for sub-microsecond processing.
//! Implements Copy trait (no heap allocations).

/// Orderbook update from WebSocket - ZERO ALLOCATION
/// All fields are Copy types (u64, bool) - no Strings, no f64
#[derive(Debug, Clone, Copy)]
pub struct OrderBookUpdate {
    pub token_id_hash: u64,   // Hashed token_id for O(1) lookup
    pub is_yes: bool,         // true = YES, false = NO
    pub price: u64,           // Fixed-point (6 decimals): 650000 = $0.65
    pub size: u64,            // Fixed-point: 10000000 = 10 shares
    pub timestamp: u64,       // TSC nanoseconds
}
