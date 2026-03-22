# IGRA Foundry Configuration

All IGRA settings are configured in `foundry.toml` under `[profile.default.igra]`.

## Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `enabled` | bool | Enable IGRA transport. Set to `true`. |
| `el_rpc_url` | string | EVM execution layer RPC endpoint |
| `kaspa_rpc_url` | string | Kaspa gRPC endpoint (e.g., `grpc://host:16110`) |
| `kaspa_network` | string | Kaspa network: `"mainnet"` or `"testnet-10"` |
| `tx_id_prefix` | string | Hex prefix for Kaspa TX IDs. Must equal chain ID in hex. |
| `expected_el_chain_id` | u64 | Expected EVM chain ID (validated at startup) |
| `el_receipt_timeout_secs` | u64 | Seconds to wait for L2 receipt after L1 submission |

## Wallet Configuration

Under `[profile.default.igra.kaspa_wallet]`:

| Field | Type | Description |
|-------|------|-------------|
| `private_key` | string | 32-byte hex private key |
| `mnemonic` | string | BIP-39 mnemonic phrase (alternative to private_key) |

CLI overrides: `--private-key-kaspa`, `--mnemonic-kaspa`, `--keystore-kaspa`

If no Kaspa wallet is specified, the EVM private key is used for Kaspa signing as well.

## Activation Methods

IGRA can be activated via (highest priority first):

1. **CLI flag**: `--igra`
2. **Environment variable**: `FOUNDRY_IGRA_ENABLED=true`
3. **Config file**: `igra.enabled = true` in `foundry.toml`

## Network Reference

| Network | Chain ID | TX ID Prefix |
|---------|----------|-------------|
| Mainnet | 38833 | `97b1` |
| Galleon Testnet | 38836 | `97b4` |

See the [IGRA docs](https://igra-labs.gitbook.io/igralabs-docs/quickstart/network-info) for RPC endpoints.

## Example: Full Mainnet Config

```toml
[profile.default]
solc_version = "0.8.20"

[profile.default.igra]
enabled = true
el_rpc_url = "YOUR_IGRA_RPC_URL"
kaspa_rpc_url = "YOUR_KASPA_GRPC_URL"
kaspa_network = "mainnet"
tx_id_prefix = "97b1"
expected_el_chain_id = 38833
el_receipt_timeout_secs = 10

[profile.default.igra.kaspa_wallet]
private_key = "YOUR_KEY_HERE"
```

## TX ID Prefix

The `tx_id_prefix` must equal the chain ID converted to hex. To find it:

```bash
cast chain-id --rpc-url YOUR_IGRA_RPC_URL
# Returns: 38833

python3 -c "print(hex(38833)[2:])"
# Returns: 97b1
```

Using the wrong prefix will cause transactions to be silently dropped by L2.
