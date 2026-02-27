use anyhow::Result;
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use tracing_subscriber::{fmt, EnvFilter};

mod cmd;
use cmd::{hedge, markets, positions, wallet};

/// HyperLiquid-Claw — trading bot CLI for Hyperliquid perpetuals
#[derive(Parser, Debug)]
#[command(
    name = "hyperclaw",
    version = env!("CARGO_PKG_VERSION"),
    about = "Trading-enabled Hyperliquid skill for OpenClaw",
    long_about = "Browse perp markets, execute trades, track positions, and discover hedge opportunities on Hyperliquid L1."
)]
struct Cli {
    /// Use testnet instead of mainnet
    #[arg(long, global = true, env = "HL_TESTNET")]
    testnet: bool,

    /// Verbosity level (repeat for more: -v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Browse and search Hyperliquid markets
    Markets {
        #[command(subcommand)]
        action: markets::MarketsCmd,
    },
    /// Open, close, or manage positions
    #[command(aliases = ["trade"])]
    Order {
        #[command(subcommand)]
        action: cmd::order::OrderCmd,
    },
    /// View open positions and PnL
    Positions {
        #[command(subcommand)]
        action: positions::PositionsCmd,
    },
    /// Wallet balance and transfers
    Wallet {
        #[command(subcommand)]
        action: wallet::WalletCmd,
    },
    /// LLM-powered hedge discovery
    Hedge {
        #[command(subcommand)]
        action: hedge::HedgeCmd,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let cli = Cli::parse();

    // Set up tracing
    let filter = match cli.verbose {
        0 => "hyperclaw=info,hl_core=warn",
        1 => "hyperclaw=debug,hl_core=info",
        2 => "hyperclaw=trace,hl_core=debug",
        _ => "trace",
    };
    fmt()
        .with_env_filter(EnvFilter::new(filter))
        .without_time()
        .with_target(false)
        .init();

    match cli.command {
        Commands::Markets { action } => markets::run(action, cli.testnet).await?,
        Commands::Order { action } => cmd::order::run(action, cli.testnet).await?,
        Commands::Positions { action } => positions::run(action, cli.testnet).await?,
        Commands::Wallet { action } => wallet::run(action, cli.testnet).await?,
        Commands::Hedge { action } => hedge::run(action, cli.testnet).await?,
    }

    Ok(())
}
