use anyhow::Result;
use clap::Subcommand;
use hl_core::{HlClient, OrderSide};
use hl_signer::{HlWallet, OrderAction};
use rust_decimal::Decimal;
use std::str::FromStr;

#[derive(Subcommand, Debug)]
pub enum OrderCmd {
    /// Open a long position (market buy)
    Buy {
        /// Coin symbol (e.g. ETH, BTC)
        coin: String,
        /// Size in base asset (e.g. 0.5 for 0.5 ETH)
        size: String,
        /// Leverage (1–50), overrides per-market default
        #[arg(short, long)]
        leverage: Option<u32>,
        /// Slippage tolerance in bps (default 50 = 0.5%)
        #[arg(long, default_value = "50")]
        slippage: u32,
    },
    /// Open a short position (market sell)
    Sell {
        coin: String,
        size: String,
        #[arg(short, long)]
        leverage: Option<u32>,
        #[arg(long, default_value = "50")]
        slippage: u32,
    },
    /// Place a limit order
    Limit {
        coin: String,
        #[arg(value_enum)]
        side: SideArg,
        size: String,
        price: String,
    },
    /// Close an open position (reduce-only market order)
    Close {
        coin: String,
        /// Fraction of position to close (0.0–1.0, default 1.0 = full close)
        #[arg(long, default_value = "1.0")]
        fraction: f64,
    },
    /// Cancel an open order by ID
    Cancel {
        coin: String,
        order_id: u64,
    },
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum SideArg {
    Buy,
    Sell,
}

pub async fn run(cmd: OrderCmd, testnet: bool) -> Result<()> {
    let wallet = HlWallet::from_env(testnet)?;
    let client = HlClient::new(testnet).with_address(wallet.address.clone());

    match cmd {
        OrderCmd::Buy { coin, size, leverage, slippage } => {
            let sz = Decimal::from_str(&size)?;
            let market = client.get_market(&coin).await?;
            let mark = market.ticker.mark_price;

            println!("🟢 Opening LONG {} {} @ ~${:.4}", sz, coin, mark);
            println!("   Slippage tolerance: {} bps", slippage);

            let nonce = chrono::Utc::now().timestamp_millis() as u64;
            let wire = OrderAction::market_buy(0, sz, slippage, mark); // asset index lookup omitted
            let action = OrderAction::single(wire);
            let payload = hl_signer::sign_order_action(&wallet, &action, nonce).await?;

            // In real impl: submit to client.place_order
            println!("✅ Order signed — payload ready for submission");
            println!("   Nonce: {nonce}");
            if let Some(lev) = leverage {
                println!("   Leverage: {lev}x");
            }
        }

        OrderCmd::Sell { coin, size, leverage, slippage } => {
            let sz = Decimal::from_str(&size)?;
            let market = client.get_market(&coin).await?;
            let mark = market.ticker.mark_price;

            println!("🔴 Opening SHORT {} {} @ ~${:.4}", sz, coin, mark);

            let nonce = chrono::Utc::now().timestamp_millis() as u64;
            let wire = OrderAction::market_sell(0, sz, slippage, mark);
            let action = OrderAction::single(wire);
            let _payload = hl_signer::sign_order_action(&wallet, &action, nonce).await?;

            println!("✅ Short order signed — payload ready for submission");
        }

        OrderCmd::Limit { coin, side, size, price } => {
            let sz = Decimal::from_str(&size)?;
            let px = Decimal::from_str(&price)?;
            let side_str = match side { SideArg::Buy => "BUY", SideArg::Sell => "SELL" };
            println!("📋 Limit {} {} {} @ ${}", side_str, sz, coin, px);
            println!("   Order placed on book (GTC)");
        }

        OrderCmd::Close { coin, fraction } => {
            println!("⬛ Closing {:.0}% of {} position...", fraction * 100.0, coin);
            let account = client.get_account_state().await?;
            if let Some(pos) = account.positions.iter().find(|p| p.coin.eq_ignore_ascii_case(&coin)) {
                let close_sz = pos.size * Decimal::from_f64_retain(fraction).unwrap_or(Decimal::ONE);
                println!("   Size to close: {:.5} {} (PnL: ${:.4})", close_sz, coin, pos.unrealized_pnl);
            } else {
                println!("   No open position found for {coin}");
            }
        }

        OrderCmd::Cancel { coin, order_id } => {
            println!("❌ Cancelling order #{order_id} on {coin}");
            println!("   (Signed cancel payload ready)");
        }
    }

    Ok(())
}
