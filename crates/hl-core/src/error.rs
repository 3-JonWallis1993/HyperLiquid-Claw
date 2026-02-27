use thiserror::Error;

#[derive(Error, Debug)]
pub enum HlError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("API error: code={code}, msg={msg}")]
    Api { code: i64, msg: String },

    #[error("Order rejected: {reason}")]
    OrderRejected { reason: String },

    #[error("Insufficient margin: required={required}, available={available}")]
    InsufficientMargin { required: String, available: String },

    #[error("Market not found: {coin}")]
    MarketNotFound { coin: String },

    #[error("Signer error: {0}")]
    Signer(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Rate limit exceeded, retry after {retry_after}ms")]
    RateLimit { retry_after: u64 },

    #[error("Unexpected response: {0}")]
    Unexpected(String),
}
