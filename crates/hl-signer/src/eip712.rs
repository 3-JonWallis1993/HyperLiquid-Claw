/// EIP-712 signing for Hyperliquid order actions.
///
/// Hyperliquid uses a custom EIP-712 domain and typed data structure for all
/// exchange actions. This module encodes the canonical wire format and produces
/// signed payloads ready for submission to /exchange.
use anyhow::Result;
use ethers::{
    signers::Signer,
    types::{Bytes, H256},
    utils::keccak256,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{error::SignerError, wallet::HlWallet};

// ─── EIP-712 Domain ───────────────────────────────────────────────────────────

const DOMAIN_NAME: &str = "Exchange";
const DOMAIN_VERSION: &str = "1";

fn domain_separator(chain_id: u64) -> H256 {
    // keccak256(abi.encode(TYPE_HASH, name_hash, version_hash, chainId, verifyingContract))
    // Hyperliquid uses a minimal domain without verifyingContract
    let type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak256(DOMAIN_NAME.as_bytes());
    let version_hash = keccak256(DOMAIN_VERSION.as_bytes());

    // Verifying contract = 0x0000...0000 (empty for Hyperliquid)
    let mut encoded = [0u8; 5 * 32];
    encoded[..32].copy_from_slice(&type_hash);
    encoded[32..64].copy_from_slice(&name_hash);
    encoded[64..96].copy_from_slice(&version_hash);
    // chain_id as big-endian u256
    let chain_bytes = chain_id.to_be_bytes();
    encoded[120..128].copy_from_slice(&chain_bytes);
    // verifyingContract stays zeroed

    H256::from(keccak256(encoded))
}

// ─── Order wire format ────────────────────────────────────────────────────────

/// Wire-format representation of a single order for EIP-712 encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderWire {
    /// Asset index (integer position in meta universe array)
    pub asset: u32,
    /// true = buy/long, false = sell/short
    pub is_buy: bool,
    /// Limit price as string (use "0" for market orders with slippage)
    pub limit_px: String,
    /// Size in base asset
    pub sz: String,
    /// Reduce-only flag
    pub reduce_only: bool,
    /// Order type: { "limit": {"tif": "Gtc"} } or { "trigger": {...} }
    pub order_type: Value,
}

/// A complete order action ready for signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAction {
    pub r#type: String,
    pub orders: Vec<OrderWire>,
    /// Grouping: "na" (none), "normalTpsl", "positionTpsl"
    pub grouping: String,
}

impl OrderAction {
    pub fn single(order: OrderWire) -> Self {
        Self {
            r#type: "order".to_string(),
            orders: vec![order],
            grouping: "na".to_string(),
        }
    }

    pub fn market_buy(asset: u32, sz: Decimal, slippage_bps: u32, mark_px: Decimal) -> OrderWire {
        // Market orders are submitted as IOC limit orders with slippage
        let slippage = Decimal::new(slippage_bps as i64, 4); // bps → fraction
        let limit = mark_px * (Decimal::ONE + slippage);
        OrderWire {
            asset,
            is_buy: true,
            limit_px: format!("{:.5}", limit),
            sz: format!("{:.5}", sz),
            reduce_only: false,
            order_type: json!({ "limit": { "tif": "Ioc" } }),
        }
    }

    pub fn market_sell(asset: u32, sz: Decimal, slippage_bps: u32, mark_px: Decimal) -> OrderWire {
        let slippage = Decimal::new(slippage_bps as i64, 4);
        let limit = mark_px * (Decimal::ONE - slippage);
        OrderWire {
            asset,
            is_buy: false,
            limit_px: format!("{:.5}", limit),
            sz: format!("{:.5}", sz),
            reduce_only: false,
            order_type: json!({ "limit": { "tif": "Ioc" } }),
        }
    }
}

// ─── Signing ──────────────────────────────────────────────────────────────────

/// Sign an order action and return the full JSON payload for /exchange.
pub async fn sign_order_action(
    wallet: &HlWallet,
    action: &OrderAction,
    nonce: u64,
) -> Result<Value, SignerError> {
    // Hyperliquid's EIP-712 hash for order actions:
    // hash = keccak256("\x19\x01" || domainSeparator || structHash)

    let action_bytes = serde_json::to_vec(action)
        .map_err(|e| SignerError::EncodingError(e.to_string()))?;

    // struct hash = keccak256(typeHash || encode(action fields))
    let type_hash = keccak256(b"HyperliquidTransaction:Order(string hyperliquidChain,address destination,uint64 nonce)");
    let chain_str = if wallet.testnet { "Testnet" } else { "Mainnet" };
    let chain_hash = keccak256(chain_str.as_bytes());

    let mut struct_data = [0u8; 3 * 32];
    struct_data[..32].copy_from_slice(&type_hash);
    struct_data[32..64].copy_from_slice(&chain_hash);
    let nonce_bytes = nonce.to_be_bytes();
    struct_data[88..96].copy_from_slice(&nonce_bytes);
    let struct_hash = keccak256(struct_data);

    let domain = domain_separator(wallet.chain_id());
    let mut digest_input = [0u8; 66];
    digest_input[0] = 0x19;
    digest_input[1] = 0x01;
    digest_input[2..34].copy_from_slice(domain.as_bytes());
    digest_input[34..66].copy_from_slice(&struct_hash);
    let digest = H256::from(keccak256(digest_input));

    // Sign the digest
    let signature = wallet
        .inner
        .sign_hash(digest)
        .map_err(|e| SignerError::SigningFailed(e.to_string()))?;

    let sig_bytes: Bytes = signature.to_vec().into();
    let sig_hex = format!("0x{}", hex::encode(&sig_bytes));

    Ok(json!({
        "action": action,
        "nonce": nonce,
        "signature": {
            "r": format!("0x{:064x}", signature.r),
            "s": format!("0x{:064x}", signature.s),
            "v": signature.v
        },
        "vaultAddress": null
    }))
}
