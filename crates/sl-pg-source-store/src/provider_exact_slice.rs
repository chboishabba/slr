//! Exact-span payment for a provider materialisation.
//!
//! A consumer may navigate by durable skeleton/provider coordinates while full
//! bytes are evicted. Before an exact quotation/parser/review action, the pinned
//! source must be resident and digest-verified. This helper then reuses the
//! existing migration-181 source writer to persist exact `span` and
//! `external_source_slice` coordinates without creating semantic/legal authority.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{
    load_provider_materialization, verify_provider_rematerialization, DatabaseConfig,
    ExactResolutionReceipt, PersistedSourceRefs, PostgresSourceStore,
    ProviderMaterializationError, ResolutionPath, ResolvedExternalDocument, SourceSlice,
    SourceStoreError, TemporalCoverage,
};

#[derive(Debug, Error)]
pub enum ProviderExactSliceError {
    #[error(transparent)]
    Provider(#[from] ProviderMaterializationError),
    #[error(transparent)]
    Store(#[from] SourceStoreError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("provider bytes are evicted; exact span payment requires verified resident bytes")]
    BytesNotResident,
    #[error("resident provider payload is not valid UTF-8")]
    InvalidUtf8,
    #[error("persisted provider temporal coverage is unknown: {0}")]
    UnknownTemporalCoverage(String),
    #[error("persisted provider resolution path is unknown: {0}")]
    UnknownResolutionPath(String),
    #[error("persisted exact source-resolution owner is missing")]
    MissingResolution,
}

pub fn persist_provider_exact_slices(
    config: &DatabaseConfig,
    materialization_ref: &str,
    slices: &[SourceSlice<'_>],
) -> Result<PersistedSourceRefs, ProviderExactSliceError> {
    let materialization = load_provider_materialization(config, materialization_ref)?;
    if !materialization.bytes_resident {
        return Err(ProviderExactSliceError::BytesNotResident);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let source_row = client.query_one(
        r#"SELECT r.document_type_ref,r.temporal_coverage_ref,r.resolution_path_ref,
                  c.payload
           FROM corpus.external_source_revision r
           JOIN corpus.document d ON d.document_ref=r.document_ref
           JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
           WHERE r.external_source_revision_ref=$1 AND r.document_ref=$2"#,
        &[
            &materialization.external_source_revision_ref,
            &materialization.document_ref,
        ],
    )?;
    let payload: Option<Vec<u8>> = source_row.get(3);
    let payload = payload.ok_or(ProviderExactSliceError::BytesNotResident)?;
    let canonical_text = String::from_utf8(payload).map_err(|_| ProviderExactSliceError::InvalidUtf8)?;
    verify_provider_rematerialization(&materialization.canonical_sha256_hex, &canonical_text)?;

    let temporal_raw: String = source_row.get(1);
    let temporal_coverage = match temporal_raw.as_str() {
        "latest_known_only" => TemporalCoverage::LatestKnownOnly,
        "historically_verified" => TemporalCoverage::HistoricallyVerified,
        other => return Err(ProviderExactSliceError::UnknownTemporalCoverage(other.to_owned())),
    };
    let path_raw: String = source_row.get(2);
    let resolution_path = match path_raw.as_str() {
        "filter_exact" => ResolutionPath::FilterExact,
        "native_parquet_scan" => ResolutionPath::NativeParquetScan,
        "offline_jsonl_replay" => ResolutionPath::OfflineJsonlReplay,
        "revision_pinned_streaming_legacy" => ResolutionPath::RevisionPinnedStreamingLegacy,
        other => return Err(ProviderExactSliceError::UnknownResolutionPath(other.to_owned())),
    };

    let receipt_row = client
        .query_opt(
            r#"SELECT demand_ref,consumer_ref,requested_citation,
                      requested_jurisdiction_ref,requested_source_role_ref,
                      requested_authority_level_ref,requested_temporal_ref,
                      exact_demand_match,acquisition_authority_ref,receipt_authority_ref,
                      network_request_count,resolver_ref,resolution_evidence_ref
               FROM evidence.external_source_resolution
               WHERE external_source_revision_ref=$1
               ORDER BY created_at DESC LIMIT 1"#,
            &[&materialization.external_source_revision_ref],
        )?
        .ok_or(ProviderExactSliceError::MissingResolution)?;

    let demand_ref: String = receipt_row.get(0);
    let consumer_ref: Option<String> = receipt_row.get(1);
    let requested_citation: String = receipt_row.get(2);
    let requested_jurisdiction_ref: String = receipt_row.get(3);
    let requested_source_role_ref: String = receipt_row.get(4);
    let requested_authority_level_ref: String = receipt_row.get(5);
    let requested_temporal_ref: Option<String> = receipt_row.get(6);
    let exact_demand_match: bool = receipt_row.get(7);
    let acquisition_authority_ref: String = receipt_row.get(8);
    let receipt_authority_ref: String = receipt_row.get(9);
    let network_request_count: i64 = receipt_row.get(10);
    let resolver_ref: String = receipt_row.get(11);
    let resolution_evidence_ref: String = receipt_row.get(12);
    drop(client);

    let document_type_ref: String = source_row.get(0);
    let identity = &materialization.identity;
    let document = ResolvedExternalDocument {
        provider_ref: &identity.provider_ref,
        dataset_ref: &identity.dataset_ref,
        dataset_revision_ref: &identity.dataset_revision_ref,
        external_version_ref: &identity.external_version_ref,
        citation: &identity.citation,
        source_ref: &identity.source_ref,
        jurisdiction_ref: &identity.jurisdiction_ref,
        document_type_ref: &document_type_ref,
        temporal_coverage,
        resolution_path,
        source_url: identity.source_url.as_deref(),
        canonical_text: &canonical_text,
    };
    let resolution = ExactResolutionReceipt {
        demand_ref: &demand_ref,
        consumer_ref: consumer_ref.as_deref(),
        requested_citation: &requested_citation,
        requested_jurisdiction_ref: &requested_jurisdiction_ref,
        requested_source_role_ref: &requested_source_role_ref,
        requested_authority_level_ref: &requested_authority_level_ref,
        requested_temporal_ref: requested_temporal_ref.as_deref(),
        exact_demand_match,
        acquisition_authority_ref: &acquisition_authority_ref,
        receipt_authority_ref: &receipt_authority_ref,
        network_request_count,
        resolver_ref: &resolver_ref,
        resolution_evidence_ref: &resolution_evidence_ref,
    };
    let mut store = PostgresSourceStore::connect(config)?;
    let refs = store.persist_resolved_source(&document, &resolution, slices)?;
    if refs.document_ref != materialization.document_ref
        || refs.external_source_revision_ref != materialization.external_source_revision_ref
    {
        return Err(ProviderExactSliceError::MissingResolution);
    }
    Ok(refs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_slice_api_requires_materialisation_identity_and_explicit_slices() {
        let _ = persist_provider_exact_slices
            as fn(&DatabaseConfig, &str, &[SourceSlice<'_>])
                -> Result<PersistedSourceRefs, ProviderExactSliceError>;
    }

    #[test]
    fn ref_mismatch_has_a_dedicated_diagnostic() {
        assert_eq!(ProviderExactSliceError::RefMismatch.to_string(),
            "persisted exact source refs disagree with the owning provider materialisation");
    }
}
