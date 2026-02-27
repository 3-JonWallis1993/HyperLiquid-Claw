use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum OrderType {
    /// Market order — fills immediately at best price
    Market,
    /// Limit order with price
    Limit,
    /// Post-only limit (maker only)
    PostOnly,
    /// Stop-market
    StopMarket,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TifType {
    /// Good-till-cancelled
    Gtc,
    /// Immediate-or-cancel
    Ioc,
    /// Fill-or-kill
    Fok,
    /// Algo (Hyperliquid TWAP etc.)
    Alo,
}

/// Request to place an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub client_id: Uuid,
    pub coin: String,
    pub side: OrderSide,
    pub size: Decimal,
    pub order_type: OrderType,
    pub tif: TifType,
    /// Required for Limit / PostOnly / StopMarket
    pub price: Option<Decimal>,
    /// Reduce-only flag (close position only)
    pub reduce_only: bool,
    /// Leverage to set before placing (1–50)
    pub leverage: Option<u32>,
    /// Slippage tolerance for market orders (bps)
    pub slippage_bps: Option<u32>,
}

impl OrderRequest {
    pub fn market(coin: impl Into<String>, side: OrderSide, size: Decimal) -> Self {
        Self {
            client_id: Uuid::new_v4(),
            coin: coin.into(),
            side,
            size,
            order_type: OrderType::Market,
            tif: TifType::Ioc,
            price: None,
            reduce_only: false,
            leverage: None,
            slippage_bps: Some(50), // 0.5% default slippage
        }
    }

    pub fn limit(
        coin: impl Into<String>,
        side: OrderSide,
        size: Decimal,
        price: Decimal,
    ) -> Self {
        Self {
            client_id: Uuid::new_v4(),
            coin: coin.into(),
            side,
            size,
            order_type: OrderType::Limit,
            tif: TifType::Gtc,
            price: Some(price),
            reduce_only: false,
            leverage: None,
            slippage_bps: None,
        }
    }

    pub fn close(coin: impl Into<String>, side: OrderSide, size: Decimal) -> Self {
        let mut req = Self::market(coin, side, size);
        req.reduce_only = true;
        req
    }
}

/// Response after order placement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: u64,
    pub client_id: Uuid,
    pub coin: String,
    pub side: OrderSide,
    pub size: Decimal,
    pub filled_size: Decimal,
    pub avg_fill_price: Option<Decimal>,
    pub status: OrderStatus,
    pub fee_usdc: Decimal,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Filled,
    PartiallyFilled,
    Open,
    Cancelled,
    Rejected,
}

/// A live order sitting on the book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: u64,
    pub coin: String,
    pub side: OrderSide,
    pub size: Decimal,
    pub remaining: Decimal,
    pub price: Decimal,
    pub tif: TifType,
    pub created_at_ms: u64,
}
