//! Kaspa key derivation and address generation.

use crate::config::IgraKaspaWalletConfig;
use alloy_primitives::B256;
use alloy_signer_local::PrivateKeySigner;
use kaspa_addresses::{
    Address as KaspaAddress, Prefix as KaspaAddressPrefix, Version as KaspaAddressVersion,
};
use kaspa_bip32::{
    secp256k1::SecretKey as KaspaSecretKey, ChildNumber as KaspaChildNumber,
    DerivationPath as KaspaDerivationPath, ExtendedPrivateKey as KaspaExtendedPrivateKey,
    Language as KaspaLanguage, Mnemonic as KaspaMnemonic,
};
use kaspa_consensus_core::network::NetworkType as KaspaNetworkType;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Error returned when no usable Kaspa key material is available for IGRA submission.
pub const IGRA_KEY_RESOLUTION_ERROR: &str = "IGRA key resolution error: cannot derive Kaspa key from current EVM signer; provide --private-key-kaspa or --mnemonic-kaspa";

/// Maps a Kaspa network name to its type and address prefix.
pub fn kaspa_network_descriptor(
    network: &str,
) -> Result<(KaspaNetworkType, KaspaAddressPrefix), String> {
    match network {
        "mainnet" => Ok((KaspaNetworkType::Mainnet, KaspaAddressPrefix::Mainnet)),
        "testnet-10" => Ok((KaspaNetworkType::Testnet, KaspaAddressPrefix::Testnet)),
        "devnet" => Ok((KaspaNetworkType::Devnet, KaspaAddressPrefix::Devnet)),
        "simnet" => Ok((KaspaNetworkType::Simnet, KaspaAddressPrefix::Simnet)),
        "custom" => Err(
            "IGRA config error: `kaspa_network=custom` requires explicit in-process network mapping"
                .to_string(),
        ),
        other => Err(format!(
            "IGRA config error: unsupported kaspa_network `{other}`"
        )),
    }
}

/// Derives a Kaspa address from a 32-byte private key.
pub fn kaspa_address_from_private_key(
    private_key: &[u8; 32],
    prefix: KaspaAddressPrefix,
) -> Result<KaspaAddress, String> {
    let secret = KaspaSecretKey::from_slice(private_key)
        .map_err(|err| format!("IGRA key resolution error: invalid private key bytes: {err}"))?;
    let public_key = kaspa_bip32::secp256k1::PublicKey::from_secret_key_global(&secret);
    let payload = public_key.x_only_public_key().0.serialize();
    Ok(KaspaAddress::new(prefix, KaspaAddressVersion::PubKey, &payload))
}

/// Resolves a 32-byte Kaspa private key from wallet configuration.
///
/// Tries, in order: private_key hex, mnemonic, keystore.
pub fn resolve_kaspa_private_key(
    config: &IgraKaspaWalletConfig,
    keystores_dir: Option<&Path>,
) -> Result<[u8; 32], String> {
    if let Some(private_key) = config.private_key.as_deref() {
        return parse_private_key_hex(private_key);
    }

    if let Some(mnemonic) = config.mnemonic.as_deref() {
        return resolve_mnemonic_private_key(
            mnemonic,
            config.mnemonic_passphrase.as_deref(),
            config.mnemonic_derivation_path.as_deref(),
            config.mnemonic_index.unwrap_or(0),
        );
    }

    if config.keystore.is_some() || config.keystore_account.is_some() {
        return resolve_keystore_private_key(config, keystores_dir);
    }

    Err(IGRA_KEY_RESOLUTION_ERROR.to_string())
}

/// Parses a hex-encoded private key string into 32 bytes.
pub fn parse_private_key_hex(private_key: &str) -> Result<[u8; 32], String> {
    let private_key = private_key.trim();
    let key = private_key.parse::<B256>().map_err(|_| {
        format!("{IGRA_KEY_RESOLUTION_ERROR}: provided --private-key-kaspa value is invalid hex")
    })?;
    Ok(key.0)
}

/// Derives a Kaspa private key from a BIP39 mnemonic.
///
/// Uses Kaspa's BIP44 derivation path: `m/44'/111111'/0'/0/<index>` by default.
pub fn resolve_mnemonic_private_key(
    mnemonic: &str,
    passphrase: Option<&str>,
    derivation_path: Option<&str>,
    index: u32,
) -> Result<[u8; 32], String> {
    let phrase = if Path::new(mnemonic).is_file() {
        fs::read_to_string(mnemonic)
            .map_err(|err| format!("IGRA key resolution error: failed to read mnemonic file: {err}"))?
    } else {
        mnemonic.to_string()
    };
    let phrase = phrase.split_whitespace().collect::<Vec<_>>().join(" ");

    let kaspa_mnemonic = KaspaMnemonic::new(phrase, KaspaLanguage::English).map_err(|err| {
        format!("IGRA key resolution error: invalid Kaspa mnemonic: {err}")
    })?;
    let seed = kaspa_mnemonic.to_seed(passphrase.unwrap_or_default());

    let xprv = KaspaExtendedPrivateKey::<KaspaSecretKey>::new(seed).map_err(|err| {
        format!("IGRA key resolution error: failed to derive Kaspa master key from mnemonic seed: {err}")
    })?;

    let secret = if let Some(path) = derivation_path {
        let path = path
            .parse::<KaspaDerivationPath>()
            .map_err(|err| format!("IGRA key resolution error: invalid Kaspa derivation path: {err}"))?;
        *xprv
            .derive_path(&path)
            .map_err(|err| format!("IGRA key resolution error: failed to derive Kaspa key by path: {err}"))?
            .private_key()
    } else {
        let base = "m/44'/111111'/0'/0"
            .parse::<KaspaDerivationPath>()
            .map_err(|err| format!("IGRA key resolution error: failed to parse default Kaspa derivation path: {err}"))?;
        let base = xprv
            .derive_path(&base)
            .map_err(|err| format!("IGRA key resolution error: failed to derive default Kaspa base key: {err}"))?;
        *base
            .derive_child(
                KaspaChildNumber::new(index, false)
                    .map_err(|err| format!("IGRA key resolution error: invalid Kaspa mnemonic index: {err}"))?,
            )
            .map_err(|err| format!("IGRA key resolution error: failed to derive Kaspa key by index: {err}"))?
            .private_key()
    };

    Ok(secret.secret_bytes())
}

fn resolve_keystore_private_key(
    config: &IgraKaspaWalletConfig,
    keystores_dir: Option<&Path>,
) -> Result<[u8; 32], String> {
    let path = resolve_keystore_path(config, keystores_dir)?;
    let password = config.password.as_deref().ok_or_else(|| {
        "IGRA key resolution error: kaspa keystore password is required; set --password-kaspa or KASPA_PASSWORD".to_string()
    })?;
    let signer = PrivateKeySigner::decrypt_keystore(&path, password).map_err(|err| {
        format!("IGRA key resolution error: failed to decrypt kaspa keystore: {err}")
    })?;
    Ok(signer.credential().to_bytes().into())
}

fn resolve_keystore_path(
    config: &IgraKaspaWalletConfig,
    keystores_dir: Option<&Path>,
) -> Result<PathBuf, String> {
    if let Some(path) = config.keystore.as_ref() {
        return Ok(PathBuf::from(path));
    }

    if let Some(account) = config.keystore_account.as_ref() {
        let keystore_dir = keystores_dir.ok_or_else(|| {
            "IGRA key resolution error: could not resolve default foundry keystore directory"
                .to_string()
        })?;
        return Ok(keystore_dir.join(account));
    }

    Err("IGRA key resolution error: kaspa keystore path or account is required".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_key_with_0x_prefix() {
        let key = "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let result = parse_private_key_hex(key);
        assert!(result.is_ok());
        assert_eq!(result.unwrap()[0], 0x01);
    }

    #[test]
    fn parse_hex_key_without_prefix() {
        let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(parse_private_key_hex(key).is_ok());
    }

    #[test]
    fn parse_hex_key_rejects_short() {
        assert!(parse_private_key_hex("0x1234").is_err());
    }

    #[test]
    fn parse_hex_key_trims_whitespace() {
        let key = "  0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef  ";
        assert!(parse_private_key_hex(key).is_ok());
    }

    #[test]
    fn resolve_from_private_key_config() {
        let config = IgraKaspaWalletConfig {
            private_key: Some(
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
            ),
            ..Default::default()
        };
        assert!(resolve_kaspa_private_key(&config, None).is_ok());
    }

    #[test]
    fn resolve_from_mnemonic_config() {
        let config = IgraKaspaWalletConfig {
            mnemonic: Some(
                "test test test test test test test test test test test junk".to_string(),
            ),
            ..Default::default()
        };
        assert!(resolve_kaspa_private_key(&config, None).is_ok());
    }

    #[test]
    fn resolve_fails_with_empty_config() {
        let config = IgraKaspaWalletConfig::default();
        let err = resolve_kaspa_private_key(&config, None).unwrap_err();
        assert!(err.contains("IGRA key resolution error"));
    }

    #[test]
    fn mnemonic_different_indices_produce_different_keys() {
        let mnemonic = "test test test test test test test test test test test junk";
        let key0 = resolve_mnemonic_private_key(mnemonic, None, None, 0).unwrap();
        let key1 = resolve_mnemonic_private_key(mnemonic, None, None, 1).unwrap();
        assert_ne!(key0, key1);
    }

    #[test]
    fn mnemonic_passphrase_produces_different_key() {
        let mnemonic = "test test test test test test test test test test test junk";
        let key_no_pass = resolve_mnemonic_private_key(mnemonic, None, None, 0).unwrap();
        let key_with_pass =
            resolve_mnemonic_private_key(mnemonic, Some("mypassword"), None, 0).unwrap();
        assert_ne!(key_no_pass, key_with_pass);
    }

    #[test]
    fn invalid_mnemonic_returns_error() {
        let result = resolve_mnemonic_private_key("not a valid mnemonic", None, None, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid Kaspa mnemonic"));
    }

    #[test]
    fn network_descriptor_maps_correctly() {
        assert!(kaspa_network_descriptor("mainnet").is_ok());
        assert!(kaspa_network_descriptor("testnet-10").is_ok());
        assert!(kaspa_network_descriptor("devnet").is_ok());
        assert!(kaspa_network_descriptor("simnet").is_ok());
        assert!(kaspa_network_descriptor("custom").is_err());
        assert!(kaspa_network_descriptor("invalid").is_err());
    }

    #[test]
    fn mainnet_address_has_kaspa_prefix() {
        let key = [0x01u8; 32];
        let address = kaspa_address_from_private_key(&key, KaspaAddressPrefix::Mainnet).unwrap();
        assert!(address.to_string().starts_with("kaspa:"));
    }

    #[test]
    fn testnet_address_has_kaspatest_prefix() {
        let key = [0x01u8; 32];
        let address = kaspa_address_from_private_key(&key, KaspaAddressPrefix::Testnet).unwrap();
        assert!(address.to_string().starts_with("kaspatest:"));
    }

    // Regression test vectors from the fork
    #[test]
    fn known_mnemonic_with_passphrase_matches_expected_testnet_address() {
        let mnemonic = "test test test test test test test test test test test junk";
        let private_key =
            resolve_mnemonic_private_key(mnemonic, Some(mnemonic), None, 0).unwrap();
        let address =
            kaspa_address_from_private_key(&private_key, KaspaAddressPrefix::Testnet).unwrap();
        assert_eq!(
            address.to_string(),
            "kaspatest:qzf364tlnl7ja0w65ydu0m5l70pur2hcm3l3ahkmhs660zcyf7cvuf6uznufr"
        );
    }

    #[test]
    fn known_mnemonic_empty_passphrase_matches_expected_testnet_address() {
        let mnemonic = "test test test test test test test test test test test junk";
        let private_key = resolve_mnemonic_private_key(mnemonic, None, None, 0).unwrap();
        let address =
            kaspa_address_from_private_key(&private_key, KaspaAddressPrefix::Testnet).unwrap();
        assert_ne!(
            address.to_string(),
            "kaspatest:qzf364tlnl7ja0w65ydu0m5l70pur2hcm3l3ahkmhs660zcyf7cvuf6uznufr"
        );
        assert_eq!(
            address.to_string(),
            "kaspatest:qzy7rgry649xpl6czj3ferxle8ls5ent0eg39xuhmujup0jlwsq3g67auy2y6"
        );
    }

    #[test]
    fn kaspa_address_version_is_pubkey() {
        let expected =
            "kaspatest:qzf364tlnl7ja0w65ydu0m5l70pur2hcm3l3ahkmhs660zcyf7cvuf6uznufr";
        let addr = KaspaAddress::try_from(expected).expect("parse kaspa address");
        assert_eq!(addr.prefix, KaspaAddressPrefix::Testnet);
        assert_eq!(addr.version, KaspaAddressVersion::PubKey);
        assert_eq!(addr.payload.len(), 32);
    }
}
