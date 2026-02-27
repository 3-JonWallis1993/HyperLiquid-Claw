use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PositionSide {
    Long,
    Short,
}

impl std::fmt::Display for PositionSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionSide::Long => write!(f, "LONG"),
            PositionSide::Short => write!(f, "SHORT"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub coin: String,
    pub side: PositionSide,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub mark_price: Decimal,
    pub liquidation_price: Option<Decimal>,
    pub leverage: Decimal,
    pub margin_used: Decimal,
    pub unrealized_pnl: Decimal,
    pub cumulative_funding: Decimal,
    pub return_on_equity: Decimal,
}

impl Position {
    /// Notional value at current mark price
    pub fn notional(&self) -> Decimal {
        self.size * self.mark_price
    }

    /// PnL as percentage of margin used
    pub fn pnl_pct(&self) -> Decimal {
        if self.margin_used.is_zero() {
            return Decimal::ZERO;
        }
        (self.unrealized_pnl / self.margin_used) * Decimal::from(100)
    }

    /// Distance to liquidation in percent
    pub fn liquidation_distance_pct(&self) -> Option<Decimal> {
        self.liquidation_price.map(|liq| {
            let dist = (self.mark_price - liq).abs();
            (dist / self.mark_price) * Decimal::from(100)
        })
    }

    pub fn is_profitable(&self) -> bool {
        self.unrealized_pnl > Decimal::ZERO
    }
}

/// Account-level margin summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub address: String,
    pub equity: Decimal,
    pub available_margin: Decimal,
    pub used_margin: Decimal,
    pub account_value: Decimal,
    pub total_unrealized_pnl: Decimal,
    pub margin_ratio: Decimal,
    pub positions: Vec<Position>,
}
