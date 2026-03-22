# IGRA Foundry SDK

Build, deploy, and interact with smart contracts on the [IGRA Network](https://igra.sh) using Foundry.

IGRA is an EVM-compatible L2 that uses Kaspa as its L1 for data availability. This SDK provides `igra-cast` and `igra-forge` — drop-in Foundry tools with IGRA support built in. They install alongside your existing Foundry and don't interfere with it.

## Quick Start

### 1. Install

**Pre-built binaries** (recommended):

```bash
curl -L https://raw.githubusercontent.com/IgraLabs/igra-foundry-sdk/main/igraup/install | bash
source ~/.zshenv  # or restart your terminal
igraup
```

This installs `igra-cast` and `igra-forge` to `~/.igra/bin/`. Your existing `cast` and `forge` are untouched.

**From source**:

```bash
git clone https://github.com/IgraLabs/foundry.git igra-foundry
cd igra-foundry
cargo build -p cast -p forge --release
# Copy with igra- prefix to keep separate from standard Foundry
cp target/release/cast ~/.igra/bin/igra-cast
cp target/release/forge ~/.igra/bin/igra-forge
```

### 2. Configure

Create `foundry.toml` in your project root:

```toml
[profile.default]

[profile.default.igra]
enabled = true
el_rpc_url = "YOUR_IGRA_RPC_URL"
kaspa_rpc_url = "YOUR_KASPA_GRPC_URL"
kaspa_network = "mainnet"
tx_id_prefix = "97b1"
expected_el_chain_id = 38833
el_receipt_timeout_secs = 10

[profile.default.igra.kaspa_wallet]
private_key = "YOUR_PRIVATE_KEY_HERE"
```

See `configs/` for mainnet and testnet templates.

### 3. Deploy Your First Contract

```bash
# Initialize a new project (use igra-forge instead of forge)
igra-forge init my-igra-project
cd my-igra-project

# Copy IGRA config
cp /path/to/igra-foundry-sdk/configs/foundry.mainnet.toml foundry.toml
# Edit foundry.toml — set your private key

# Deploy the default Counter contract
igra-forge create src/Counter.sol:Counter \
  --legacy \
  --gas-price 1100000000000 \
  --gas-limit 500000
```

### 4. Send Transactions

```bash
# Write — sends transaction via Kaspa L1
igra-cast send --legacy \
  --gas-price 1100000000000 \
  0xCONTRACT_ADDRESS "increment()"

# Read — direct RPC call, no L1 needed (standard cast works too)
igra-cast call 0xCONTRACT_ADDRESS "count()(uint256)" \
  --rpc-url YOUR_IGRA_RPC_URL

# Check balance
igra-cast balance YOUR_ADDRESS --rpc-url YOUR_IGRA_RPC_URL --ether
```

## How It Works

`igra-cast` and `igra-forge` are full Foundry tools — they do everything standard `cast` and `forge` do. When IGRA is enabled in `foundry.toml`, write operations (`send`, `create`) are automatically routed through Kaspa L1 instead of going directly to an EVM mempool.

Read operations (`call`, `balance`, `block-number`) go straight to the EVM RPC, same as standard Foundry.

```
Standard Foundry:     cast send → EVM mempool → block
IGRA Foundry:         igra-cast send → Kaspa L1 TX → IGRA L2 block
```

You can use `igra-cast` / `igra-forge` for non-IGRA chains too — just don't set `igra.enabled = true` and they behave identically to standard Foundry.

## Key Differences from Standard Foundry

| Setting | IGRA | Standard Ethereum |
|---------|------|-------------------|
| Binary | `igra-cast` / `igra-forge` | `cast` / `forge` |
| TX type | `--legacy` (required) | EIP-1559 default |
| Gas price | 1100 Gwei (1.1e12 wei) | ~1-50 Gwei |
| Gas limit | Set realistic values (1-5M) | 30M is fine |
| Confirmation | < 3 seconds | ~12 seconds |
| TX ordering | FIFO (no MEV) | Gas-price ordering |
| Max TX size | ~21 KB | ~128 KB |
| DA layer | Kaspa L1 | Ethereum L1 |

**Important**: Do NOT set gas limit to very large values (e.g., 30M). IGRA will silently drop transactions with unreasonably large gas limits. Use a realistic estimate (1.5-2x expected usage).

## Network Details

| Network | Chain ID | TX ID Prefix |
|---------|----------|-------------|
| Mainnet | 38833 | `97b1` |
| Galleon Testnet | 38836 | `97b4` |

See the [IGRA docs](https://igra-labs.gitbook.io/igralabs-docs/quickstart/network-info) for RPC endpoints.

## Example Contracts

The `examples/deploy-suite/` directory contains 11 battle-tested contracts deployed on IGRA mainnet:

- **Counter** — minimal state contract (926B, 246K gas)
- **SimpleToken** — basic ERC-20 (5.3KB, 934K gas)
- **OZToken** — OpenZeppelin ERC-20 + Burnable (6.4KB, 1.0M gas)
- **IgraNFT** — OpenZeppelin ERC-721 (8.7KB, 1.8M gas)
- **Vault** — native token deposit/withdraw (1.9KB, 453K gas)
- **MultiSig** — multi-signature wallet (6.1KB, 1.2M gas)
- **Staking** — ERC-20 staking pool (3.4KB, 760K gas)
- **Auction** — English auction (3.5KB, 766K gas)
- **GovToken** — ERC-20 with Votes + Permit (20KB, 3.3M gas)
- **SimpleProxy** — delegatecall proxy (1.3KB, 308K gas)
- **SimpleDEX** — Uniswap-style AMM (8.9KB, 1.8M gas)

See [examples/deploy-suite/](examples/deploy-suite/) for the full source and deploy script.

## Documentation

- [Getting Started](docs/getting-started.md) — detailed setup guide
- [Configuration](docs/configuration.md) — all `foundry.toml` IGRA options
- [Limitations](docs/limitations.md) — gas limits, TX size, ordering behavior
- [IGRA Network Spec](https://igralabs.com/skills/igra-network/igra-network.md) — official network documentation

## Links

- [IGRA Website](https://igra.sh)
- [IGRA Explorer](https://explorer.igra.sh)
- [IGRA Foundry Fork](https://github.com/IgraLabs/foundry)
- [Foundry Book](https://book.getfoundry.sh)
