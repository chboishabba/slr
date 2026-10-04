//! Thin bridge from the existing cache-first acquisition ABI to the
//! provider-pinned materialisation store. Provider adapters (OALC/HF today,
//! other source families later) remain responsible for acquisition; this layer
//! only pays exact persistence/replay coordinates.

use crate::{
    persist_provider_materialization, AcquiredSourceBundle, DatabaseConfig,
    ExactResolutionReceipt, PersistedProviderMaterialization,
    ProviderMaterializationError, ProviderMaterializationIdentity,
    ResolvedExternalDocument, SourceSlice,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderAcquisitionMaterializationReceipt {
    pub materialization: PersistedProviderMaterialization,
    pub demand_ref: String,
    pub resolver_ref: String,
    pub resolution_evidence_ref: String,
    pub exact_demand_match: bool,
    pub provider_bytes_are_cache_state: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub promotes_applicability: bool,
    pub promotes_claim_truth: bool,
}

pub fn provider_identity_from_acquired_bundle(
    bundle: &AcquiredSourceBundle,
    split_ref: &str,
) -> ProviderMaterializationIdentity {
    ProviderMaterializationIdentity {
        provider_ref: bundle.document.provider_ref.clone(),
        dataset_ref: bundle.document.dataset_ref.clone(),
        dataset_revision_ref: bundle.document.dataset_revision_ref.clone(),
        split_ref: split_ref.to_owned(),
        external_version_ref: bundle.document.external_version_ref.clone(),
        citation: bundle.document.citation.clone(),
        source_ref: bundle.document.source_ref.clone(),
        jurisdiction_ref: bundle.document.jurisdiction_ref.clone(),
        source_url: bundle.document.source_url.clone(),
        acquisition_receipt_ref: bundle.receipt.resolution_evidence_ref.clone(),
    }
}

pub fn persist_acquired_provider_bundle(
    config: &DatabaseConfig,
    bundle: &AcquiredSourceBundle,
    split_ref: &str,
    slices: &[SourceSlice<'_>],
) -> Result<ProviderAcquisitionMaterializationReceipt, ProviderMaterializationError> {
    let identity = provider_identity_from_acquired_bundle(bundle, split_ref);
    let document = ResolvedExternalDocument {
        provider_ref: &bundle.document.provider_ref,
        dataset_ref: &bundle.document.dataset_ref,
        dataset_revision_ref: &bundle.document.dataset_revision_ref,
        external_version_ref: &bundle.document.external_version_ref,
        citation: &bundle.document.citation,
        source_ref: &bundle.document.source_ref,
        jurisdiction_ref: &bundle.document.jurisdiction_ref,
        document_type_ref: &bundle.document.document_type_ref,
        temporal_coverage: bundle.document.temporal_coverage,
        resolution_path: bundle.document.resolution_path,
        source_url: bundle.document.source_url.as_deref(),
        canonical_text: &bundle.document.canonical_text,
    };
    let receipt = ExactResolutionReceipt {
        demand_ref: &bundle.receipt.demand_ref,
        consumer_ref: bundle.receipt.consumer_ref.as_deref(),
        requested_citation: &bundle.receipt.requested_citation,
        requested_jurisdiction_ref: &bundle.receipt.requested_jurisdiction_ref,
        requested_source_role_ref: &bundle.receipt.requested_source_role_ref,
        requested_authority_level_ref: &bundle.receipt.requested_authority_level_ref,
        requested_temporal_ref: bundle.receipt.requested_temporal_ref.as_deref(),
        exact_demand_match: bundle.receipt.exact_demand_match,
        acquisition_authority_ref: &bundle.receipt.acquisition_authority_ref,
        receipt_authority_ref: &bundle.receipt.receipt_authority_ref,
        network_request_count: bundle.receipt.network_request_count,
        resolver_ref: &bundle.receipt.resolver_ref,
        resolution_evidence_ref: &bundle.receipt.resolution_evidence_ref,
    };
    let materialization = persist_provider_materialization(
        config,
        &identity,
        &document,
        &receipt,
        slices,
    )?;
    Ok(ProviderAcquisitionMaterializationReceipt {
        materialization,
        demand_ref: bundle.receipt.demand_ref.clone(),
        resolver_ref: bundle.receipt.resolver_ref.clone(),
        resolution_evidence_ref: bundle.receipt.resolution_evidence_ref.clone(),
        exact_demand_match: true,
        provider_bytes_are_cache_state: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        promotes_applicability: false,
        promotes_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ExactResolutionReceiptOwned, ResolutionPath, ResolvedExternalDocumentOwned,
        TemporalCoverage,
    };

    #[test]
    fn acquired_bundle_identity_preserves_provider_pin() {
        let bundle = AcquiredSourceBundle {
            document: ResolvedExternalDocumentOwned {
                provider_ref: "provider:oalc-hf".into(),
                dataset_ref: "isaacus/open-australian-legal-corpus".into(),
                dataset_revision_ref: "0123456789abcdef0123456789abcdef01234567".into(),
                external_version_ref: "case:[1992] HCA 23".into(),
                citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
                source_ref: "oalc:mabo".into(),
                jurisdiction_ref: "AU".into(),
                document_type_ref: "case_law".into(),
                temporal_coverage: TemporalCoverage::HistoricallyVerified,
                resolution_path: ResolutionPath::NativeParquetScan,
                source_url: Some("hf://datasets/oalc".into()),
                canonical_text: "fixture".into(),
            },
            receipt: ExactResolutionReceiptOwned {
                demand_ref: "demand:mabo".into(),
                consumer_ref: Some("consumer:legal-follow".into()),
                requested_citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
                requested_jurisdiction_ref: "AU".into(),
                requested_source_role_ref: "primary_case_law".into(),
                requested_authority_level_ref: "high_court".into(),
                requested_temporal_ref: Some("1992".into()),
                exact_demand_match: true,
                acquisition_authority_ref: "governed-provider".into(),
                receipt_authority_ref: "source-observation-only".into(),
                network_request_count: 1,
                resolver_ref: "oalc-hf".into(),
                resolution_evidence_ref: "receipt:oalc:mabo".into(),
            },
        };
        let identity = provider_identity_from_acquired_bundle(&bundle, "train");
        assert_eq!(identity.dataset_revision_ref, bundle.document.dataset_revision_ref);
        assert_eq!(identity.external_version_ref, bundle.document.external_version_ref);
        assert_eq!(identity.acquisition_receipt_ref, bundle.receipt.resolution_evidence_ref);
    }
}
