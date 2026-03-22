# Getting Started with IGRA Foundry

## Prerequisites

- **Kaspa wallet** with KAS tokens (for L1 fees and bridging to iKAS)
- **Rust toolchain** (only for source install): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

## Installation

### Option A: Pre-built Binaries

```bash
curl -L https://raw.githubusercontent.com/IgraLabs/igra-foundry-sdk/main/igraup/install | bash
source ~/.zshenv  # or restart terminal
igraup
```

This installs `igra-cast` and `igra-forge` to `~/.igra/bin/`. Your existing Foundry tools are untouched.

### Option B: Build from Source

```bash
git clone https://github.com/IgraLabs/foundry.git igra-foundry
cd igra-foundry
cargo build -p cast -p forge --release
mkdir -p ~/.igra/bin
cp target/release/cast ~/.igra/bin/igra-cast
cp target/release/forge ~/.igra/bin/igra-forge
```

Add `~/.igra/bin` to your PATH if not already there.

## Project Setup

### 1. Create a new project

```bash
igra-forge init my-project
cd my-project
```

### 2. Configure IGRA

Replace the generated `foundry.toml` with IGRA config:

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
private_key = "YOUR_KASPA_PRIVATE_KEY"
```

The private key is a 32-byte hex string (64 characters). The same key derives both your Kaspa L1 address and your EVM L2 address.

### 3. Fund your wallet

You need KAS on L1 for transaction fees. Each transaction costs approximately 0.0022 KAS in L1 fees.

For contract deployment gas on L2, you need iKAS (bridged from KAS). Bridge at least 100 KAS to get iKAS for deployments.

### 4. Check your setup

```bash
# Check L2 balance
cast balance YOUR_EVM_ADDRESS --rpc-url YOUR_IGRA_RPC_URL --ether

# Check chain ID
cast chain-id --rpc-url YOUR_IGRA_RPC_URL

# Check current block
cast block-number --rpc-url YOUR_IGRA_RPC_URL
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
  --legacy \
  --gas-price 1100000000000 \
  --gas-limit 500000
```

You should see output with the deployed contract address within a few seconds.

## Interacting with Contracts

```bash
# Write (sends a transaction via Kaspa L1)
igra-cast send --legacy --gas-price 1100000000000 \
  0xYOUR_CONTRACT "increment()"

# Read (direct RPC call, no L1 needed)
igra-cast call 0xYOUR_CONTRACT "count()(uint256)" \
  --rpc-url YOUR_IGRA_RPC_URL
```

## Using OpenZeppelin

```bash
igra-forge install OpenZeppelin/openzeppelin-contracts
```

Add to `remappings.txt`:
```
@openzeppelin/contracts/=lib/openzeppelin-contracts/contracts/
```

Now you can import OZ contracts in your Solidity code:

```solidity
import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
```

## Next Steps

- See [Configuration](configuration.md) for all available options
- See [Limitations](limitations.md) for important constraints
- Check `examples/deploy-suite/` for 11 ready-to-deploy contracts
