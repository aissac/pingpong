//! SIMD JSON parsing for orderbook updates

use simd_json::{from_slice};
use std::borrow::Cow;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use rustc_hash::FxHasher;

/// Orderbook update - BORROWED from WebSocket message
#[derive(Debug, Clone, Deserialize)]
pub struct PriceChange<'a> {
    #[serde(rename = "asset_id")]
    pub asset_id: Cow<'a, str>,
    #[serde(rename = "price")]
    pub price: Cow<'a, str>,
    #[serde(rename = "size")]
    pub size: Cow<'a, str>,
    #[serde(rename = "side")]
    pub side: Cow<'a, str>,
}

/// Book update from WebSocket - BORROWED, zero allocation
#[derive(Debug, Clone, Deserialize)]
pub struct BookUpdate<'a> {
    #[serde(rename = "market")]
    pub market: Cow<'a, str>,
    #[serde(rename = "price_changes", default)]
    pub price_changes: Vec<PriceChange<'a>>,
}

/// Orderbook update - ZERO ALLOCATION (Copy struct)
#[derive(Debug, Clone, Copy)]
pub struct OrderBookUpdate {
    pub token_id_hash: u64,
    pub condition_hash: u64,
    pub is_yes: bool,
    pub price: u64,
    pub size: u64,
}

/// Fast hash for token IDs (FxHash - used by rustc compiler)
#[inline(always)]
pub fn fast_hash(bytes: &[u8]) -> u64 {
    let mut hasher = FxHasher::default();
    bytes.hash(&mut hasher);
    hasher.finish()
}

/// Parse a decimal string to u64 fixed-point with 6 decimal places
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
        if b >= b'0' && b <= b'9' {
            res = res.saturating_mul(10).saturating_add((b - b'0') as u64);
            if in_decimals {
                dec_places += 1;
                if dec_places >= 6 {
                    break;
                }
            }
        }
    }
    
    // Add trailing zeros for missing decimal places
    for _ in dec_places..6 {
        res = res.saturating_mul(10);
    }
    
    res
}

/// Parse orderbook update using SIMD JSON
/// CRITICAL: is_yes is determined by checking if asset_id is in the YES_TOKENS set
pub fn parse_orderbook_update_simd<'a>(
    data: &'a mut [u8], 
    _max_depth: usize,
    yes_tokens: &HashSet<String>,
) -> Result<Vec<OrderBookUpdate>, simd_json::Error> {
    let update: BookUpdate = from_slice(data)?;
    
    let mut result = Vec::with_capacity(1);
    
    if let Some(best_level) = update.price_changes.first() {
        let asset_id = best_level.asset_id.as_str();
        
        // CRITICAL: Use market field as condition_hash (same for YES and NO)
        let condition_hash = fast_hash(update.market.as_bytes());
        
        // CRITICAL: is_yes is determined by which token this is
        // YES tokens are the first in clobTokenIds array from API
        let is_yes = yes_tokens.contains(asset_id);
        
        let token_id_hash = fast_hash(asset_id.as_bytes());
        let price_u64 = parse_fixed_6(best_level.price.as_bytes());
        let size_u64 = parse_fixed_6(best_level.size.as_bytes());
        
        result.push(OrderBookUpdate {
            token_id_hash,
            condition_hash,
            is_yes,
            price: price_u64,
            size: size_u64,
        });
    }
    
    Ok(result)
}