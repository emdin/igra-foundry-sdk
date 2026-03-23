# IGRA Foundry Limitations & Best Practices

## Transaction Type

Always use the `--legacy` flag:

```bash
igra-cast send --legacy ...
igra-forge create --legacy ...
```

Legacy (type 0), EIP-2930 (type 1), and EIP-1559 (type 2) transactions are all supported. EIP-4844 (blob) and EIP-7702 are **not** supported. Legacy is recommended for simplicity.

## Gas Price

- **Mainnet minimum**: 1000 Gwei (1e12 wei)
- **Recommended**: 1100 Gwei (1.1e12 wei) for safety margin
- **Testnet minimum**: ~2000 Gwei

Always specify gas price explicitly:

```bash
--gas-price 1100000000000
```

Note: IGRA uses FIFO ordering. Increasing gas price does **not** make your transaction faster. There is no MEV.

## Gas Limit

**Critical**: Do NOT set gas limit to very large values (e.g., 30,000,000). IGRA will silently drop transactions with unreasonably large gas limits.

Set gas limit to 1.5-2x the expected gas usage:

| Contract Type | Typical Gas | Recommended Limit |
|--------------|------------|-------------------|
| Simple transfer | 21,000 | 50,000 |
| Counter / minimal | 250,000 | 500,000 |
| ERC-20 | 1,000,000 | 2,000,000 |
| ERC-721 | 1,800,000 | 3,000,000 |
| Complex (Governance) | 3,300,000 | 5,000,000 |
| Large contract (20KB) | 3,300,000 | 5,000,000 |

## Maximum Transaction Size

Maximum L2 data payload is approximately **24 KB** (24,800 bytes), constrained by Kaspa's L1 data availability limit. Large contract deployments may need to use proxy patterns to stay under this limit.

The block gas limit is 10,000,000,000 (10B), so gas is not the bottleneck — payload size is.

**Important**: Larger contracts cost significantly more iKAS to deploy. At 1100 Gwei gas price:

| Contract size | Typical gas | Deploy cost |
|--------------|------------|-------------|
| ~500 B (Counter) | ~158K | ~0.17 iKAS |
| ~3 KB (ERC-20) | ~618K | ~0.68 iKAS |
| ~6 KB (ERC-20 + mint/burn) | ~1.2M | ~1.31 iKAS |

If your deploy is silently dropped, check your L2 iKAS balance first — insufficient balance causes transactions to be dropped with no error message.

## Confirmation Behavior

Transactions are confirmed on L2 within **3 seconds** or not at all. There is no mempool and no backlog.

If a transaction is not found on L2 after 3 seconds, it has been permanently dropped. Common causes:
- Wrong `tx_id_prefix`
- Incorrect nonce (out-of-order submission)
- Gas limit too large
- Insufficient iKAS balance on L2

## Finality

| Use Case | Confirmations | Time |
|----------|--------------|------|
| Low-value transfers | 10 | ~10 seconds |
| DeFi / swaps | 30 | ~30 seconds |
| High-value transfers | 250 | ~4 minutes |
| Exchange deposits | 500 | ~8 minutes |

Formal finality follows Kaspa's protocol (~12 hours).

## Bridging (KAS to iKAS)

To get iKAS (L2 gas token), bridge KAS from L1. Minimum bridge amount is **100 KAS**.

Each L1 transaction costs approximately **0.002 KAS** in Kaspa network fees.

## Nonce Ordering

IGRA processes transactions in strict nonce order. If you send nonce N+1 before nonce N is confirmed, nonce N+1 will be dropped.

For sequential operations, wait for each transaction receipt before sending the next:

```bash
# Good: wait for confirmation
igra-cast send --legacy ... --timeout 10

# Risky: rapid-fire without waiting
for i in {1..10}; do igra-cast send --legacy ... --async; done  # many will be dropped
```

## Sequential Deployments

**Known issue**: Deploying multiple contracts in quick succession often fails after the first one. The second deployment is silently dropped because the Kaspa change UTXO from the first transaction hasn't confirmed yet (~1-2 seconds).

**Workaround**: Wait 3-5 seconds between deployments:

```bash
igra-forge create src/Counter.sol:Counter --legacy --gas-price 1100000000000 --gas-limit 500000 --broadcast ...
sleep 5
igra-forge create src/Vault.sol:Vault --legacy --gas-price 1100000000000 --gas-limit 500000 --broadcast ...
```

This affects any back-to-back write operations from the same wallet. Single transactions and read operations are not affected.

## Reorgs

Reorged transactions are **discarded permanently** and not re-injected into the mempool.
