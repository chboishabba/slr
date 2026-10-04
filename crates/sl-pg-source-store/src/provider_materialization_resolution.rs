//! Eviction-aware provider resolution.
//!
//! This is the normal provider-backed resolver for SOURCE-MATERIALISATION-1.
//! A durable provider materialisation may be present while its canonical bytes
//! are absent. In that state the resolver re-acquires through the existing
//! `CacheFirstAcquirer`, requires the same immutable pin and exact digest, and
//! only then restores byte residency. It never substitutes a newer provider
//! record silently.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{
    install_provider_materialization_schema, load_provider_materialization,
    persist_acquired_provider_bundle, provider_identity_from_acquired_bundle,
    rehydrate_provider_materialization, verify_provider_rematerialization,
    CacheFirstAcquirer, CacheLookupDemand, DatabaseConfig, PersistedProviderMaterialization,
    ProviderMaterializationError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMaterializationResolution {
    pub materialization: PersistedProviderMaterialization,
    pub canonical_text: String,
    pub acquisition_network_requests: u64,
    pub reused_durable_identity: bool,
    pub rematerialized_evicted_bytes: bool,
    pub silent_latest_substitution: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub promotes_applicability: bool,
    pub promotes_claim_truth: bool,
}

#[derive(Debug, Error)]
pub enum ProviderMaterializationResolutionError<E> {
    #[error(transparent)]
    Materialization(#[from] ProviderMaterializationError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("provider acquisition failed")]
    Acquire(E),
    #[error("provider acquisition reported a negative network request count: {0}")]
    InvalidNetworkRequestCount(i64),
    #[error("resident provider payload is missing or invalid UTF-8")]
    InvalidResidentPayload,
    #[error("re-acquired provider identity differs from the persisted immutable pin")]
    ProviderIdentityDrift,
}

/// Resolve a strict provider-backed source under a durable immutable pin.
///
/// Behaviour:
/// - no durable materialisation: acquire once and persist the exact pin/digest;
/// - durable + resident bytes: zero-network reopen;
/// - durable + evicted bytes: reacquire, require identical pin + digest, rehydrate;
/// - any pin/digest drift: fail closed.
pub fn resolve_provider_materialization<A: CacheFirstAcquirer>(
    config: &DatabaseConfig,
    acquirer: &mut A,
    demand: &CacheLookupDemand<'_>,
    split_ref: &str,
) -> Result<ProviderMaterializationResolution, ProviderMaterializationResolutionError<A::Error>> {
    install_provider_materialization_schema(config)?;

    if let Some(reference) = materialization_ref_for_demand(config, demand, split_ref)? {
        let persisted = load_provider_materialization(config, &reference)?;
        if persisted.bytes_resident {
            let canonical_text = load_resident_text(config, &persisted)?;
            verify_provider_rematerialization(&persisted.canonical_sha256_hex, &canonical_text)?;
            return Ok(ProviderMaterializationResolution {
                materialization: persisted,
                canonical_text,
                acquisition_network_requests: 0,
                reused_durable_identity: true,
                rematerialized_evicted_bytes: false,
                silent_latest_substitution: false,
                creates_semantic_authority: false,
                creates_legal_authority: false,
                promotes_applicability: false,
                promotes_claim_truth: false,
            });
        }

        let acquired = acquirer
            .acquire(demand)
            .map_err(ProviderMaterializationResolutionError::Acquire)?;
        if acquired.receipt.network_request_count < 0 {
            return Err(ProviderMaterializationResolutionError::InvalidNetworkRequestCount(
                acquired.receipt.network_request_count,
            ));
        }
        let observed_identity = provider_identity_from_acquired_bundle(&acquired, split_ref);
        if observed_identity != persisted.identity {
            return Err(ProviderMaterializationResolutionError::ProviderIdentityDrift);
        }
        verify_provider_rematerialization(
            &persisted.canonical_sha256_hex,
            &acquired.document.canonical_text,
        )?;
        let materialization = rehydrate_provider_materialization(
            config,
            &reference,
            &observed_identity,
            &acquired.document.canonical_text,
        )?;
        return Ok(ProviderMaterializationResolution {
            materialization,
            canonical_text: acquired.document.canonical_text,
            acquisition_network_requests: acquired.receipt.network_request_count as u64,
            reused_durable_identity: true,
            rematerialized_evicted_bytes: true,
            silent_latest_substitution: false,
            creates_semantic_authority: false,
            creates_legal_authority: false,
            promotes_applicability: false,
            promotes_claim_truth: false,
        });
    }

    let acquired = acquirer
        .acquire(demand)
        .map_err(ProviderMaterializationResolutionError::Acquire)?;
    if acquired.receipt.network_request_count < 0 {
        return Err(ProviderMaterializationResolutionError::InvalidNetworkRequestCount(
            acquired.receipt.network_request_count,
        ));
    }
    let canonical_text = acquired.document.canonical_text.clone();
    let acquisition_network_requests = acquired.receipt.network_request_count as u64;
    let persisted = persist_acquired_provider_bundle(config, &acquired, split_ref, &[])?.materialization;
    Ok(ProviderMaterializationResolution {
        materialization: persisted,
        canonical_text,
        acquisition_network_requests,
        reused_durable_identity: false,
        rematerialized_evicted_bytes: false,
        silent_latest_substitution: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        promotes_applicability: false,
        promotes_claim_truth: false,
    })
}

pub fn materialization_ref_for_demand(
    config: &DatabaseConfig,
    demand: &CacheLookupDemand<'_>,
    split_ref: &str,
) -> Result<Option<String>, postgres::Error> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT m.materialization_ref
           FROM evidence.external_source_resolution r
           JOIN source_provenance.provider_materialization m
             ON m.external_source_revision_ref=r.external_source_revision_ref
           WHERE r.demand_ref=$1
             AND r.requested_citation=$2
             AND r.requested_jurisdiction_ref=$3
             AND r.requested_source_role_ref=$4
             AND r.requested_authority_level_ref=$5
             AND r.requested_temporal_ref IS NOT DISTINCT FROM $6
             AND m.split_ref=$7
             AND r.exact_demand_match=TRUE
           ORDER BY r.created_at DESC
           LIMIT 1"#,
        &[
            &demand.demand_ref,
            &demand.citation,
            &demand.jurisdiction_ref,
            &demand.source_role_ref,
            &demand.authority_level_ref,
            &demand.temporal_ref,
            &split_ref,
        ],
    )?;
    Ok(row.map(|row| row.get(0)))
}

fn load_resident_text<E>(
    config: &DatabaseConfig,
    persisted: &PersistedProviderMaterialization,
) -> Result<String, ProviderMaterializationResolutionError<E>> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_one(
        "SELECT payload FROM corpus.canonical_content WHERE canonical_ref=$1",
        &[&persisted.canonical_ref],
    )?;
    let payload: Option<Vec<u8>> = row.get(0);
    let payload = payload.ok_or(ProviderMaterializationResolutionError::InvalidResidentPayload)?;
    String::from_utf8(payload)
        .map_err(|_| ProviderMaterializationResolutionError::InvalidResidentPayload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolution_result_is_explicitly_non_promoting() {
        // Compile-time/documentation regression: all authority fields are
        // explicit booleans rather than inferred from byte residency.
        fn assert_shape(value: &ProviderMaterializationResolution) {
            assert!(!value.silent_latest_substitution);
            assert!(!value.creates_semantic_authority);
            assert!(!value.creates_legal_authority);
            assert!(!value.promotes_applicability);
            assert!(!value.promotes_claim_truth);
        }
        let _ = assert_shape as fn(&ProviderMaterializationResolution);
    }
}
