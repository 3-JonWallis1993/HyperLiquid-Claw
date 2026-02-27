use anyhow::Result;
use clap::Subcommand;
use hl_core::HlClient;

#[derive(Subcommand, Debug)]
pub enum MarketsCmd {
    /// Show top markets by 24h volume
    Trending {
        /// Number of markets to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Search markets by coin name
    Search {
        /// Search query (e.g. "BTC", "ETH", "SOL")
        query: String,
    },
    /// Show detailed info for a specific market
    Info {
        /// Coin symbol (e.g. BTC, ETH)
        coin: String,
    },
}

pub async fn run(cmd: MarketsCmd, testnet: bool) -> Result<()> {
    let client = HlClient::new(testnet);

    match cmd {
        MarketsCmd::Trending { limit } => {
            println!("\n📊 Trending Markets on Hyperliquid\n");
            println!(
                "{:<8} {:>14} {:>14} {:>12} {:>14}",
                "COIN", "MARK PRICE", "24H VOLUME", "FUNDING%", "OI"
            );
            println!("{}", "─".repeat(65));

            let mut markets = client.get_markets().await?;
            markets.sort_by(|a, b| b.ticker.volume_24h.cmp(&a.ticker.volume_24h));

            for m in markets.into_iter().take(limit) {
                println!(
                    "{:<8} {:>14} {:>14} {:>12} {:>14}",
                    m.info.coin,
                    format!("${:.2}", m.ticker.mark_price),
                    format!("${:.0}", m.ticker.volume_24h),
                    format!("{:.4}%", m.ticker.funding_rate * rust_decimal::Decimal::from(100)),
                    format!("${:.0}", m.ticker.open_interest),
                );
            }
        }

        MarketsCmd::Search { query } => {
            let markets = client.get_markets().await?;
            let q = query.to_lowercase();
            let results: Vec<_> = markets
                .into_iter()
                .filter(|m| m.info.coin.to_lowercase().contains(&q))
                .collect();

            if results.is_empty() {
                println!("No markets found matching '{query}'");
            } else {
                println!("\n🔍 Markets matching '{query}':\n");
                for m in results {
                    println!(
                        "  {} — ${:.4} | funding: {:.4}% | max lev: {}x",
                        m.info.coin,
                        m.ticker.mark_price,
                        m.ticker.funding_rate * rust_decimal::Decimal::from(100),
                        m.info.max_leverage,
                    );
                }
            }
        }

        MarketsCmd::Info { coin } => {
            let market = client.get_market(&coin).await?;
            println!("\n📈 {} Market Details\n", market.info.coin);
            println!("  Mark price:      ${:.6}", market.ticker.mark_price);
            println!("  Oracle price:    ${:.6}", market.ticker.oracle_price);
            println!("  Funding rate:    {:.6}% (8h)", market.ticker.funding_rate * rust_decimal::Decimal::from(100));
            println!("  Funding APR:     {:.2}%", market.funding_apr());
            println!("  Open interest:   ${:.2}", market.ticker.open_interest);
            println!("  24h volume:      ${:.2}", market.ticker.volume_24h);
            println!("  Max leverage:    {}x", market.info.max_leverage);
            println!("  Min size:        {}", market.info.min_sz);
            println!("  Spread (bps):    {:.2}", market.spread_bps());
        }
    }

    Ok(())
}
