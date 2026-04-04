//! CLOB Order Execution for Polymarket
//! 
//! Uses polymarket-client-sdk OrderBuilder for order creation and signing.

use reqwest::{Client, StatusCode, header::{HeaderMap, HeaderValue, CONTENT_TYPE}};
use serde_json::{json, Value};
use tokio::time::{sleep, Duration};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

// Re-export from signing
pub use crate::signing::{Order, Side, OrderType, SignatureType, SignableOrder, OrderBuilder, PrivateKeySigner};
use alloy_primitives::Address;
use rust_decimal::Decimal;

const CLOB_BASE: &str = "https://clob.polymarket.com";

/// Build L2 authentication headers for Polymarket API
pub fn build_l2_headers(
    api_key: &str,
    api_secret: &str,
    api_passphrase: &str,
    signer_address: &str,
    method: &str,
    request_path: &str,
    body_str: &str,
) -> HeaderMap {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    // Message: timestamp + method + request_path + body
    let msg = format!("{}{}{}{}", timestamp, method, request_path, body_str);

    // Decode base64 API secret
    let decoded_secret = BASE64.decode(api_secret).expect("Invalid base64 secret");

    // HMAC-SHA256 signature
    let mut mac = Hmac::<Sha256>::new_from_slice(&decoded_secret).expect("HMAC key error");
    mac.update(msg.as_bytes());
    let signature_bytes = mac.finalize().into_bytes();
    let signature_b64 = BASE64.encode(signature_bytes);

    // Build headers
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("POLY_ADDRESS", HeaderValue::from_str(signer_address).unwrap());
    headers.insert("POLY_API_KEY", HeaderValue::from_str(api_key).unwrap());
    headers.insert("POLY_PASSPHRASE", HeaderValue::from_str(api_passphrase).unwrap());
    headers.insert("POLY_TIMESTAMP", HeaderValue::from_str(&timestamp).unwrap());
    headers.insert("POLY_SIGNATURE", HeaderValue::from_str(&signature_b64).unwrap());

    headers
}

/// Fetch dynamic fee rate for a token
pub async fn fetch_fee_rate(client: &Client, token_id: &str) -> Result<u64, String> {
    let url = format!("{}/fee-rate?token_id={}", CLOB_BASE, token_id);
    
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Fee rate fetch error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to fetch fee rate: {}", resp.status()));
    }

    let body = resp.text().await.map_err(|e| e.to_string())?;
    let json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    // feeRateBps is the fee in basis points
    json.get("feeRateBps")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "No feeRateBps in response".to_string())
}

/// Submit order with exponential backoff for 429 errors
pub async fn submit_order_with_backoff(
    client: &Client,
    order: &Order,
    api_key: &str,
    api_secret: &str,
    api_passphrase: &str,
    signer_address: &str,
    max_retries: u32,
) -> Result<Value, String> {
    let mut retry_delay_ms = 100; // Start with 100ms
    
    for attempt in 0..=max_retries {
        let body_str = serde_json::to_string(order).map_err(|e| e.to_string())?;
        let headers = build_l2_headers(
            api_key,
            api_secret,
            api_passphrase,
            signer_address,
            "POST",
            "/order",
            &body_str,
        );

        let resp = client
            .post(&format!("{}/order", CLOB_BASE))
            .headers(headers)
            .json(order)
            .send()
            .await
            .map_err(|e| format!("Request error: {}", e))?;

        match resp.status() {
            StatusCode::OK | StatusCode::CREATED => {
                let body = resp.text().await.map_err(|e| e.to_string())?;
                let json: Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
                return Ok(json);
            }
            StatusCode::TOO_MANY_REQUESTS => {
                if attempt == max_retries {
                    return Err("Max retries exceeded for rate limit".to_string());
                }
                sleep(Duration::from_millis(retry_delay_ms)).await;
                retry_delay_ms *= 2; // Exponential backoff
            }
            status => {
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Order rejected: {} - {}", status, body));
            }
        }
    }
    
    Err("Unexpected error".to_string())
}

/// Build a limit order using OrderBuilder (SDK approach)
pub fn build_limit_order(
    maker: Address,
    token_id: &str,
    price: Decimal,
    size: Decimal,
    side: Side,
    fee_rate_bps: u64,
) -> Order {
    use polymarket_client_sdk::clob::order_builder::Limit;
    
    OrderBuilder::new()
        .maker(maker)
        .token_id(token_id.to_string())
        .price(price)
        .size(size)
        .side(side)
        .order_type(OrderType::GTC)
        .fee_rate_bps(fee_rate_bps)
        .build::<Limit, polymarket_client_sdk::clob::NoAuth>()
        .expect("Failed to build order")
}

/// Build a FAK (Fill-And-Kill) order for stop-loss
pub fn build_fak_order(
    maker: Address,
    token_id: &str,
    price: Decimal,
    size: Decimal,
    side: Side,
    fee_rate_bps: u64,
) -> Order {
    use polymarket_client_sdk::clob::order_builder::Fak;
    
    OrderBuilder::new()
        .maker(maker)
        .token_id(token_id.to_string())
        .price(price)
        .size(size)
        .side(side)
        .order_type(OrderType::FOK) // Fill-Or-Kill for immediate execution
        .fee_rate_bps(fee_rate_bps)
        .build::<Fak, polymarket_client_sdk::clob::NoAuth>()
        .expect("Failed to build FAK order")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_l2_headers() {
        let headers = build_l2_headers(
            "test_key",
            "dGVzdF9zZWNyZXQ=", // base64 of "test_secret"
            "test_pass",
            "0x1234",
            "POST",
            "/order",
            "{}",
        );
        
        assert!(headers.contains_key("POLY_ADDRESS"));
        assert!(headers.contains_key("POLY_API_KEY"));
        assert!(headers.contains_key("POLY_TIMESTAMP"));
        assert!(headers.contains_key("POLY_SIGNATURE"));
    }
}