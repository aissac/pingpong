//! EIP-712 Signing for Polymarket Orders
//! 
//! Implements typed data signing for gasless Gnosis Safe transactions on Polygon.

use alloy_primitives::{address, Address, U256};
use alloy_sol_types::{eip712_domain, sol, Eip712Domain, SolStruct};
use alloy_signer::SignerSync;
use alloy_signer_local::PrivateKeySigner;
use std::str::FromStr;

// Order struct for Polymarket CLOB
sol! {
    #[derive(Debug, Clone)]
    pub struct Order {
        uint256 salt;
        address maker;
        address signer;
        address taker;
        uint256 tokenId;
        uint256 makerAmount;
        uint256 takerAmount;
        uint256 expiration;
        uint256 nonce;
        uint256 feeRateBps;
        uint8 side;
        uint8 signatureType;
    }
}

// Polymarket EIP-712 Domain Constants
pub const POLYGON_CHAIN_ID: u64 = 137;
pub const EXCHANGE_ADDRESS: Address = address!("4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E");

/// Returns the exact EIP-712 domain required by Polymarket's CTF Exchange
pub fn get_ctf_domain() -> Eip712Domain {
    eip712_domain! {
        name: "CTFExchange",
        version: "1",
        chain_id: POLYGON_CHAIN_ID,
        verifying_contract: EXCHANGE_ADDRESS,
    }
}

/// Initialize the signer from a private key hex string
/// Accepts keys with or without the "0x" prefix
pub fn init_signer(private_key_hex: &str) -> Result<PrivateKeySigner, String> {
    let pk = private_key_hex.trim_start_matches("0x");
    PrivateKeySigner::from_str(pk).map_err(|e| e.to_string())
}

/// Synchronously signs the order and formats the signature for the API payload
/// Ideal for HFT as it avoids async/await context switching overhead
pub fn sign_polymarket_order(
    order: &Order,
    signer: &PrivateKeySigner,
) -> Result<String, String> {
    let domain = get_ctf_domain();

    // Sign the EIP-712 typed data payload synchronously
    let signature = signer
        .sign_typed_data_sync(order, &domain)
        .map_err(|e| e.to_string())?;

    // Encode the 65-byte signature into a hex string with a 0x prefix
    Ok(format!("0x{}", hex::encode(signature.as_bytes())))
}

/// Generate a random salt for the order
pub fn generate_salt() -> U256 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    U256::from(timestamp)
}

/// Create a Polymarket order for signing
#[allow(clippy::too_many_arguments)]
pub fn create_order(
    maker: Address,
    signer: Address,
    taker: Address,
    token_id: &str,
    maker_amount: u64,
    taker_amount: u64,
    fee_rate_bps: u64,
    side: u8,
) -> Order {
    Order {
        salt: generate_salt(),
        maker,
        signer,
        taker,
        tokenId: U256::from_str_radix(token_id, 10).unwrap_or_default(),
        makerAmount: U256::from(maker_amount),
        takerAmount: U256::from(taker_amount),
        feeRateBps: U256::from(fee_rate_bps),
        side,
        signatureType: 2, // GNOSIS_SAFE (gasless)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_creation() {
        let domain = get_ctf_domain();
        assert_eq!(domain.chain_id.unwrap(), POLYGON_CHAIN_ID);
    }

    #[test]
    fn test_init_signer() {
        // Test with a dummy private key
        let pk = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let signer = init_signer(pk);
        assert!(signer.is_ok());
    }
}