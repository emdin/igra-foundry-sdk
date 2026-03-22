# Getting Started with IGRA Foundry

## Prerequisites

- **Kaspa wallet** with KAS tokens (for L1 DA fees)
- **iKAS on L2** for EVM gas — get from the [faucet](https://faucet.igralabs.com) or bridge KAS
- **Rust toolchain** (only for source install): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

## Installation

### Option A: Pre-built Binaries

```bash
curl -L https://raw.githubusercontent.com/emdin/igra-foundry-sdk/main/igraup/install | bash
source ~/.zshenv  # or restart terminal
igraup
```

This installs `igra-cast` and `igra-forge` to `~/.igra/bin/`. Your existing Foundry tools are untouched.

> **If `igraup` reports no releases**, pre-built binaries haven't been published yet. Use Option B.

### Option B: Build from Source

The IGRA Foundry fork requires three repos as siblings:

```bash
mkdir igra-build && cd igra-build

# 1. Clone the Kaspa node libraries
git clone https://github.com/IgraLabs/rusty-kaspa.git

# 2. Clone the Kaspa wallet (build dependency)
git clone https://github.com/IgraLabs/kaswallet.git

# 3. Clone the Foundry fork (IGRA branch)
git clone -b roman/kaspa-igra-support https://github.com/IgraLabs/foundry.git

# 4. Build
cd foundry
cargo build -p cast -p forge --release

# 5. Install
mkdir -p ~/.igra/bin
cp target/release/cast ~/.igra/bin/igra-cast
cp target/release/forge ~/.igra/bin/igra-forge
```

Add `~/.igra/bin` to your PATH if not already there:
```bash
echo 'export PATH="$HOME/.igra/bin:$PATH"' >> ~/.zshenv
source ~/.zshenv
```

## Project Setup

### 1. Create a new project

```bash
igra-forge init my-project
cd my-project
```

### 2. Configure IGRA

Replace the generated `foundry.toml`:

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

The private key is a 32-byte hex string (64 characters). The same key derives both your Kaspa L1 address and your EVM L2 address.

### 3. Fund your wallet

Sending transactions on IGRA requires tokens on **two layers**:

| Layer | Token | What it pays for | How to get it |
|-------|-------|-----------------|---------------|
| Kaspa L1 | **KAS** | Data availability fee (~0.002 KAS per tx) | Buy or receive KAS |
| IGRA L2 | **iKAS** | EVM gas + transfer value | [Faucet](https://faucet.igralabs.com) or bridge KAS → iKAS |

The faucet gives you iKAS on L2 only. That covers L2 gas, but every write transaction (`send`, `create`) also submits a Kaspa L1 transaction, which requires KAS on L1.

**Finding your Kaspa L1 address**: try sending a transaction — if you don't have L1 KAS, the error will show it:

```
IGRA submit error: insufficient Kaspa UTXOs for fee payment
(source address: kaspa:qq...)
```

Fund that `kaspa:qq...` address with a small amount of KAS (0.1 KAS is enough for ~45 transactions).

### 4. Check your setup

```bash
# Check L2 balance
igra-cast balance YOUR_EVM_ADDRESS --rpc-url https://rpc.igralabs.com:8545 --ether

# Check chain ID (should return 38833)
igra-cast chain-id --rpc-url https://rpc.igralabs.com:8545

# Check current block
igra-cast block-number --rpc-url https://rpc.igralabs.com:8545
```

## Your First Deployment

Edit `src/Counter.sol`:

```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract Counter {
    uint256 public count;

    function increment() external {
        count += 1;
    }
}
```

Deploy:

```bash
igra-forge create src/Counter.sol:Counter \
  --rpc-url https://rpc.igralabs.com:8545 \
  --private-key YOUR_PRIVATE_KEY \
  --legacy --gas-price 1100000000000 --gas-limit 500000 \
  --broadcast
```

You should see output with the deployed contract address within ~3 seconds. `--broadcast` is required — without it, Foundry does a dry run.

## Interacting with Contracts

```bash
# Write (sends a transaction via Kaspa L1)
igra-cast send \
  --rpc-url https://rpc.igralabs.com:8545 \
  --private-key YOUR_PRIVATE_KEY \
  --legacy --gas-price 1100000000000 \
  0xYOUR_CONTRACT "increment()"

# Read (direct RPC call, no L1 needed)
igra-cast call 0xYOUR_CONTRACT "count()(uint256)" \
  --rpc-url https://rpc.igralabs.com:8545
```

> **Always use `--legacy`.** Without it, Foundry defaults to EIP-1559 which may trigger a confusing "Insufficient gas fee" error.

## Using OpenZeppelin

```bash
igra-forge install OpenZeppelin/openzeppelin-contracts
```

Add to `remappings.txt`:
```
@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/
```

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `Insufficient gas fee` | Missing `--legacy` flag; EIP-1559 splits gas price | Add `--legacy` to all write commands |
| `Error accessing local wallet` | No `--private-key` on CLI | Add `--private-key YOUR_KEY` (or apply [SDK patches](../patches/) for config fallback) |
| `localhost:8545` connection refused | No `--rpc-url` on CLI | Add `--rpc-url https://rpc.igralabs.com:8545` (or apply [SDK patches](../patches/) for config fallback) |
| `insufficient Kaspa UTXOs` | No KAS on L1 for DA fees | Fund your Kaspa address (shown in error) with KAS |
| Transaction hangs then times out | Gas limit too high (e.g., 30M), wrong `tx_id_prefix`, or nonce gap | Use realistic gas limit (500K-5M), verify prefix matches chain ID |
| Dry run instead of actual deploy | Missing `--broadcast` on `forge create` | Add `--broadcast` to deploy commands |

## Next Steps

- See [Configuration](configuration.md) for all available options
- See [Limitations](limitations.md) for important constraints
- Check `examples/deploy-suite/` for 11 ready-to-deploy contracts
