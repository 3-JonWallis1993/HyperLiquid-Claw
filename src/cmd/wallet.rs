use anyhow::Result;
use clap::Subcommand;
use hl_core::HlClient;
use hl_signer::HlWallet;

#[derive(Subcommand, Debug)]
pub enum WalletCmd {
    /// Show address, equity, and margin summary
    Status,
    /// Show recent trade history
    History {
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
}

pub async fn run(cmd: WalletCmd, testnet: bool) -> Result<()> {
    let wallet = HlWallet::from_env(testnet)?;
    let client = HlClient::new(testnet).with_address(wallet.address.clone());

    match cmd {
        WalletCmd::Status => {
            let account = client.get_account_state().await?;
            let net = if testnet { "TESTNET" } else { "MAINNET" };

            println!("\n💼 HyperClaw Wallet — {net}\n");
            println!("  Address:           {}", wallet.address);
            println!("  Account value:     ${:.4}", account.account_value);
            println!("  Equity:            ${:.4}", account.equity);
            println!("  Available margin:  ${:.4}", account.available_margin);
            println!("  Used margin:       ${:.4}", account.used_margin);
            println!("  Total unrealised:  ${:.4}", account.total_unrealized_pnl);
            println!("  Margin ratio:      {:.2}%",
                account.margin_ratio * rust_decimal::Decimal::from(100));
            println!("  Open positions:    {}", account.positions.len());

            if account.positions.is_empty() {
                println!("\n  No open positions. Ready to trade.");
            }
        }

        WalletCmd::History { limit } => {
            println!("\n📜 Trade History (last {limit} fills)\n");
            println!("  [Fetch from Hyperliquid /info → userFills — requires WebSocket auth]");
            println!("  Address: {}", wallet.address);
            println!("\n  Tip: view full history at https://app.hyperliquid.xyz → Portfolio → Activity");
        }
    }

    Ok(())
}
