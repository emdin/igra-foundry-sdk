//! Kaspa payload submission: trait abstraction and in-process gRPC implementation.

use crate::config::IgraKaspaWalletConfig;
use crate::keys::{
    kaspa_address_from_private_key, kaspa_network_descriptor, resolve_kaspa_private_key,
};
use crate::payload::{
    build_igra_l2data, mine_and_build_signed_payload_transaction, normalize_hex_prefix,
    IGRA_L2DATA_TOO_LARGE_ERROR_CODE, IGRA_MAX_L2DATA_BYTES,
};
use alloy_primitives::hex;
use async_trait::async_trait;
use kaspa_addresses::Address as KaspaAddress;
use kaspa_grpc_client::GrpcClient;
use kaspa_rpc_core::{RpcTransaction, RpcUtxosByAddressesEntry, api::rpc::RpcApi};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Mutex as TokioMutex;
use tracing::info;

/// Error returned when unsupported send methods are used in IGRA mode.
pub const IGRA_SEND_TRANSACTION_UNSUPPORTED_ERROR: &str =
    "IGRA mode requires raw signed transactions; eth_sendTransaction* is not supported";
/// Error returned when unsupported signed transaction envelopes are used in IGRA mode.
pub const IGRA_EIP4844_UNSUPPORTED_ERROR: &str =
    "IGRA unsupported transaction type: EIP-4844 (blob transactions)";
/// Error returned when unsupported EIP-7702 envelopes are used in IGRA mode.
pub const IGRA_EIP7702_UNSUPPORTED_ERROR: &str = "IGRA unsupported transaction type: EIP-7702";

const CACHE_TTL_SECS: u64 = 20;

/// Payload submission request forwarded to a Kaspa submitter implementation.
#[derive(Clone, Debug)]
pub struct IgraSubmitRequest {
    pub l2_tx_hash: String,
    pub raw_tx_bytes: Vec<u8>,
    pub tx_id_prefix: String,
    pub mining_timeout_secs: u64,
    pub kaspa_rpc_url: Option<String>,
    pub kaspa_network: Option<String>,
    pub payload_compression: Option<String>,
    pub kaspa_wallet: IgraKaspaWalletConfig,
}

/// Result returned by a Kaspa submitter implementation.
#[derive(Clone, Debug)]
pub struct IgraSubmitResult {
    pub kaspa_tx_id: String,
    pub payload_nonce: u64,
}

/// Abstraction over Kaspa submission to keep IGRA transport testable.
#[async_trait]
pub trait IgraPayloadSubmitter: Send + Sync + std::fmt::Debug {
    async fn submit_payload(&self, request: &IgraSubmitRequest)
        -> Result<IgraSubmitResult, String>;
}

#[derive(Clone)]
struct CachedUtxoSet {
    cache_key: String,
    fetched_at: Instant,
    entries: Vec<RpcUtxosByAddressesEntry>,
}

/// Default submitter implementation backed by in-process Kaspa RPC, signing, and broadcast.
#[derive(Clone)]
pub struct InProcessKaspaPayloadSubmitter {
    utxo_cache: Arc<TokioMutex<Option<CachedUtxoSet>>>,
    utxo_cache_epoch: Arc<AtomicU64>,
}

impl std::fmt::Debug for InProcessKaspaPayloadSubmitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InProcessKaspaPayloadSubmitter").finish()
    }
}

impl Default for InProcessKaspaPayloadSubmitter {
    fn default() -> Self {
        Self {
            utxo_cache: Arc::new(TokioMutex::new(None)),
            utxo_cache_epoch: Arc::new(AtomicU64::new(1)),
        }
    }
}

#[async_trait]
impl IgraPayloadSubmitter for InProcessKaspaPayloadSubmitter {
    async fn submit_payload(
        &self,
        request: &IgraSubmitRequest,
    ) -> Result<IgraSubmitResult, String> {
        let (payload_header, l2data) = build_igra_l2data(
            &request.raw_tx_bytes,
            request.payload_compression.as_deref(),
        )?;
        if l2data.len() > IGRA_MAX_L2DATA_BYTES {
            return Err(format!(
                "{IGRA_L2DATA_TOO_LARGE_ERROR_CODE}: L2Data size {} bytes exceeds max {} bytes",
                l2data.len(),
                IGRA_MAX_L2DATA_BYTES
            ));
        }

        let rpc_url = request
            .kaspa_rpc_url
            .as_deref()
            .ok_or_else(|| "IGRA config error: `kaspa_rpc_url` is required".to_string())?;
        let network = request
            .kaspa_network
            .as_deref()
            .ok_or_else(|| "IGRA config error: `kaspa_network` is required".to_string())?;
        let (network_type, address_prefix) = kaspa_network_descriptor(network)?;
        let private_key = resolve_kaspa_private_key(&request.kaspa_wallet, None)?;
        let source_address = kaspa_address_from_private_key(&private_key, address_prefix)?;
        info!(
            "IGRA submit: kaspa_source_address={} kaspa_network={} kaspa_rpc_url={} l2_tx_hash={}",
            source_address, network, rpc_url, request.l2_tx_hash
        );
        let mut client = GrpcClient::connect(rpc_url.to_string())
            .await
            .map_err(|err| format!("IGRA submit error: failed to connect to Kaspa RPC: {err}"))?;

        let prefix = normalize_hex_prefix(request.tx_id_prefix.clone());
        if prefix.is_empty() {
            return Err("IGRA config error: `tx_id_prefix` cannot be empty".to_string());
        }
        let prefix_bytes = hex::decode(prefix.clone())
            .map_err(|err| format!("IGRA config error: `tx_id_prefix` is invalid hex: {err}"))?;
        let mining_timeout = Duration::from_secs(request.mining_timeout_secs);

        for attempt in 0..=1 {
            let force_refresh = attempt > 0;
            let utxos = self
                .load_utxos(
                    &mut client,
                    rpc_url,
                    network,
                    &source_address,
                    force_refresh,
                )
                .await?;

            // Allow one forced refresh pass in case the cache is stale or the node just finished
            // syncing.
            if utxos.is_empty() {
                if !force_refresh {
                    continue;
                }

                let mut message = format!(
                    "IGRA submit error: insufficient Kaspa UTXOs for fee payment (source address: {source_address})"
                );
                if request.kaspa_wallet.mnemonic.is_some()
                    && request.kaspa_wallet.mnemonic_passphrase.is_none()
                {
                    message.push_str(
                        "; hint: if this mnemonic was created/imported with a non-empty BIP39 passphrase, set --mnemonic-passphrase-kaspa (or KASPA_MNEMONIC_PASSPHRASE) to match the funded address",
                    );
                }
                return Err(message);
            }

            let private_key_for_build = private_key;
            let source_address_for_build = source_address.clone();
            let l2data_for_build = l2data.clone();
            let prefix_for_build = prefix_bytes.clone();
            let mining_timeout_for_build = mining_timeout;
            let (payload_nonce, transaction) = tokio::task::spawn_blocking(move || {
                mine_and_build_signed_payload_transaction(
                    &private_key_for_build,
                    &source_address_for_build,
                    network_type,
                    payload_header,
                    &l2data_for_build,
                    &prefix_for_build,
                    mining_timeout_for_build,
                    &utxos,
                )
            })
            .await
            .map_err(|err| {
                format!("IGRA submit error: failed to join Kaspa tx builder task: {err}")
            })??;
            // Invalidate local UTXO cache before broadcast so concurrent reads cannot reuse
            // potentially spent entries from this submission attempt.
            self.invalidate_utxo_cache().await;
            let rpc_transaction = RpcTransaction::from(&transaction);
            match client.submit_transaction(rpc_transaction, false).await {
                Ok(tx_id) => {
                    self.invalidate_utxo_cache().await;
                    let kaspa_tx_id = tx_id.to_string();
                    info!(
                        "IGRA submit: kaspa_tx_id={} payload_nonce={} payload_header=0x{:02x} l2data_len={} payload_compression={} l2_tx_hash={}",
                        kaspa_tx_id,
                        payload_nonce,
                        payload_header,
                        l2data.len(),
                        request
                            .payload_compression
                            .as_deref()
                            .unwrap_or("none")
                            .trim(),
                        request.l2_tx_hash
                    );
                    return Ok(IgraSubmitResult { kaspa_tx_id, payload_nonce });
                }
                Err(err) => {
                    if attempt == 1 {
                        return Err(format!("IGRA submit error: {err}"));
                    }
                }
            }
        }

        Err("IGRA submit error: failed to submit Kaspa transaction".to_string())
    }
}

impl InProcessKaspaPayloadSubmitter {
    async fn invalidate_utxo_cache(&self) {
        self.utxo_cache_epoch.fetch_add(1, Ordering::Relaxed);
        let mut cache_guard = self.utxo_cache.lock().await;
        *cache_guard = None;
    }

    async fn load_utxos(
        &self,
        client: &mut GrpcClient,
        rpc_url: &str,
        network: &str,
        source_address: &KaspaAddress,
        force_refresh: bool,
    ) -> Result<Vec<RpcUtxosByAddressesEntry>, String> {
        let cache_key = format!("{rpc_url}|{network}|{source_address}");
        let read_epoch = self.utxo_cache_epoch.load(Ordering::Relaxed);
        if !force_refresh {
            let cache_guard = self.utxo_cache.lock().await;
            if let Some(cache) = cache_guard.as_ref() {
                if cache.cache_key == cache_key
                    && cache.fetched_at.elapsed() <= Duration::from_secs(CACHE_TTL_SECS)
                {
                    return Ok(cache.entries.clone());
                }
            }
        }

        let entries = client
            .get_utxos_by_addresses(vec![source_address.clone()])
            .await
            .map_err(|err| format!("IGRA submit error: failed to load Kaspa UTXOs: {err}"))?;

        let mut cache_guard = self.utxo_cache.lock().await;
        let current_epoch = self.utxo_cache_epoch.load(Ordering::Relaxed);
        if force_refresh || current_epoch == read_epoch {
            *cache_guard = Some(CachedUtxoSet {
                cache_key,
                fetched_at: Instant::now(),
                entries: entries.clone(),
            });
        }
        Ok(entries)
    }
}
