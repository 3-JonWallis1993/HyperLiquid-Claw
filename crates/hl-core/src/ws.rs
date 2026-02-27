/// WebSocket feed for real-time market data from Hyperliquid.
///
/// Hyperliquid exposes a single WS endpoint at wss://api.hyperliquid.xyz/ws
/// Subscriptions are JSON messages of the form:
///   { "method": "subscribe", "subscription": { "type": "...", ... } }
///
/// This module provides typed subscription helpers and a channel-based event pump.
use serde::{Deserialize, Serialize};

pub const WS_MAINNET: &str = "wss://api.hyperliquid.xyz/ws";
pub const WS_TESTNET: &str = "wss://api.hyperliquid-testnet.xyz/ws";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsSubscription {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl WsSubscription {
    /// Subscribe to all mid-prices (fast ticker)
    pub fn all_mids() -> Self {
        Self { r#type: "allMids".to_string(), coin: None, user: None }
    }

    /// Subscribe to L2 order book for a specific coin
    pub fn l2_book(coin: impl Into<String>) -> Self {
        Self { r#type: "l2Book".to_string(), coin: Some(coin.into()), user: None }
    }

    /// Subscribe to trades feed for a coin
    pub fn trades(coin: impl Into<String>) -> Self {
        Self { r#type: "trades".to_string(), coin: Some(coin.into()), user: None }
    }

    /// Subscribe to user fills (requires address)
    pub fn user_fills(address: impl Into<String>) -> Self {
        Self { r#type: "userFills".to_string(), coin: None, user: Some(address.into()) }
    }

    /// Subscribe to user funding payments
    pub fn user_funding(address: impl Into<String>) -> Self {
        Self {
            r#type: "userFunding".to_string(),
            coin: None,
            user: Some(address.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    pub method: String,
    pub subscription: WsSubscription,
}

impl WsMessage {
    pub fn subscribe(sub: WsSubscription) -> Self {
        Self { method: "subscribe".to_string(), subscription: sub }
    }
    pub fn unsubscribe(sub: WsSubscription) -> Self {
        Self { method: "unsubscribe".to_string(), subscription: sub }
    }
}

/// Parsed inbound WS events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "channel", content = "data")]
pub enum WsEvent {
    #[serde(rename = "allMids")]
    AllMids(serde_json::Value),
    #[serde(rename = "l2Book")]
    L2Book(serde_json::Value),
    #[serde(rename = "trades")]
    Trades(serde_json::Value),
    #[serde(rename = "userFills")]
    UserFills(serde_json::Value),
    #[serde(rename = "userFunding")]
    UserFunding(serde_json::Value),
    #[serde(other)]
    Unknown,
}
