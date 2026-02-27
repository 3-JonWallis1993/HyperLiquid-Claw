use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Coverage tier for a hedge pair
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CoverageTier {
    /// < 85% — speculative, usually filtered
    Low,
    /// 85–90% — moderate hedge
    Moderate,
    /// 90–95% — good hedge
    Good,
    /// ≥ 95% — near-arbitrage
    High,
}

impl CoverageTier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::High => "T1 (≥95%) HIGH",
            Self::Good => "T2 (90-95%) GOOD",
            Self::Moderate => "T3 (85-90%) MODERATE",
            Self::Low => "T4 (<85%) LOW",
        }
    }

    pub fn emoji(&self) -> &'static str {
        match self {
            Self::High => "🟢",
            Self::Good => "🔵",
            Self::Moderate => "🟡",
            Self::Low => "🔴",
        }
    }
}

/// A scored hedge opportunity between two Hyperliquid markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HedgePair {
    /// Primary (target) market
    pub target_coin: String,
    /// Covering (hedge) market
    pub cover_coin: String,
    /// Coverage percentage (0–100)
    pub coverage_pct: Decimal,
    /// Tier based on coverage
    pub tier: CoverageTier,
    /// Correlation coefficient (-1 to 1)
    pub correlation: Decimal,
    /// Estimated combined funding cost (APR)
    pub combined_funding_apr: Decimal,
    /// Human-readable rationale from LLM
    pub rationale: String,
}

impl HedgePair {
    pub fn summary(&self) -> String {
        format!(
            "{} {} ↔ {} | cov={:.1}% | corr={:.2} | funding_apr={:.2}%",
            self.tier.emoji(),
            self.target_coin,
            self.cover_coin,
            self.coverage_pct,
            self.correlation,
            self.combined_funding_apr,
        )
    }
}

/// Score a candidate hedge pair given their price correlation and funding rates.
///
/// # Arguments
/// * `correlation` – Pearson correlation of daily returns (positive = same direction)
/// * `target_is_long` – true if we are long the target market
/// * `cover_is_short` – true if we are short the cover market
/// * `combined_funding_apr` – net annualised funding cost of holding both legs
pub fn score_hedge(
    correlation: Decimal,
    target_is_long: bool,
    cover_is_short: bool,
    combined_funding_apr: Decimal,
) -> (Decimal, CoverageTier) {
    // A perfect hedge has correlation = +1 with opposite positions.
    // Coverage = correlation * 100 when directions are correct.
    let directional_match = target_is_long != cover_is_short; // XOR = opposite directions
    let raw_coverage = if directional_match {
        correlation.abs() * Decimal::from(100)
    } else {
        (Decimal::ONE - correlation.abs()) * Decimal::from(100)
    };

    // Penalise for funding cost: -1% coverage per 5% APR
    let funding_penalty = combined_funding_apr / Decimal::from(5);
    let coverage = (raw_coverage - funding_penalty).max(Decimal::ZERO);

    let tier = if coverage >= Decimal::from(95) {
        CoverageTier::High
    } else if coverage >= Decimal::from(90) {
        CoverageTier::Good
    } else if coverage >= Decimal::from(85) {
        CoverageTier::Moderate
    } else {
        CoverageTier::Low
    };

    (coverage, tier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_hedge_is_high_tier() {
        let (cov, tier) = score_hedge(
            Decimal::ONE,
            true,
            true,
            Decimal::ZERO,
        );
        assert_eq!(tier, CoverageTier::High);
        assert!(cov >= Decimal::from(95));
    }

    #[test]
    fn high_funding_reduces_coverage() {
        let (cov_low_funding, _) =
            score_hedge(Decimal::new(98, 2), true, true, Decimal::ZERO);
        let (cov_high_funding, _) =
            score_hedge(Decimal::new(98, 2), true, true, Decimal::from(30));
        assert!(cov_high_funding < cov_low_funding);
    }
}
