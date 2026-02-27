use anyhow::Result;
use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;
use tracing::{debug, instrument};

use crate::{
    error::HlError,
    market::{Market, MarketInfo, Ticker},
    order::{OrderRequest, OrderResponse, OrderSide, OrderStatus},
    position::{AccountState, Position, PositionSide},
};

const MAINNET_URL: &str = "https://api.hyperliquid.xyz";
const TESTNET_URL: &str = "https://api.hyperliquid-testnet.xyz";

pub struct HlClient {
    http: Client,
    base_url: String,
    address: Option<String>,
}

impl HlClient {
    pub fn new(testnet: bool) -> Self {
        let base_url = if testnet {
            TESTNET_URL.to_string()
        } else {
            MAINNET_URL.to_string()
        };
        Self {
            http: Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .expect("failed to build HTTP client"),
            base_url,
            address: None,
        }
    }

    pub fn with_address(mut self, address: impl Into<String>) -> Self {
        self.address = Some(address.into());
        self
    }

    // ─── Info endpoint ────────────────────────────────────────────────────────

    #[instrument(skip(self))]
    pub async fn get_all_mids(&self) -> Result<Vec<Ticker>, HlError> {
        let body = json!({ "type": "allMids" });
        let resp: Value = self.info_post(body).await?;

        // Parse tickers from the response map
        let mut tickers = Vec::new();
        if let Some(obj) = resp.as_object() {
            for (coin, price_val) in obj {
                let mark = parse_decimal(price_val)?;
                tickers.push(Ticker {
                    coin: coin.clone(),
                    mark_price: mark,
                    oracle_price: mark,    // placeholder; real impl fetches ctx
                    funding_rate: Decimal::ZERO,
                    open_interest: Decimal::ZERO,
                    volume_24h: Decimal::ZERO,
                    change_24h: Decimal::ZERO,
                });
            }
        }
        Ok(tickers)
    }

    #[instrument(skip(self))]
    pub async fn get_meta(&self) -> Result<Vec<MarketInfo>, HlError> {
        let body = json!({ "type": "meta" });
        let resp: Value = self.info_post(body).await?;

        let mut markets = Vec::new();
        if let Some(universe) = resp["universe"].as_array() {
            for m in universe {
                markets.push(MarketInfo {
                    coin: m["name"].as_str().unwrap_or("").to_string(),
                    sz_decimals: m["szDecimals"].as_u64().unwrap_or(3) as u32,
                    max_leverage: m["maxLeverage"].as_u64().unwrap_or(20) as u32,
                    min_sz: Decimal::from_str(m["minSz"].as_str().unwrap_or("0.001"))
                        .unwrap_or(Decimal::new(1, 3)),
                    tick_sz: Decimal::from_str(m["tickSz"].as_str().unwrap_or("0.01"))
                        .unwrap_or(Decimal::new(1, 2)),
                });
            }
        }
        Ok(markets)
    }

    pub async fn get_markets(&self) -> Result<Vec<Market>, HlError> {
        let (infos, tickers) = tokio::try_join!(self.get_meta(), self.get_all_mids())?;
        let markets = infos
            .into_iter()
            .filter_map(|info| {
                let ticker = tickers.iter().find(|t| t.coin == info.coin)?.clone();
                Some(Market { info, ticker })
            })
            .collect();
        Ok(markets)
    }

    pub async fn get_market(&self, coin: &str) -> Result<Market, HlError> {
        let markets = self.get_markets().await?;
        markets
            .into_iter()
            .find(|m| m.info.coin.eq_ignore_ascii_case(coin))
            .ok_or_else(|| HlError::MarketNotFound {
                coin: coin.to_string(),
            })
    }

    #[instrument(skip(self))]
    pub async fn get_account_state(&self) -> Result<AccountState, HlError> {
        let addr = self.address.as_deref().unwrap_or_default();
        let body = json!({ "type": "clearinghouseState", "user": addr });
        let resp: Value = self.info_post(body).await?;

        let equity = parse_decimal(&resp["marginSummary"]["accountValue"])?;
        let available = parse_decimal(&resp["withdrawable"])?;
        let total_pnl = parse_decimal(&resp["marginSummary"]["unrealizedPnl"])?;
        let used = parse_decimal(&resp["marginSummary"]["totalMarginUsed"])?;
        let ratio = if equity.is_zero() {
            Decimal::ZERO
        } else {
            used / equity
        };

        let mut positions = Vec::new();
        if let Some(pos_array) = resp["assetPositions"].as_array() {
            for p in pos_array {
                let pos_data = &p["position"];
                let size = parse_decimal(&pos_data["szi"])?;
                if size.is_zero() {
                    continue;
                }
                let side = if size > Decimal::ZERO {
                    PositionSide::Long
                } else {
                    PositionSide::Short
                };
                positions.push(Position {
                    coin: pos_data["coin"].as_str().unwrap_or("").to_string(),
                    side,
                    size: size.abs(),
                    entry_price: parse_decimal(&pos_data["entryPx"])?,
                    mark_price: parse_decimal(&pos_data["positionValue"])
                        .unwrap_or(Decimal::ZERO),
                    liquidation_price: pos_data["liquidationPx"]
                        .as_str()
                        .and_then(|s| Decimal::from_str(s).ok()),
                    leverage: parse_decimal(&pos_data["leverage"]["value"])
                        .unwrap_or(Decimal::ONE),
                    margin_used: parse_decimal(&pos_data["marginUsed"])?,
                    unrealized_pnl: parse_decimal(&pos_data["unrealizedPnl"])?,
                    cumulative_funding: parse_decimal(&pos_data["cumFunding"]["allTime"])
                        .unwrap_or(Decimal::ZERO),
                    return_on_equity: Decimal::ZERO, // computed separately
                });
            }
        }

        Ok(AccountState {
            address: addr.to_string(),
            equity,
            available_margin: available,
            used_margin: used,
            account_value: equity,
            total_unrealized_pnl: total_pnl,
            margin_ratio: ratio,
            positions,
        })
    }

    // ─── Exchange endpoint (orders) ───────────────────────────────────────────

    pub async fn place_order(
        &self,
        req: &OrderRequest,
        signed_payload: Value,
    ) -> Result<OrderResponse, HlError> {
        debug!("Placing {:?} order for {} {}", req.side, req.size, req.coin);
        let resp: Value = self.exchange_post(signed_payload).await?;

        let status_str = resp["response"]["data"]["statuses"][0]
            .as_str()
            .unwrap_or("rejected");

        let status = match status_str {
            "filled" => OrderStatus::Filled,
            "resting" => OrderStatus::Open,
            _ => OrderStatus::Rejected,
        };

        Ok(OrderResponse {
            order_id: resp["response"]["data"]["statuses"][0]["resting"]["oid"]
                .as_u64()
                .unwrap_or(0),
            client_id: req.client_id,
            coin: req.coin.clone(),
            side: req.side,
            size: req.size,
            filled_size: req.size,
            avg_fill_price: req.price,
            status,
            fee_usdc: Decimal::ZERO,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        })
    }

    pub async fn cancel_order(&self, coin: &str, order_id: u64, signed: Value) -> Result<bool, HlError> {
        let resp: Value = self.exchange_post(signed).await?;
        let ok = resp["response"]["data"]["statuses"][0] == json!("success");
        if !ok {
            tracing::warn!("Cancel order {order_id} on {coin} may have failed: {resp}");
        }
        Ok(ok)
    }

    // ─── Internal helpers ─────────────────────────────────────────────────────

    async fn info_post(&self, body: Value) -> Result<Value, HlError> {
        let url = format!("{}/info", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        Ok(resp)
    }

    async fn exchange_post(&self, body: Value) -> Result<Value, HlError> {
        let url = format!("{}/exchange", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;

        if resp["status"] == json!("err") {
            return Err(HlError::Api {
                code: -1,
                msg: resp["response"].as_str().unwrap_or("unknown").to_string(),
            });
        }
        Ok(resp)
    }
}

fn parse_decimal(val: &Value) -> Result<Decimal, HlError> {
    let s = match val {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        _ => "0".to_string(),
    };
    Decimal::from_str(&s).map_err(|e| HlError::Unexpected(e.to_string()))
}
