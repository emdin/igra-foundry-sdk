//! IGRA Network transport layer — Kaspa L1 submission for EVM L2 transactions.
//!
//! This crate implements the IGRA protocol: wrapping signed EVM transactions
//! inside Kaspa L1 transactions for data availability, with TX ID prefix mining,
//! UTXO management, and transaction lifecycle tracking.
//!
//! It is designed to be used as a standalone library or integrated into
//! Foundry via the IgraLabs/foundry fork.

pub mod config;
pub mod errors;
pub mod keys;
pub mod payload;
pub mod store;
pub mod submitter;
pub mod transport;

// Re-export key types at crate root for convenience.
pub use config::{IgraConfig, IgraConfigError, IgraKaspaWalletConfig};
pub use errors::{
    ensure_supported_igra_signer_flow, error_catalog_entry, IgraErrorCatalogEntry,
    IGRA_SIGNER_GUARDRAIL_CODE,
};
pub use keys::{kaspa_address_from_private_key, kaspa_network_descriptor, resolve_kaspa_private_key};
pub use payload::{build_igra_l2data, build_payload_with_nonce, estimated_fee_sompi};
pub use store::{IgraStore, IgraStoreConfig, NonceOrdering, TxLifecycleState, TxLifecycleUpdate};
pub use submitter::{
    IgraPayloadSubmitter, IgraSubmitRequest, IgraSubmitResult, InProcessKaspaPayloadSubmitter,
};
pub use transport::{IgraTransport, IgraTransportConfig};
