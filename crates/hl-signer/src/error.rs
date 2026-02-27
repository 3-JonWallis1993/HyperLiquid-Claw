use thiserror::Error;

#[derive(Error, Debug)]
pub enum SignerError {
    #[error("Invalid private key: {0}")]
    InvalidKey(String),

    #[error("EIP-712 encoding failed: {0}")]
    EncodingError(String),

    #[error("Signing failed: {0}")]
    SigningFailed(String),

    #[error("Wallet not configured — set HL_PRIVATE_KEY")]
    NoWallet,
}
