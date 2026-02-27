use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Live ticker data for a perpetual market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticker {
    pub coin: String,
    pub mark_price: Decimal,
    pub oracle_price: Decimal,
    pub funding_rate: Decimal,
    pub open_interest: Decimal,
    pub volume_24h: Decimal,
    pub change_24h: Decimal,
}

/// Static market metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub coin: String,
    pub sz_decimals: u32,
    pub max_leverage: u32,
    pub min_sz: Decimal,
    pub tick_sz: Decimal,
}

/// Combined market snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub info: MarketInfo,
    pub ticker: Ticker,
}

impl Market {
    pub fn spread_bps(&self) -> Decimal {
        let diff = (self.ticker.mark_price - self.ticker.oracle_price).abs();
        if self.ticker.oracle_price.is_zero() {
            return Decimal::ZERO;
        }
        (diff / self.ticker.oracle_price) * Decimal::from(10_000)
    }

    pub fn funding_apr(&self) -> Decimal {
        // 8-hour funding → annualise (3 periods/day × 365)
        self.ticker.funding_rate * Decimal::from(3 * 365)
    }
}
