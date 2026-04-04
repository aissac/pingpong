//! Network Module - Async Networking Components
//!
//! Contains:
//! - `ws_engine`: WebSocket engine for Polymarket real-time data

pub mod ws_engine;

pub use ws_engine::{WsEngine, WsEvent, WsConfig, TradeSide, ConnectionState, spawn_ws_engine};