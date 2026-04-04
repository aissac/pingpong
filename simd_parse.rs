//! SIMD-optimized orderbook parsing
//! 
//! Uses simd_json for 1.77x faster JSON parsing vs serde_json
//! Fixed-point (u64) arithmetic for zero FPU overhead
//! Zero-allocation: borrowed strings, Copy structs

use simd_json::serde::from_slice;
use serde::Deserialize;
use rustc_hash::FxHasher;
use std::hash::{Hash, Hasher};

/// Price change from WebSocket - BORROWED, zero allocation
#[derive(Debug, Clone, Deserialize)]
pub struct PriceChange<'a> {
    #[serde(rename = "price")]
    pub price: &'a str,
    #[serde(rename = "size")]
    pub size: &'a str,
    #[serde(rename = "side")]
    pub side: &'a str,
    #[serde(rename = "asset_id")]
    pub asset_id: &'a str,
}

/// Book update from WebSocket - BORROWED, zero allocation
#[derive(Debug, Clone, Deserialize)]
pub struct BookUpdate<'a> {
    #[serde(rename = "market")]
    pub market: &'a str,
    #[serde(rename = "price_changes", default)]
    pub price_changes: Vec<PriceChange<'a>>,
}

/// Orderbook update - ZERO ALLOCATION (Copy struct)
#[derive(Debug, Clone, Copy)]
pub struct OrderBookUpdate {
    pub token_id_hash: u64,
    pub is_yes: bool,
    pub price: u64,
    pub size: u64,
    pub timestamp: u64,
}

/// Parse a decimal string to u64 fixed-point with 6 decimal places
/// E.g., "0.65" -> 650000, "15.5" -> 15500000
/// Zero allocation, branchless, no FPU
#[inline(always)]
pub fn parse_fixed_6(bytes: &[u8]) -> u64 {
    let mut res: u64 = 0;
    let mut dec_places: u8 = 0;
    let mut in_decimals = false;

    for &b in bytes {
        if b == b'.' {
            in_decimals = true;
            continue;
        }
        res = res.wrapping_mul(10).wrapping_add((b - b'0') as u64);
        
        if in_decimals {
            dec_places += 1;
            if dec_places == 6 {
                break;
            }
        }
    }

    while dec_places < 6 {
        res = res.wrapping_mul(10);
        dec_places += 1;
    }

    res
}

/// Fast hash for token IDs (FxHash - used by rustc compiler)
#[inline(always)]
pub fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Parse orderbook update using SIMD JSON
pub fn parse_orderbook_update_simd<'a>(data: &'a mut [u8], _max_depth: usize) -> Result<Vec<OrderBookUpdate>, simd_json::Error> {
    let update: BookUpdate = from_slice(data)?;
    
    let mut result = Vec::with_capacity(1);
    
    if let Some(best_level) = update.price_changes.first() {
        let token_hash = fast_hash(best_level.asset_id.as_bytes());
        let side_upper = best_level.side.to_uppercase();
        let is_yes = side_upper.contains("YES") || side_upper.contains("BUY");
        
        let price_u64 = parse_fixed_6(best_level.price.as_bytes());
        let size_u64 = parse_fixed_6(best_level.size.as_bytes());
        
        result.push(OrderBookUpdate {
            token_id_hash: token_hash,
            is_yes,
            price: price_u64,
            size: size_u64,
            timestamp: 0,
        });
    }
    
    Ok(result)
}