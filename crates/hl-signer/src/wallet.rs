use ethers::signers::{LocalWallet, Signer};
use std::str::FromStr;

use crate::error::SignerError;

/// Thin wrapper around an ethers LocalWallet configured for Hyperliquid.
pub struct HlWallet {
    pub inner: LocalWallet,
    pub address: String,
    pub testnet: bool,
}

impl HlWallet {
    /// Load from a hex private key string (with or without 0x prefix).
    pub fn from_key(private_key: &str, testnet: bool) -> Result<Self, SignerError> {
        let key = private_key.trim_start_matches("0x");
        let wallet = LocalWallet::from_str(key)
            .map_err(|e| SignerError::InvalidKey(e.to_string()))?;
        let address = format!("{:#x}", wallet.address());
        Ok(Self { inner: wallet, address, testnet })
    }

    /// Load from HL_PRIVATE_KEY environment variable.
    pub fn from_env(testnet: bool) -> Result<Self, SignerError> {
        let key = std::env::var("HL_PRIVATE_KEY").map_err(|_| SignerError::NoWallet)?;
        Self::from_key(&key, testnet)
    }

    /// Chain id: 1337 for mainnet actions, 421614 for testnet.
    pub fn chain_id(&self) -> u64 {
        if self.testnet { 421_614 } else { 1_337 }
    }
}

impl std::fmt::Debug for HlWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HlWallet(address={}, testnet={})", self.address, self.testnet)
    }
}
