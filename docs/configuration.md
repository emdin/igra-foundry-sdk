# IGRA Foundry Configuration

All IGRA settings are configured in `foundry.toml` under `[profile.default.igra]`.

## Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | bool | Enable IGRA transport. Set to `true`. |
| `el_rpc_url` | string | EVM execution layer RPC endpoint |
| `kaspa_rpc_url` | string | Kaspa gRPC endpoint (e.g., `grpc://host:16110`) |
| `kaspa_network` | string | Kaspa network: `"mainnet"` or `"testnet-10"` |
| `tx_id_prefix` | string | Hex prefix for Kaspa TX IDs (chain ID in hex) |
| `expected_el_chain_id` | u64 | Expected EVM chain ID (validated at startup) |
| `el_receipt_timeout_secs` | u64 | Seconds to wait for L2 receipt after L1 submission |

## Wallet Configuration

Under `[profile.default.igra.kaspa_wallet]`:

| Field | Type | Description |
|-------|------|-------------|
| `private_key` | string | 32-byte hex private key (64 chars, with or without `0x` prefix) |
| `mnemonic` | string | BIP-39 mnemonic phrase (alternative to private_key) |

CLI overrides: `--private-key-kaspa`, `--mnemonic-kaspa`, `--keystore-kaspa`

If no Kaspa wallet is specified, the EVM `--private-key` is used for Kaspa signing as well.

## Activation Methods

IGRA can be activated via (highest priority first):

1. **CLI flag**: `--igra`
2. **Environment variable**: `FOUNDRY_IGRA_ENABLED=true`
3. **Config file**: `igra.enabled = true` in `foundry.toml`

## Network Reference

| Network | Chain ID | TX ID Prefix | RPC URL |
|---------|----------|-------------|---------|
| Mainnet | 38833 | `97b1` | `https://rpc.igralabs.com:8545` |
| Galleon Testnet | 38836 | `97b4` | `https://galleon-testnet.igralabs.com:8545` |

Kaspa gRPC: `grpc://95.217.73.85:16110`

## Example: Full Mainnet Config

```toml
[profile.default]
solc_version = "0.8.20"

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

## TX ID Prefix

The `tx_id_prefix` is the chain ID converted to hex:

```bash
python3 -c "print(hex(38833)[2:])"
# Returns: 97b1
```

Using the wrong prefix will cause transactions to be silently dropped by L2.
