use rust_decimal::Decimal;

/// Calculate position size in base asset units for a given USDC notional.
///
/// # Arguments
/// * `notional_usdc` – desired position notional in USDC
/// * `mark_price` – current mark price of the asset
/// * `leverage` – leverage to apply (1–50)
/// * `sz_decimals` – number of decimal places for the asset
pub fn position_size_usdc(
    notional_usdc: Decimal,
    mark_price: Decimal,
    leverage: u32,
    sz_decimals: u32,
) -> Decimal {
    if mark_price.is_zero() {
        return Decimal::ZERO;
    }
    let raw = (notional_usdc * Decimal::from(leverage)) / mark_price;
    round_to_decimals(raw, sz_decimals)
}

/// Calculate the maximum safe leverage given account equity and open positions.
///
/// Returns the highest leverage that keeps margin ratio below `max_margin_ratio`.
pub fn max_safe_leverage(
    account_equity: Decimal,
    existing_used_margin: Decimal,
    new_notional: Decimal,
    max_margin_ratio: Decimal,
) -> u32 {
    if account_equity.is_zero() || new_notional.is_zero() {
        return 1;
    }
    // We want: (existing_used + new_notional / lev) / equity <= max_margin_ratio
    // Solving for lev: lev >= new_notional / (equity * max_margin_ratio - existing_used)
    let budget = account_equity * max_margin_ratio - existing_used_margin;
    if budget <= Decimal::ZERO {
        return 1;
    }
    let min_margin = new_notional / budget;
    let max_lev = (Decimal::ONE / min_margin)
        .floor()
        .to_u32_digits()
        .1
        .first()
        .copied()
        .unwrap_or(1);
    max_lev.clamp(1, 50)
}

fn round_to_decimals(val: Decimal, decimals: u32) -> Decimal {
    let factor = Decimal::from(10u64.pow(decimals));
    (val * factor).floor() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_calculation_basic() {
        // $1000 notional at $50k BTC with 1x leverage → 0.02 BTC
        let sz = position_size_usdc(
            Decimal::from(1000),
            Decimal::from(50_000),
            1,
            3,
        );
        assert_eq!(sz, Decimal::new(20, 3)); // 0.020
    }

    #[test]
    fn leverage_magnifies_size() {
        let sz_1x = position_size_usdc(Decimal::from(100), Decimal::from(1000), 1, 3);
        let sz_5x = position_size_usdc(Decimal::from(100), Decimal::from(1000), 5, 3);
        assert_eq!(sz_5x, sz_1x * Decimal::from(5));
    }
}
