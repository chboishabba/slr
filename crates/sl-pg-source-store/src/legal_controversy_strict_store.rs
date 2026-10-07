//! Strict write facade for legal controversy child rows.
//!
//! The low-level store uses idempotent `ON CONFLICT DO NOTHING` inserts. The
//! REAL-MATTER production path must additionally reopen and compare the entire
//! requested object so a reused stable ref can never silently change meaning.

use crate::{DatabaseConfig, LegalControversyStoreError};
use super::legal_controversy_store as raw;

pub fn persist_typed_response_edge_strict(
    config: &DatabaseConfig,
    value: &raw::TypedResponseEdge,
) -> Result<raw::TypedResponseEdge, LegalControversyStoreError> {
    raw::install_legal_controversy_schema(config)?;
    let loaded = raw::persist_typed_response_edge(config, value)?;
    if loaded != *value {
        return Err(LegalControversyStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_legal_controversy_residual_strict(
    config: &DatabaseConfig,
    value: &raw::LegalControversyResidual,
) -> Result<raw::LegalControversyResidual, LegalControversyStoreError> {
    raw::install_legal_controversy_schema(config)?;
    let loaded = raw::persist_legal_controversy_residual(config, value)?;
    if loaded != *value {
        return Err(LegalControversyStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_legal_proof_obligation_strict(
    config: &DatabaseConfig,
    value: &raw::LegalProofObligation,
) -> Result<raw::LegalProofObligation, LegalControversyStoreError> {
    raw::install_legal_controversy_schema(config)?;
    let loaded = raw::persist_legal_proof_obligation(config, value)?;
    if loaded != *value {
        return Err(LegalControversyStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_reverse_legal_proof_search_strict(
    config: &DatabaseConfig,
    value: &raw::ReverseLegalProofSearch,
) -> Result<raw::ReverseLegalProofSearch, LegalControversyStoreError> {
    raw::install_legal_controversy_schema(config)?;
    let loaded = raw::persist_reverse_legal_proof_search(config, value)?;
    if loaded != *value {
        return Err(LegalControversyStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}
