//! Integration Guide for Live Trading
//! 
//! This module shows how to integrate all components into hft_pingpong.rs

/*

## CARGO.TOML DEPENDENCIES

Add to your Cargo.toml:

```toml
[dependencies]
# Existing dependencies
tokio = { version = "1.0", features = ["full"] }
tokio-tungstenite = "0.21"
futures-util = "0.3"
serde_json = "1.0"
reqwest = { version = "0.11", features = ["json"] }

# New dependencies for live trading
alloy-primitives = "0.8"
alloy-sol-types = "0.8"
alloy-signer = "0.2"
alloy-signer-local = "0.2"
hex = "0.4"
base64 = "0.21"
hmac = "0.12"
sha2 = "0.10"
dashmap = "5.5"
```

## MODULE STRUCTURE

```
src/
├── lib.rs              # Add module declarations
├── signing.rs          # EIP-712 signing
├── execution.rs        # CLOB order submission
├── merge_worker.rs    # CTF merge (25 RPM)
├── stop_loss.rs        # 3-second stop-loss
├── state.rs           # Shared ExecutionState
├── user_ws.rs         # WebSocket monitoring
└── bin/
    └── hft_pingpong.rs # Main binary (integrated)
```

## LIB.RS MODULE DECLARATIONS

```rust
// src/lib.rs
pub mod signing;
pub mod execution;
pub mod merge_worker;
pub mod stop_loss;
pub mod state;
pub mod user_ws;
pub mod hft_hot_path;  // Your existing hot path

// Re-exports
pub use signing::{init_signer, sign_polymarket_order, create_order, Order};
pub use execution::{submit_order_with_backoff, build_l2_headers, fetch_fee_rate};
pub use merge_worker::{run_merge_worker, MergeTask};
pub use stop_loss::{start_stop_loss_timer, execute_fak_order};
pub use state::ExecutionState;
pub use user_ws::run_user_ws;
```

## BIN/HFT_PINGPONG.RS INTEGRATION

```rust
// src/bin/hft_pingpong.rs

use std::sync::Arc;
use tokio::sync::mpsc;
use reqwest::Client;

use pingpong::{
    signing::init_signer,
    state::ExecutionState,
    user_ws::run_user_ws,
    merge_worker::{run_merge_worker, MergeTask},
    hft_hot_path::{run_sync_hot_path, BackgroundTask},
};

// Environment variables
const POLYMARKET_PRIVATE_KEY: &str = "POLYMARKET_PRIVATE_KEY";
const POLYMARKET_SAFE_ADDRESS: &str = "POLYMARKET_SAFE_ADDRESS";
const POLYMARKET_API_KEY: &str = "POLYMARKET_API_KEY";
const POLYMARKET_API_SECRET: &str = "POLYMARKET_API_SECRET";
const POLYMARKET_PASSPHRASE: &str = "POLYMARKET_PASSPHRASE";

#[tokio::main]
async fn main() {
    // Load environment variables
    let private_key = std::env::var(POLYMARKET_PRIVATE_KEY)
        .expect("POLYMARKET_PRIVATE_KEY not set");
    let safe_address = std::env::var(POLYMARKET_SAFE_ADDRESS)
        .expect("POLYMARKET_SAFE_ADDRESS not set");
    let api_key = std::env::var(POLYMARKET_API_KEY)
        .expect("POLYMARKET_API_KEY not set");
    let api_secret = std::env::var(POLYMARKET_API_SECRET)
        .expect("POLYMARKET_API_SECRET not set");
    let passphrase = std::env::var(POLYMARKET_PASSPHRASE)
        .expect("POLYMARKET_PASSPHRASE not set");

    // Initialize signer
    let signer = init_signer(&private_key)
        .expect("Failed to initialize signer");

    // Initialize shared state
    let state = Arc::new(ExecutionState::new());

    // Initialize HTTP client
    let clob_client = Arc::new(Client::new());

    // Channel for CTF merge tasks
    let (merge_tx, merge_rx) = mpsc::channel::<MergeTask>(100);

    println!("🚀 Starting Polymarket HFT Engine (Live Trading Mode)");

    // 1. Spawn User WebSocket Monitor
    let state_ws = Arc::clone(&state);
    let api_key_ws = api_key.clone();
    let api_secret_ws = api_secret.clone();
    let passphrase_ws = passphrase.clone();
    let merge_tx_ws = merge_tx.clone();

    tokio::spawn(async move {
        run_user_ws(
            api_key_ws,
            api_secret_ws,
            passphrase_ws,
            state_ws,
            merge_tx_ws,
        ).await;
    });

    // 2. Spawn CTF Merge Worker (25 RPM)
    tokio::spawn(async move {
        run_merge_worker(
            merge_rx,
            &api_key,
            &api_secret,
            &passphrase,
            safe_address.parse().unwrap(),
            &private_key,
        ).await;
    });

    // 3. Run HFT Hot Path (pinned to CPU 1)
    let killswitch = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let (tx, rx) = crossbeam_channel::bounded(65536);

    // Background thread for execution
    let state_bg = Arc::clone(&state);
    let signer_bg = signer.clone();
    let clob_bg = Arc::clone(&clob_client);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            while let Ok(task) = rx.recv() {
                match task {
                    BackgroundTask::EdgeDetected { token_hash, combined_price, .. } => {
                        // TODO: Execute Maker + Taker orders
                        // - Sign orders with signer_bg
                        // - POST to CLOB with clob_bg
                        // - Record hedge pair in state_bg
                        println!("[BG] Edge detected: {:016x} at ${:.4}", token_hash, combined_price as f64 / 1_000_000.0);
                    }
                    BackgroundTask::LatencyStats { avg_ns, .. } => {
                        println!("[HFT] 🔥 avg={:.2}µs", avg_ns as f64 / 1000.0);
                    }
                }
            }
        });
    });

    // Fetch tokens and run hot path
    let tokens = fetch_tokens().await;
    let token_pairs = build_token_pairs(&tokens).await;

    run_sync_hot_path(tx, tokens, killswitch, token_pairs);
}

async fn fetch_tokens() -> Vec<String> {
    // Your existing token fetching logic
    vec![]
}

async fn build_token_pairs(tokens: &[String]) -> std::collections::HashMap<u64, u64> {
    // Your existing pair building logic
    std::collections::HashMap::new()
}
```

## ENVIRONMENT SETUP

Create `.env` file on server:

```bash
# Wallet (EOA private key for signing)
POLYMARKET_PRIVATE_KEY=0x...

# Gnosis Safe Proxy Address (holds funds)
POLYMARKET_SAFE_ADDRESS=0x...

# L2 API Credentials
POLYMARKET_API_KEY=...
POLYMARKET_API_SECRET=...  # base64 encoded
POLYMARKET_PASSPHRASE=...

# Mode (dry_run or production)
POLYMARKET_MODE=dry_run

# Safety limits
POLYMARKET_MAX_POSITION=5000000  # $5 in micro-USDC
POLYMARKET_KILLSWITCH_DRAWDOWN=3  # -3% halt
```

## TOKEN ALLOWANCES (ONE-TIME SETUP)

Run these on Polygon with your wallet:

```bash
# Approve USDC.e on Exchange contract
cast send --private-key $PK \
  0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  "approve(address,uint256)" \
  0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E \
  115792089237316195423570985008687907853269984665640564039457584007913129639935

# Approve USDC.e on CTF Exchange
cast send --private-key $PK \
  0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174 \
  "approve(address,uint256)" \
  0x4D97DCd97eC945f40cF65F87097ACe5EA0476045 \
  115792089237316195423570985008687907853269984665640564039457584007913129639935

# Approve Conditional Tokens on CTF Exchange
cast send --private-key $PK \
  0x4D97DCd97eC945f40cF65F87097ACe5EA0476045 \
  "setApprovalForAll(address,bool)" \
  0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E \
  true
```

## TESTING CHECKLIST

### Dry Run Mode
- [ ] EIP-712 signing works with test key
- [ ] Order submission returns valid order_id
- [ ] User WebSocket connects and authenticates
- [ ] Stop-loss timer triggers correctly
- [ ] CTF merge queue processes without 429s
- [ ] Telegram alerts fire on events

### Production Mode
- [ ] Wallet funded with $300-500 USDC.e
- [ ] Token allowances approved
- [ ] API credentials validated
- [ ] First arbitrage executes successfully
- [ ] CTF merge recycles capital
- [ ] PnL tracking accurate

## FILES CREATED THIS SESSION

| File | Lines | Purpose |
|------|-------|---------|
| `signing.rs` | 115 | EIP-712 signing |
| `execution.rs` | 260 | CLOB order submission |
| `merge_worker.rs` | 180 | CTF merge (25 RPM) |
| `stop_loss.rs` | 245 | 3-sec stop-loss |
| `state.rs` | 100 | Shared ExecutionState |
| `user_ws.rs` | 220 | WebSocket monitoring |

**Total:** ~1,120 lines of Rust code

*/