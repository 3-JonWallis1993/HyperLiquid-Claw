use anyhow::Result;
use clap::Subcommand;
use hl_core::HlClient;
use hl_signer::HlWallet;

#[derive(Subcommand, Debug)]
pub enum PositionsCmd {
    /// List all open positions with live PnL
    List,
    /// Show detailed info for a specific position
    Show {
        /// Coin symbol
        coin: String,
    },
}

pub async fn run(cmd: PositionsCmd, testnet: bool) -> Result<()> {
    let wallet = HlWallet::from_env(testnet)?;
    let client = HlClient::new(testnet).with_address(wallet.address.clone());
    let account = client.get_account_state().await?;

    match cmd {
        PositionsCmd::List => {
            if account.positions.is_empty() {
                println!("\n📭 No open positions.");
                return Ok(());
            }

            println!("\n📊 Open Positions ({})\n", wallet.address);
            println!(
                "{:<8} {:<6} {:>10} {:>12} {:>12} {:>10} {:>10}",
                "COIN", "SIDE", "SIZE", "ENTRY $", "MARK $", "PnL $", "PnL %"
            );
            println!("{}", "─".repeat(72));

            for pos in &account.positions {
                let pnl_symbol = if pos.is_profitable() { "▲" } else { "▼" };
                println!(
                    "{:<8} {:<6} {:>10} {:>12} {:>12} {:>10} {:>10}",
                    pos.coin,
                    pos.side.to_string(),
                    format!("{:.4}", pos.size),
                    format!("${:.4}", pos.entry_price),
                    format!("${:.4}", pos.mark_price),
                    format!("{}{:.4}", pnl_symbol, pos.unrealized_pnl),
                    format!("{:.2}%", pos.pnl_pct()),
                );
            }

            println!("{}", "─".repeat(72));
            println!(
                "\n  Account equity:      ${:.4}",
                account.equity
            );
            println!("  Available margin:    ${:.4}", account.available_margin);
            println!("  Total PnL:           ${:.4}", account.total_unrealized_pnl);
            println!("  Margin ratio:        {:.2}%", account.margin_ratio * rust_decimal::Decimal::from(100));
        }

        PositionsCmd::Show { coin } => {
            let pos = account
                .positions
                .iter()
                .find(|p| p.coin.eq_ignore_ascii_case(&coin));

            match pos {
                None => println!("No open position for {coin}"),
                Some(p) => {
                    println!("\n📈 {} Position Detail\n", p.coin);
                    println!("  Side:              {}", p.side);
                    println!("  Size:              {:.6}", p.size);
                    println!("  Entry price:       ${:.6}", p.entry_price);
                    println!("  Mark price:        ${:.6}", p.mark_price);
                    println!("  Notional:          ${:.2}", p.notional());
                    println!("  Leverage:          {:.1}x", p.leverage);
                    println!("  Margin used:       ${:.4}", p.margin_used);
                    println!("  Unrealised PnL:    ${:.6}", p.unrealized_pnl);
                    println!("  PnL %:             {:.4}%", p.pnl_pct());
                    println!("  Cumulative funding:${:.6}", p.cumulative_funding);
                    if let Some(liq) = p.liquidation_price {
                        println!("  Liq. price:        ${:.6}", liq);
                        if let Some(dist) = p.liquidation_distance_pct() {
                            println!("  Liq. distance:     {:.2}%", dist);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
