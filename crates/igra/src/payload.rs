//! IGRA payload building, nonce mining, fee estimation, and Kaspa TX construction.

use kaspa_addresses::Address as KaspaAddress;
use kaspa_consensus_core::{
    config::params::Params as KaspaParams,
    mass::MassCalculator as KaspaMassCalculator,
    network::NetworkType as KaspaNetworkType,
    sign::{sign_with_multiple_v2 as kaspa_sign_with_multiple_v2, verify as kaspa_verify},
    subnets::SubnetworkId,
    tx::{
        SignableTransaction as KaspaSignableTransaction, Transaction as KaspaTransaction,
        TransactionInput as KaspaTransactionInput, TransactionOutput as KaspaTransactionOutput,
        UtxoEntry as KaspaUtxoEntry,
    },
};
use kaspa_rpc_core::RpcUtxosByAddressesEntry;
use kaspa_txscript::pay_to_address_script;
use std::time::{Duration, Instant};

/// Default mining timeout in seconds.
pub const DEFAULT_MINING_TIMEOUT_SECS: u64 = 120;
/// Maximum standard Kaspa transaction mass.
pub const MAX_STANDARD_KASPA_TX_MASS: u64 = 100_000;
/// Maximum L2Data payload size in bytes.
pub const IGRA_MAX_L2DATA_BYTES: usize = 24_800;
/// Base submission fee in sompi.
pub const BASE_SUBMIT_FEE_SOMPI: u64 = 200_000;
/// Fee per KiB of payload in sompi.
pub const FEE_PER_KIB_SOMPI: u64 = 20_000;
/// Additional fee per extra UTXO input in sompi.
pub const EXTRA_INPUT_FEE_SOMPI: u64 = 10_000;
/// Minimum change output value in sompi.
pub const MIN_CHANGE_SOMPI: u64 = 1_000;
/// Error code returned when payload prefix mining times out.
pub const IGRA_MINING_TIMEOUT_ERROR_CODE: &str = "IGRA_MINING_001";
/// Error code returned when the embedded L2 tx exceeds IGRA payload size limits.
pub const IGRA_L2DATA_TOO_LARGE_ERROR_CODE: &str = "IGRA_PAYLOAD_001";

/// Strips `0x` prefix, lowercases, and trims whitespace from a hex prefix string.
pub fn normalize_hex_prefix(prefix: String) -> String {
    prefix.trim().trim_start_matches("0x").to_ascii_lowercase()
}

/// Estimates the Kaspa fee in sompi for a given payload size and number of UTXO inputs.
pub fn estimated_fee_sompi(payload_len: usize, inputs: usize) -> u64 {
    let payload_len = u64::try_from(payload_len).unwrap_or(u64::MAX);
    let kib = payload_len.div_ceil(1024);
    let input_tail = u64::try_from(inputs.saturating_sub(1)).unwrap_or(u64::MAX);
    BASE_SUBMIT_FEE_SOMPI
        .saturating_add(kib.saturating_mul(FEE_PER_KIB_SOMPI))
        .saturating_add(input_tail.saturating_mul(EXTRA_INPUT_FEE_SOMPI))
}

/// Build an IGRA payload for embedding into a Kaspa L1 TX.
///
/// Format: `[1-byte header] [L2Data bytes] [4-byte big-endian nonce]`
///
/// Header: `(version << 4) | txTypeId` where version=0x9, txTypeId=0x4 (raw uncompressed).
pub fn build_payload_with_nonce(header: u8, l2data: &[u8], nonce: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + l2data.len().saturating_add(4));
    payload.push(header);
    payload.extend_from_slice(l2data);
    payload.extend_from_slice(&nonce.to_be_bytes());
    payload
}

/// Builds the L2Data portion of an IGRA payload from raw EVM transaction bytes.
///
/// Returns `(header_byte, l2data_bytes)`.
pub fn build_igra_l2data(
    raw_tx: &[u8],
    payload_compression: Option<&str>,
) -> Result<(u8, Vec<u8>), String> {
    const IGRA_VERSION: u8 = 0x9;
    const TX_TYPE_RAW_UNCOMPRESSED: u8 = 0x4;

    let mode = payload_compression
        .unwrap_or("none")
        .trim()
        .to_ascii_lowercase();
    match mode.as_str() {
        "" | "none" => {
            let header = (IGRA_VERSION << 4) | TX_TYPE_RAW_UNCOMPRESSED;
            Ok((header, raw_tx.to_vec()))
        }
        "zlib" => Err(
            "IGRA config error: `payload_compression=zlib` is not implemented; use `none`"
                .to_string(),
        ),
        _ => Err(format!(
            "IGRA config error: `payload_compression` is invalid (supported: none)"
        )),
    }
}

/// Mines a nonce to match a Kaspa TX ID prefix, builds and signs the Kaspa transaction.
///
/// Returns `(payload_nonce, signed_kaspa_transaction)`.
pub fn mine_and_build_signed_payload_transaction(
    private_key: &[u8; 32],
    source_address: &KaspaAddress,
    network_type: KaspaNetworkType,
    payload_header: u8,
    l2data: &[u8],
    tx_id_prefix: &[u8],
    timeout: Duration,
    utxos: &[RpcUtxosByAddressesEntry],
) -> Result<(u64, KaspaTransaction), String> {
    if utxos.is_empty() {
        return Err(format!(
            "IGRA submit error: insufficient Kaspa UTXOs for fee payment (source address: {source_address})"
        ));
    }

    if tx_id_prefix.is_empty() {
        return Err("IGRA config error: `tx_id_prefix` cannot be empty".to_string());
    }

    let payload_len = 1usize.saturating_add(l2data.len()).saturating_add(4);

    let mut sorted = utxos.to_vec();
    sorted.sort_by_key(|entry| std::cmp::Reverse(entry.utxo_entry.amount));
    let mut selected = Vec::new();
    let mut total_input = 0u64;
    for entry in sorted {
        total_input = total_input.saturating_add(entry.utxo_entry.amount);
        selected.push(entry);
        let required_fee = estimated_fee_sompi(payload_len, selected.len());
        if total_input >= required_fee.saturating_add(MIN_CHANGE_SOMPI) {
            break;
        }
    }

    let fee = estimated_fee_sompi(payload_len, selected.len());
    if total_input < fee.saturating_add(MIN_CHANGE_SOMPI) {
        return Err(format!(
            "IGRA submit error: insufficient Kaspa UTXOs for fee payment (source address: {source_address})"
        ));
    }

    let output_value = total_input.saturating_sub(fee);
    if output_value < MIN_CHANGE_SOMPI {
        return Err(format!(
            "IGRA submit error: insufficient Kaspa UTXOs for fee payment (source address: {source_address})"
        ));
    }

    let script_public_key = pay_to_address_script(source_address);
    let inputs = selected
        .iter()
        .map(|entry| KaspaTransactionInput::new(entry.outpoint.clone().into(), Vec::new(), 0, 1))
        .collect::<Vec<_>>();
    let outputs = vec![KaspaTransactionOutput::new(output_value, script_public_key)];

    let payload = build_payload_with_nonce(payload_header, l2data, 0);
    let nonce_offset = payload.len().saturating_sub(4);
    let mut tx = KaspaTransaction::new(0, inputs, outputs, 0, SubnetworkId::default(), 0, payload);

    let start = Instant::now();
    let mut nonce = 0_u32;
    loop {
        if start.elapsed() > timeout {
            return Err(format!(
                "{IGRA_MINING_TIMEOUT_ERROR_CODE}: timed out mining kaspa txid prefix after {}ms",
                timeout.as_millis()
            ));
        }

        tx.payload[nonce_offset..].copy_from_slice(&nonce.to_be_bytes());
        tx.finalize();
        let tx_id = tx.id();
        if tx_id.as_bytes().starts_with(tx_id_prefix) {
            break;
        }

        nonce = nonce.wrapping_add(1);
        if nonce == 0 {
            if let Some(first) = tx.outputs.first_mut() {
                first.value = first.value.saturating_sub(1);
            }
            tx.finalize();
        }
    }

    let entries = selected
        .iter()
        .map(|entry| KaspaUtxoEntry {
            amount: entry.utxo_entry.amount,
            script_public_key: entry.utxo_entry.script_public_key.clone(),
            block_daa_score: entry.utxo_entry.block_daa_score,
            is_coinbase: entry.utxo_entry.is_coinbase,
        })
        .collect::<Vec<_>>();

    let signable = KaspaSignableTransaction::with_entries(tx, entries);
    let signed = kaspa_sign_with_multiple_v2(signable, std::slice::from_ref(private_key))
        .fully_signed()
        .map_err(|err| format!("IGRA submit error: failed to sign Kaspa tx: {err}"))?;
    kaspa_verify(&signed.as_verifiable())
        .map_err(|err| format!("IGRA submit error: invalid Kaspa signature set: {err}"))?;

    if !signed.tx.id().as_bytes().starts_with(tx_id_prefix) {
        return Err(
            "IGRA submit error: mined Kaspa txid prefix changed after signing; refusing to broadcast"
                .to_string(),
        );
    }

    let mass_calculator =
        KaspaMassCalculator::new_with_consensus_params(&KaspaParams::from(network_type));
    let non_contextual = mass_calculator.calc_non_contextual_masses(&signed.tx);
    let contextual = mass_calculator
        .calc_contextual_masses(&signed.as_verifiable())
        .ok_or_else(|| {
            "IGRA submit error: failed to calculate Kaspa tx storage mass".to_string()
        })?;
    let mass = contextual.max(non_contextual);
    if mass > MAX_STANDARD_KASPA_TX_MASS {
        return Err(format!(
            "IGRA submit error: Kaspa transaction mass {mass} exceeds standard limit {MAX_STANDARD_KASPA_TX_MASS}"
        ));
    }

    let tx = signed.tx;
    tx.set_mass(mass);
    Ok((nonce as u64, tx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_format_header_l2data_nonce() {
        let l2data = vec![0x01, 0x02, 0x03];
        let payload = build_payload_with_nonce(0x94, &l2data, 0);
        assert_eq!(payload.len(), 1 + 3 + 4);
        assert_eq!(payload[0], 0x94);
        assert_eq!(&payload[1..4], &[0x01, 0x02, 0x03]);
        assert_eq!(&payload[4..8], &[0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn payload_nonce_is_big_endian() {
        let payload = build_payload_with_nonce(0x94, &[], 0x01020304);
        assert_eq!(&payload[1..5], &[0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn payload_nonce_max_value() {
        let payload = build_payload_with_nonce(0x94, &[], u32::MAX);
        assert_eq!(&payload[1..5], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn build_l2data_uncompressed() {
        let raw_tx = vec![0xf8, 0x65];
        let (header, l2data) = build_igra_l2data(&raw_tx, None).unwrap();
        assert_eq!(header, 0x94);
        assert_eq!(l2data, raw_tx);
    }

    #[test]
    fn build_l2data_explicit_none() {
        let (header, _) = build_igra_l2data(&[0xf8], Some("none")).unwrap();
        assert_eq!(header, 0x94);
    }

    #[test]
    fn build_l2data_rejects_zlib() {
        assert!(build_igra_l2data(&[0x01], Some("zlib")).is_err());
    }

    #[test]
    fn build_l2data_rejects_unknown() {
        assert!(build_igra_l2data(&[0x01], Some("brotli")).is_err());
    }

    #[test]
    fn payload_header_encodes_version_and_type() {
        let (header, _) = build_igra_l2data(&[0x01], None).unwrap();
        assert_eq!(header >> 4, 0x9);
        assert_eq!(header & 0x0F, 0x4);
    }

    #[test]
    fn fee_base_for_single_input() {
        let fee = estimated_fee_sompi(100, 1);
        assert_eq!(fee, BASE_SUBMIT_FEE_SOMPI + FEE_PER_KIB_SOMPI);
    }

    #[test]
    fn fee_scales_with_payload_size() {
        let fee_1k = estimated_fee_sompi(1024, 1);
        let fee_2k = estimated_fee_sompi(2048, 1);
        assert_eq!(fee_2k - fee_1k, FEE_PER_KIB_SOMPI);
    }

    #[test]
    fn fee_scales_with_input_count() {
        let fee_1 = estimated_fee_sompi(100, 1);
        let fee_3 = estimated_fee_sompi(100, 3);
        assert_eq!(fee_3 - fee_1, 2 * EXTRA_INPUT_FEE_SOMPI);
    }

    #[test]
    fn fee_does_not_overflow() {
        let fee = estimated_fee_sompi(usize::MAX, usize::MAX);
        assert!(fee > 0);
    }

    #[test]
    fn normalize_hex_strips_0x_and_lowercases() {
        assert_eq!(normalize_hex_prefix("0x97B1".to_string()), "97b1");
        assert_eq!(normalize_hex_prefix("97b1".to_string()), "97b1");
        assert_eq!(normalize_hex_prefix("  0x97B1  ".to_string()), "97b1");
        assert_eq!(normalize_hex_prefix("".to_string()), "");
    }
}
