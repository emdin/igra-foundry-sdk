//! IGRA error catalog and signer-flow guardrails.

use crate::config::IgraConfigError;
use eyre::Result;

/// Error code for unsupported signer flows in IGRA mode.
pub const IGRA_SIGNER_GUARDRAIL_CODE: &str = "IGRA_SIG_001";

/// User-facing mapping for IGRA error codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IgraErrorCatalogEntry {
    pub code: &'static str,
    pub message: &'static str,
    pub remediation: &'static str,
}

/// Returns a user-facing error catalog entry for a known IGRA error code.
pub fn error_catalog_entry(code: &str) -> Option<IgraErrorCatalogEntry> {
    match code {
        IGRA_SIGNER_GUARDRAIL_CODE => Some(IgraErrorCatalogEntry {
            code: IGRA_SIGNER_GUARDRAIL_CODE,
            message: "Unsupported signer flow in IGRA mode.",
            remediation: "Use a local raw-signing wallet instead of unlocked/browser flows.",
        }),
        "IGRA_NONCE_001" => Some(IgraErrorCatalogEntry {
            code: "IGRA_NONCE_001",
            message: "Transaction blocked by nonce gap for this sender.",
            remediation:
                "Submit missing lower-nonce transactions first or wait for prior nonce to progress.",
        }),
        "IGRA_NONCE_002" => Some(IgraErrorCatalogEntry {
            code: "IGRA_NONCE_002",
            message: "Sender lock acquisition timed out.",
            remediation:
                "Retry after in-flight transactions finish; investigate long-running sender operations.",
        }),
        "IGRA_NONCE_003" => Some(IgraErrorCatalogEntry {
            code: "IGRA_NONCE_003",
            message: "Sender nonce counter overflowed.",
            remediation:
                "Reset local IGRA cache for this sender/profile and resynchronize state.",
        }),
        "IGRA_NONCE_004" => Some(IgraErrorCatalogEntry {
            code: "IGRA_NONCE_004",
            message: "Stale nonce replacement candidate observed.",
            remediation:
                "Confirm replacement intent and track the latest tx hash/receipt for this nonce.",
        }),
        _ => None,
    }
}

/// Rejects unsupported signer flows in IGRA mode.
///
/// This is a decoupled version that takes pre-computed values instead of the
/// full Foundry `Config`. Call sites pass `config.igra.enabled` and
/// `config.validate_igra()` respectively.
pub fn ensure_supported_igra_signer_flow(
    igra_enabled: bool,
    config_validation: Result<(), IgraConfigError>,
    command: &str,
    uses_unlocked: bool,
    uses_browser_wallet: bool,
) -> Result<()> {
    config_validation?;
    if !igra_enabled {
        return Ok(());
    }

    if uses_unlocked {
        eyre::bail!(
            "{IGRA_SIGNER_GUARDRAIL_CODE}: IGRA mode does not support `--unlocked` in `{command}`; \
             use a local raw-signing wallet"
        );
    }

    if uses_browser_wallet {
        eyre::bail!(
            "{IGRA_SIGNER_GUARDRAIL_CODE}: IGRA mode does not support browser wallet signer flows in `{command}`; \
             use a local raw-signing wallet"
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{IgraConfig, IgraKaspaWalletConfig};

    fn valid_igra_config() -> IgraConfig {
        IgraConfig {
            enabled: true,
            el_rpc_url: Some("http://127.0.0.1:8545".to_string()),
            kaspa_rpc_url: Some("grpc://127.0.0.1:16110".to_string()),
            expected_el_chain_id: Some(1337),
            kaspa_network: Some("testnet-10".to_string()),
            tx_id_prefix: Some("97b1".to_string()),
            el_receipt_timeout_secs: Some(300),
            kaspa_wallet: IgraKaspaWalletConfig::default(),
            ..Default::default()
        }
    }

    #[test]
    fn rejects_unlocked_signer_flows() {
        let config = valid_igra_config();
        let err = ensure_supported_igra_signer_flow(
            config.enabled,
            config.validate(),
            "cast send",
            true,
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("IGRA_SIG_001"));
        assert!(err.contains("--unlocked"));
    }

    #[test]
    fn rejects_browser_signer_flows() {
        let config = valid_igra_config();
        let err = ensure_supported_igra_signer_flow(
            config.enabled,
            config.validate(),
            "cast send",
            false,
            true,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("IGRA_SIG_001"));
        assert!(err.contains("browser wallet"));
    }

    #[test]
    fn accepts_supported_signer_flows() {
        let config = valid_igra_config();
        assert!(ensure_supported_igra_signer_flow(
            config.enabled,
            config.validate(),
            "cast send",
            false,
            false,
        )
        .is_ok());
    }

    #[test]
    fn allows_any_flow_when_disabled() {
        assert!(ensure_supported_igra_signer_flow(
            false,
            Ok(()),
            "cast send",
            true,
            true,
        )
        .is_ok());
    }

    #[test]
    fn validates_config_before_signer_check() {
        let mut config = valid_igra_config();
        config.el_rpc_url = None; // invalid when enabled
        let err = ensure_supported_igra_signer_flow(
            config.enabled,
            config.validate(),
            "cast send",
            false,
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("el_rpc_url"));
    }

    #[test]
    fn exposes_error_catalog_entries() {
        let entry = error_catalog_entry("IGRA_NONCE_001").expect("known code should resolve");
        assert_eq!(entry.code, "IGRA_NONCE_001");
        assert!(entry.message.contains("nonce gap"));
        assert!(!entry.remediation.is_empty());
    }

    #[test]
    fn unknown_error_code_returns_none() {
        assert!(error_catalog_entry("UNKNOWN").is_none());
    }

    #[test]
    fn all_known_codes_resolve() {
        for code in [
            "IGRA_SIG_001",
            "IGRA_NONCE_001",
            "IGRA_NONCE_002",
            "IGRA_NONCE_003",
            "IGRA_NONCE_004",
        ] {
            assert!(
                error_catalog_entry(code).is_some(),
                "code {code} should resolve"
            );
        }
    }
}
