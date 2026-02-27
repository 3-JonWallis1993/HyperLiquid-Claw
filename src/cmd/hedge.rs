use anyhow::Result;
use clap::Subcommand;
use hl_core::HlClient;
use hl_risk::coverage::{score_hedge, CoverageTier};
use hl_signer::HlWallet;
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Subcommand, Debug)]
pub enum HedgeCmd {
    /// Scan trending markets for hedge opportunities
    Scan {
        /// Filter by keyword
        #[arg(short, long)]
        query: Option<String>,
        /// Number of market pairs to evaluate
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Minimum coverage tier to display (low/moderate/good/high)
        #[arg(long, default_value = "moderate")]
        min_tier: String,
    },
    /// Analyse a specific pair of coins
    Analyze {
        coin1: String,
        coin2: String,
    },
}

pub async fn run(cmd: HedgeCmd, testnet: bool) -> Result<()> {
    let wallet = HlWallet::from_env(testnet)?;
    let client = HlClient::new(testnet).with_address(wallet.address.clone());

    match cmd {
        HedgeCmd::Scan { query, limit, min_tier } => {
            println!("\n🔍 Scanning hedge opportunities on Hyperliquid...\n");

            let markets = client.get_markets().await?;
            let mut filtered: Vec<_> = markets
                .into_iter()
                .filter(|m| {
                    if let Some(q) = &query {
                        m.info.coin.to_lowercase().contains(&q.to_lowercase())
                    } else {
                        true
                    }
                })
                .take(limit)
                .collect();

            filtered.sort_by(|a, b| b.ticker.volume_24h.cmp(&a.ticker.volume_24h));

            let min = match min_tier.as_str() {
                "high" => CoverageTier::High,
                "good" => CoverageTier::Good,
                "low" => CoverageTier::Low,
                _ => CoverageTier::Moderate,
            };

            println!(
                "{:<6} {:<6} {:>10} {:>10} {:>8}",
                "TARGET", "COVER", "COV %", "CORR", "TIER"
            );
            println!("{}", "─".repeat(45));

            let mut found = 0;
            for i in 0..filtered.len() {
                for j in (i + 1)..filtered.len() {
                    let a = &filtered[i];
                    let b = &filtered[j];

                    // Stub correlation: in production, compute from 7d price history
                    let corr = estimate_correlation(&a.info.coin, &b.info.coin);
                    let combined_funding = a.ticker.funding_rate.abs() + b.ticker.funding_rate.abs();
                    let combined_funding_apr = combined_funding * Decimal::from(3 * 365 * 100);

                    let (cov, tier) = score_hedge(corr, true, true, combined_funding_apr);
                    if tier < min {
                        continue;
                    }

                    println!(
                        "{:<6} {:<6} {:>10} {:>10} {:>8}",
                        a.info.coin,
                        b.info.coin,
                        format!("{:.1}%", cov),
                        format!("{:.3}", corr),
                        tier.label(),
                    );
                    found += 1;
                }
            }

            if found == 0 {
                println!("  No pairs found above {} tier.", min_tier);
            }
            println!("\n  Scanned {} pairs.", filtered.len() * (filtered.len() - 1) / 2);
        }

        HedgeCmd::Analyze { coin1, coin2 } => {
            println!("\n🧮 Analysing hedge pair: {} ↔ {}\n", coin1, coin2);

            let (m1, m2) = tokio::try_join!(
                client.get_market(&coin1),
                client.get_market(&coin2),
            )?;

            let corr = estimate_correlation(&coin1, &coin2);
            let combined_funding = (m1.ticker.funding_rate + m2.ticker.funding_rate).abs();
            let combined_apr = combined_funding * Decimal::from(3 * 365 * 100);
            let (cov, tier) = score_hedge(corr, true, true, combined_apr);

            println!("  {} mark:          ${:.4}", coin1, m1.ticker.mark_price);
            println!("  {} mark:          ${:.4}", coin2, m2.ticker.mark_price);
            println!("  Correlation:        {:.4}", corr);
            println!("  Coverage:           {:.2}%", cov);
            println!("  Tier:               {}", tier.label());
            println!("  Combined fund APR:  {:.4}%", combined_apr);
            println!("\n  {} To hedge 1 {} long:", tier.emoji(), coin1);
            let ratio = m1.ticker.mark_price / m2.ticker.mark_price;
            println!("  Short {:.6} {} (ratio {:.4})", ratio, coin2, ratio);
        }
    }

    Ok(())
}

/// Stub correlation estimator — in production fetch 7d hourly OHLC and
/// compute Pearson correlation of log returns.
fn estimate_correlation(coin1: &str, coin2: &str) -> Decimal {
    // BTC/ETH are highly correlated
    let btc_eth = ["BTC", "ETH"];
    let a = coin1.to_uppercase();
    let b = coin2.to_uppercase();

    if btc_eth.contains(&a.as_str()) && btc_eth.contains(&b.as_str()) {
        return Decimal::from_str("0.92").unwrap();
    }
    // Most large-cap alts correlate ~0.75 with BTC
    if btc_eth.contains(&a.as_str()) || btc_eth.contains(&b.as_str()) {
        return Decimal::from_str("0.75").unwrap();
    }
    // Stablecoin pairs have near-zero correlation
    let stables = ["USDC", "USDT", "DAI"];
    if stables.contains(&a.as_str()) || stables.contains(&b.as_str()) {
        return Decimal::from_str("0.05").unwrap();
    }
    // Default mid-correlation for unknown pairs
    Decimal::from_str("0.65").unwrap()
}
