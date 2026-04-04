//! Rate Limiter - Token Bucket for Polymarket CLOB API
//!
//! Implements token bucket rate limiting:
//! - Burst capacity: 300 requests
//! - Sustained rate: 50 requests/second
//! - Handles HTTP 429 with exponential backoff

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use std::collections::VecDeque;

/// Token bucket rate limiter for Polymarket CLOB API
/// 
/// Configuration:
/// - Max burst: 300 tokens
/// - Refill rate: 50 tokens/sec
/// - Min backoff on 429: 100ms, max: 10s
pub struct RateLimiter {
    /// Current available tokens
    tokens: Arc<Mutex<f64>>,
    /// Maximum burst capacity
    max_tokens: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill timestamp
    last_refill: Arc<Mutex<Instant>>,
    /// Queue of waiting requests (for fairness)
    wait_queue: Arc<Mutex<VecDeque<tokio::sync::oneshot::Sender<()>>>>,
    /// Current backoff duration (increases on 429, decreases on success)
    current_backoff_ms: Arc<Mutex<u64>>,
}

impl RateLimiter {
    /// Create a new rate limiter with Polymarket defaults
    /// - 300 burst capacity
    /// - 50 req/sec sustained
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(300.0)),
            max_tokens: 300.0,
            refill_rate: 50.0,
            last_refill: Arc::new(Mutex::new(Instant::now())),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
            current_backoff_ms: Arc::new(Mutex::new(100)),
        }
    }

    /// Create a rate limiter with custom settings
    pub fn with_config(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: Arc::new(Mutex::new(Instant::now())),
            wait_queue: Arc::new(Mutex::new(VecDeque::new())),
            current_backoff_ms: Arc::new(Mutex::new(100)),
        }
    }

    /// Wait for a token to become available
    /// 
    /// This is the primary method for API calls. It will:
    /// 1. Refill tokens based on elapsed time
    /// 2. Wait if no tokens available
    /// 3. Consume one token
    pub async fn wait_for_token(&self) {
        loop {
            // Refill tokens based on elapsed time
            self.refill().await;
            
            // Try to acquire a token
            let mut tokens = self.tokens.lock().await;
            if *tokens >= 1.0 {
                *tokens -= 1.0;
                return;
            }
            
            // No tokens available - calculate wait time
            drop(tokens); // Release lock before sleeping
            
            let wait_ms = self.calculate_wait_time().await;
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
        }
    }

    /// Try to acquire a token without waiting
    /// Returns true if token was acquired, false otherwise
    pub async fn try_acquire(&self) -> bool {
        self.refill().await;
        
        let mut tokens = self.tokens.lock().await;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Get current number of available tokens (for monitoring)
    pub async fn available_tokens(&self) -> f64 {
        self.refill().await;
        *self.tokens.lock().await
    }

    /// Check if we're currently rate-limited
    pub async fn is_limited(&self) -> bool {
        self.refill().await;
        *self.tokens.lock().await < 1.0
    }

    /// Handle HTTP 429 response - increase backoff and wait
    /// 
    /// Call this when API returns 429 Too Many Requests
    /// It will exponentially increase backoff
    pub async fn handle_429(&self) {
        let mut backoff = self.current_backoff_ms.lock().await;
        *backoff = (*backoff * 2).min(10_000); // Max 10s backoff
        
        println!("[RATE_LIMIT] ⚠️ 429 received, backing off for {}ms", *backoff);
        
        // Also reduce available tokens as a penalty
        let mut tokens = self.tokens.lock().await;
        *tokens = (*tokens * 0.5).max(0.0);
        
        tokio::time::sleep(Duration::from_millis(*backoff)).await;
    }

    /// Reset backoff after successful request
    pub async fn reset_backoff(&self) {
        let mut backoff = self.current_backoff_ms.lock().await;
        *backoff = (*backoff / 2).max(10); // Min 10ms backoff
    }

    /// Refill tokens based on elapsed time
    async fn refill(&self) {
        let mut last = self.last_refill.lock().await;
        let mut tokens = self.tokens.lock().await;
        
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs_f64();
        
        // Refill tokens based on elapsed time
        let new_tokens = elapsed * self.refill_rate;
        *tokens = (*tokens + new_tokens).min(self.max_tokens);
        
        *last = now;
    }

    /// Calculate wait time when no tokens available
    async fn calculate_wait_time(&self) -> u64 {
        let tokens = self.tokens.lock().await;
        let deficit = 1.0 - *tokens;
        let wait_secs = deficit / self.refill_rate;
        
        // Add small buffer to avoid busy-waiting
        let wait_ms = (wait_secs * 1000.0).ceil() as u64;
        wait_ms.max(10).min(1000) // 10ms - 1000ms
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Global rate limiter instance (singleton pattern)
static RATE_LIMITER: std::sync::OnceLock<Arc<RateLimiter>> = std::sync::OnceLock::new();

/// Get the global rate limiter instance
pub fn global_rate_limiter() -> Arc<RateLimiter> {
    RATE_LIMITER.get_or_init(|| Arc::new(RateLimiter::new())).clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new();
        
        // Should have tokens available initially
        assert!(limiter.try_acquire().await);
        assert!(limiter.available_tokens().await < 300.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let limiter = RateLimiter::new();
        
        // Consume all tokens
        for _ in 0..300 {
            assert!(limiter.try_acquire().await);
        }
        
        // Should be empty now
        assert!(!limiter.try_acquire().await);
        
        // Wait for refill (50 tokens/sec = 20ms per token)
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Should have ~5 tokens refilled
        assert!(limiter.available_tokens().await > 0.0);
    }

    #[tokio::test]
    async fn test_429_backoff() {
        let limiter = RateLimiter::new();
        
        // Initial backoff is 100ms
        limiter.handle_429().await;
        
        // After 429, backoff doubles
        let backoff = *limiter.current_backoff_ms.lock().await;
        assert_eq!(backoff, 200); // 100 * 2
        
        // Successful requests reduce backoff
        limiter.reset_backoff().await;
        let backoff = *limiter.current_backoff_ms.lock().await;
        assert_eq!(backoff, 100); // 200 / 2
    }
}