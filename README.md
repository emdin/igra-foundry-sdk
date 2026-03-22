# IGRA Foundry SDK

Deploy smart contracts on [IGRA Network](https://igra.sh) in under 60 seconds.

IGRA is an EVM L2 on Kaspa. This SDK gives you `igra-cast` and `igra-forge` — Foundry with IGRA support built in. They install alongside your existing Foundry and don't touch it.

## Quick Start

### 1. Install

```bash
curl -L https://raw.githubusercontent.com/IgraLabs/igra-foundry-sdk/main/igraup/install | bash
source ~/.zshenv  # or restart your terminal
igraup
```

This downloads pre-built `igra-cast` and `igra-forge` to `~/.igra/bin/`. Your existing Foundry is untouched.

> **Build from source?** See [Getting Started](docs/getting-started.md#build-from-source) for the full build instructions (requires Rust + 3 repos).

### 2. Init + Configure

```bash
igra-forge init my-project && cd my-project
```

Replace `foundry.toml` with this (fill in your private key):

```toml
[profile.default]

[profile.default.igra]
enabled = true
el_rpc_url = "https://rpc.igralabs.com:8545"
kaspa_rpc_url = "grpc://95.217.73.85:16110"
kaspa_network = "mainnet"
tx_id_prefix = "97b1"
expected_el_chain_id = 38833
el_receipt_timeout_secs = 10

[profile.default.igra.kaspa_wallet]
private_key = "YOUR_64_CHAR_HEX_PRIVATE_KEY"
```

The same private key derives both your Kaspa L1 address and your EVM L2 address.

### 3. Fund your wallet

You need tokens on **two layers**:

| Layer | Token | What it pays for | How to get it |
|-------|-------|-----------------|---------------|
| IGRA L2 | **iKAS** | EVM gas | [Faucet](https://faucet.igralabs.com) (testnet) or bridge KAS |
| Kaspa L1 | **KAS** | DA fee (~0.002 KAS/tx) | Buy KAS, send to your Kaspa address |

Don't know your Kaspa address? Try sending a tx — the error message will show it:
```
IGRA submit error: insufficient Kaspa UTXOs (source address: kaspa:qq...)
```

### 4. Deploy

```bash
igra-forge create src/Counter.sol:Counter \
  --rpc-url https://rpc.igralabs.com:8545 \
  --private-key YOUR_64_CHAR_HEX_PRIVATE_KEY \
  --legacy --gas-price 1100000000000 --gas-limit 500000 \
  --broadcast
```

Should confirm in ~3 seconds. `--broadcast` is required (without it, Foundry does a dry run).

### 5. Interact

```bash
# Write (routed through Kaspa L1)
igra-cast send \
  --rpc-url https://rpc.igralabs.com:8545 \
  --private-key YOUR_64_CHAR_HEX_PRIVATE_KEY \
  --legacy --gas-price 1100000000000 \
  0xYOUR_CONTRACT "increment()"

# Read (direct RPC, no L1 needed)
igra-cast call 0xYOUR_CONTRACT "number()(uint256)" \
  --rpc-url https://rpc.igralabs.com:8545
```

> **Always use `--legacy`.** Without it, Foundry defaults to EIP-1559 which splits the gas price and may trigger "Insufficient gas fee".

## How It Works

```
Standard Foundry:   cast send  →  EVM mempool  →  block
IGRA Foundry:       igra-cast send  →  Kaspa L1 TX  →  IGRA L2 block (~3s)
```

When `igra.enabled = true` in `foundry.toml`, write operations (`send`, `create`) are routed through Kaspa L1. Reads go straight to the EVM RPC. When IGRA is disabled, the tools behave identically to standard Foundry.

## Key Rules

| | IGRA | Standard Ethereum |
|--|------|-------------------|
| TX type | `--legacy` required | EIP-1559 default |
| Gas price | 1100 Gwei (`1100000000000`) | ~1-50 Gwei |
| Gas limit | Realistic (1-5M) | 30M is fine |
| Confirmation | ~3 seconds | ~12 seconds |
| Ordering | FIFO (no MEV) | Gas-price auction |
| Max TX size | ~24 KB | ~128 KB |

**Do NOT set gas limit to 30M.** IGRA silently drops transactions with unreasonably large gas limits. Use 1.5-2x expected usage.

## Network Reference

| Network | Chain ID | TX ID Prefix | RPC URL | Explorer |
|---------|----------|-------------|---------|----------|
| Mainnet | 38833 | `97b1` | `https://rpc.igralabs.com:8545` | [explorer.igralabs.com](https://explorer.igralabs.com) |
| Galleon Testnet | 38836 | `97b4` | `https://galleon-testnet.igralabs.com:8545` | [explorer.galleon-testnet.igralabs.com](https://explorer.galleon-testnet.igralabs.com) |

Faucet: [faucet.igralabs.com](https://faucet.igralabs.com)

## Example Contracts

The `examples/deploy-suite/` directory has 11 contracts deployed on IGRA mainnet — from a 926-byte Counter to a 20KB Governance token. See [examples/deploy-suite/](examples/deploy-suite/).

## Docs

- [Getting Started](docs/getting-started.md) — detailed setup, funding, troubleshooting
- [Configuration](docs/configuration.md) — all `foundry.toml` options
- [Limitations](docs/limitations.md) — gas limits, TX size, nonce ordering

## Links

- [IGRA Website](https://igra.sh) / [Explorer](https://explorer.igralabs.com) / [Faucet](https://faucet.igralabs.com)
- [IGRA Foundry Fork](https://github.com/IgraLabs/foundry) (branch: `roman/kaspa-igra-support`)
- [IGRA Docs](https://igra-labs.gitbook.io/igralabs-docs)
- [Foundry Book](https://book.getfoundry.sh)
