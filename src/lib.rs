//! Market Maker - Phase 3 Integration Library
//!
//! Modules:
//! - `rate_limiter`: Token bucket rate limiting for CLOB API
//! - `api_client`: REST API client with EIP-712 signing
//! - `inventory_tracker`: Position tracking
//! - `quote_manager`: Skew-aware quote generation
//! - `hot_path`: Zero-allocation event processing
//! - `network::ws_engine`: Async WebSocket engine

pub mod rate_limiter;
pub mod api_client;
pub mod inventory_tracker;
pub mod quote_manager;
pub mod hot_path;
pub mod market_state;

pub mod network {
    pub mod ws_engine;
}

// Re-exports for convenience
pub use rate_limiter::{RateLimiter, global_rate_limiter};
pub use api_client::{ClobClient, Side, OrderStatus, OrderResult, Orderbook, PriceLevel};
pub use inventory_tracker::{InventoryTracker, Position, MarketPosition, Side as InventorySide};
pub use quote_manager::{QuoteManager, QuoteConfig, Quote, QuoteSide, MarketState as QuoteMarketState};
pub use market_state::{MarketState, MarketPrices, MarketStats, MicroUsdc};
pub use hot_path::{HotPath, BackgroundTask, spawn_hot_path};
pub use network::ws_engine::{WsEngine, WsEvent, WsConfig, spawn_ws_engine};