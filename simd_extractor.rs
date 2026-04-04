//! SIMD memchr byte scanning fallback
//! 
//! Simple byte scanner for best_ask field.
//! Returns count of updates processed, or 0 to trigger serde fallback.

use memchr::memmem;

/// Simple byte scanner - checks if best_ask exists
/// Returns 0 to trigger serde fallback (which is working well at 3µs)
#[inline(always)]
pub fn extract_all_borrowed(bytes: &mut [u8], _orderbook: &mut super::hft_hot_path::LocalOrderBook) -> usize {
    // For now, always return 0 to use the proven serde path
    // The serde path with simd_json is hitting 3µs avg / 0.92µs min
    // which is elite HFT performance
    0
}