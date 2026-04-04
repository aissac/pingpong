# Polymarket SDK Implementation - Rust

## From NotebookLM (2026-03-30)

### 1. Official Rust SDK

**GitHub:** `Polymarket/rs-clob-client`
**Crate:** `polymarket-client-sdk` (version 0.2.1)

**Add to Cargo.toml:**
```toml
[dependencies]
polymarket-client-sdk = "0.2.1"
tokio = { version = "1", features = ["full"] }
```

### 2. Authentication Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    L1 (Private Key)                         │
│         Proves wallet ownership via EIP-712                 │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│           SDK: .authenticate()                               │
│           - Signs authorization message                      │
│           - Derives L2 credentials                           │
│           - Stores in client memory                          │
└────────────────────────┬────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                    L2 (API Key)                              │
│         apiKey, secret, passphrase                           │
│         Signs REST requests via HMAC-SHA256                  │
└─────────────────────────────────────────────────────────────┘
```

### 3. Complete Implementation Code

```rust
use std::str::FromStr;
use polymarket_client_sdk::POLYGON;
use polymarket_client_sdk::auth::{LocalSigner, Signer};
use polymarket_client_sdk::clob::{Client, Config};
use polymarket_client_sdk::types::{Side, SignatureType};
use polymarket_client_sdk::types::dec;

async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load private key from env
    let private_key = std::env::var("POLYMARKET_PRIVATE_KEY")
        .expect("Missing POLYMARKET_PRIVATE_KEY");

    // 1. Initialize L1 Signer
    let signer = LocalSigner::from_str(&private_key)?
        .with_chain_id(Some(POLYGON)); // Polygon chain ID = 137

    // 2. Authenticate & Derive L2 Credentials
    // CRITICAL: Set signature type based on wallet:
    // - EOA = 0 (MetaMask standard wallet)
    // - Proxy = 1 (Magic Link)
    // - GnosisSafe = 2 (Polymarket Builder Program - gasless)
    let client = Client::new("https://clob.polymarket.com", Config::default())?
        .authentication_builder(&signer)
        .signature_type(SignatureType::GnosisSafe) // Type 2 for gasless
        .authenticate()
        .await?;

    // (Optional) Cache L2 credentials for future restarts
    let creds = client.credentials();
    println!("API Key Derived: {}", creds.key());

    // 3. Build Maker Order (Post-Only GTC)
    let maker_order = client.limit_order()
        .token_id("YOUR_TOKEN_ID".parse()?)
        .price(dec!(0.475))    // $0.475
        .size(dec!(100))        // 100 shares
        .side(Side::Buy)        // BUY
        .post_only(true)        // GTC Post-Only
        .build()
        .await?;

    // 4. Sign the order (L1 EIP-712)
    let signed_maker = client.sign(&signer, maker_order).await?;

    // 5. Submit order (SDK adds L2 HMAC headers automatically)
    let response = client.post_order(signed_maker).await?;
    println!("Maker Order ID: {}", response.order_id);

    // Wait for User WebSocket "MATCHED" event...
    // Then fire Taker order (FAK)

    let taker_order = client.limit_order()
        .token_id("OTHER_TOKEN_ID".parse()?)
        .price(dec!(0.475))
        .size(dec!(100))
        .side(Side::Buy)
        .time_in_force("FAK")    // Fill-And-Kill
        .build()
        .await?;

    let signed_taker = client.sign(&signer, taker_order).await?;
    let taker_response = client.post_order(signed_taker).await?;
    println!("Taker Order ID: {}", taker_response.order_id);

    Ok(())
}
```

### 4. Wallet Types (SignatureType)

| Type | Value | Use Case |
|------|-------|----------|
| `EOA` | 0 | Standard MetaMask wallet |
| `Proxy` | 1 | Magic Link |
| `GnosisSafe` | 2 | Polymarket Builder Program (gasless) |

### 5. Order Types

**Maker Order (Post-Only GTC):**
```rust
client.limit_order()
    .token_id(token_id)
    .price(dec!(0.475))
    .size(dec!(100))
    .side(Side::Buy)
    .post_only(true)  // Key: Post-Only
    .build()
```

**Taker Order (FAK):**
```rust
client.limit_order()
    .token_id(token_id)
    .price(dec!(0.475))
    .size(dec!(100))
    .side(Side::Buy)
    .time_in_force("FAK")  // Key: Fill-And-Kill
    .build()
```

### 6. Key Notes

1. **SDK handles everything:** EIP-712 signing, HMAC headers, feeRateBps fetching
2. **Gnosis Safe users:** Must use `SignatureType::GnosisSafe` (Type 2)
3. **Cache credentials:** Use `client.credentials()` to get L2 keys for .env
4. **Fee calculation:** SDK automatically fetches `feeRateBps` for the token

---

## Next Steps

1. [ ] Add `polymarket-client-sdk` to Cargo.toml
2. [ ] Create `src/auth.rs` for authentication
3. [ ] Create `src/order.rs` for order submission
4. [ ] Open User WebSocket for fill confirmations
5. [ ] Implement Maker → Wait → Taker flow

---

*Source: NotebookLM conversation 2026-03-30*