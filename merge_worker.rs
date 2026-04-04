//! CTF Merge Worker for Capital Recycling
//! 
//! Burns YES+NO shares to reclaim USDC.e collateral without waiting for market resolution.

use alloy_primitives::{Address, B256, U256, Bytes};
use alloy_sol_types::{sol, SolCall};
use tokio::sync::mpsc::Receiver;
use tokio::time::{sleep, Duration};
use reqwest::Client;
use serde_json::json;

use crate::signing::{init_signer, sign_polymarket_order};
use crate::execution::build_l2_headers;

const RELAYER_URL: &str = "https://relayer.polymarket.com";
const USDC_ADDRESS: &str = "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174";
const CTF_ADDRESS: &str = "0x4D97DCd97eC945f40cF65F87097ACe5EA0476045";

// CTF mergePositions ABI
sol! {
    interface ICTF {
        function mergePositions(
            address collateralToken,
            bytes32 parentCollectionId,
            bytes32 conditionId,
            uint256[] calldata partition,
            uint256 amount
        ) external;
    }
}

/// Task to queue for merge
pub struct MergeTask {
    pub condition_id: B256,
    pub amount: u64,  // micro-USDC shares
}

/// Run the merge worker with 25 RPM rate limit
/// 
/// This should be spawned as a dedicated Tokio task during startup.
/// It listens to a channel and enforces the 25 RPM limit using a strict delay.
pub async fn run_merge_worker(
    mut receiver: Receiver<MergeTask>,
    api_key: &str,
    api_secret: &str,
    api_passphrase: &str,
    safe_address: Address,
    private_key: &str,
) {
    let client = Client::new();
    let signer = match init_signer(private_key) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ Failed to init signer for merge: {}", e);
            return;
        }
    };

    let usdc_address: Address = USDC_ADDRESS.parse().unwrap();
    let ctf_address: Address = CTF_ADDRESS.parse().unwrap();

    while let Some(task) = receiver.recv().await {
        println!("🔄 Attempting CTF Merge for condition: {:?}", task.condition_id);

        // 1. Build the calldata for mergePositions
        // partition for standard binary markets is always [1, 2]
        let merge_call = ICTF::mergePositionsCall {
            collateralToken: usdc_address,
            parentCollectionId: B256::ZERO, // Top-level condition
            conditionId: task.condition_id,
            partition: vec![U256::from(1), U256::from(2)],
            amount: U256::from(task.amount),
        };
        let encoded_data = Bytes::from(merge_call.abi_encode());

        // 2. Build the transaction request
        let tx_request = json!({
            "to": format!("{:?}", ctf_address),
            "value": "0",
            "data": format!("0x{}", hex::encode(&encoded_data)),
            "operation": 0,
            "safeTxGas": "0",
            "baseGas": "0",
            "gasPrice": "0",
            "gasToken": "0x0000000000000000000000000000000000000000",
            "refundReceiver": "0x0000000000000000000000000000000000000000",
            "nonce": "0", // Will be set by relayer
        });

        // 3. Build headers
        let body_str = tx_request.to_string();
        let headers = build_l2_headers(
            api_key,
            api_secret,
            api_passphrase,
            &format!("{:?}", safe_address),
            "POST",
            "/submit",
            &body_str,
        );

        // 4. Submit to Relayer with exponential backoff on 429s
        let mut retries = 0;
        loop {
            match client
                .post(&format!("{}/submit", RELAYER_URL))
                .headers(headers.clone())
                .json(&tx_request)
                .send()
                .await
            {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        println!("✅ CTF Merge Successful! Capital Recycled. Tx: {}", body);
                        break;
                    } else if resp.status().as_u16() == 429 && retries < 3 {
                        println!("⏳ 429 Rate Limit Hit. Backing off...");
                        sleep(Duration::from_millis(5000 * (retries + 1) as u64)).await;
                        retries += 1;
                    } else {
                        eprintln!("❌ Relayer Merge Failed: {}", resp.status());
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("❌ Relayer request error: {:?}", e);
                    break;
                }
            }
        }

        // 5. ENFORCE THE 25 RPM LIMIT
        // Delay exactly 2.4 seconds before processing the next merge
        sleep(Duration::from_millis(2400)).await;
    }
}

/// Get condition_id from Gamma API
pub async fn fetch_condition_id(
    client: &Client,
    market_slug: &str,
) -> Result<B256, String> {
    let url = format!(
        "https://gamma-api.polymarket.com/markets?slug={}",
        market_slug
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Gamma API error: {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let markets: Vec<serde_json::Value> = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    if markets.is_empty() {
        return Err("No markets found".to_string());
    }

    let condition_id = markets[0]
        .get("conditionId")
        .and_then(|v| v.as_str())
        .ok_or("No conditionId in response")?;

    // Parse as bytes32
    B256::parse_str(condition_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addresses() {
        let usdc: Address = USDC_ADDRESS.parse().unwrap();
        let ctf: Address = CTF_ADDRESS.parse().unwrap();
        assert!(!usdc.is_zero());
        assert!(!ctf.is_zero());
    }
}