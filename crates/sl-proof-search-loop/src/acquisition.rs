use crate::world::{ResearchWorldSnapshot, SourceRevisionRecord, WorldExtensionError};
use sensiblaw_governed_legal_provider::{LocalIngestionReceipt, RECEIPT_AUTHORITY};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcquiredAuthorityHandoff {
    pub source_revision_ref: String,
    pub source_identity_ref: String,
    pub provider_receipt_ref: String,
    pub network_requests_used_to_acquire: u64,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcquisitionHandoffError {
    NotLocallyIngested,
    NonCandidateAuthority,
    WorldExtension(WorldExtensionError),
}

// The explicit arguments mirror the persisted acquisition-to-world receipt ABI.
#[allow(clippy::too_many_arguments)]
pub fn handoff_locally_ingested_authority(
    world: &mut ResearchWorldSnapshot,
    acquisition: &LocalIngestionReceipt,
    canonical_text_digest: impl Into<String>,
    parsed_pnf_ref: impl Into<String>,
    citation_topology_ref: impl Into<String>,
    jurisdiction_ref: Option<String>,
    source_role_ref: impl Into<String>,
    authority_candidate_ref: Option<String>,
) -> Result<AcquiredAuthorityHandoff, AcquisitionHandoffError> {
    if !acquisition.locally_ingested {
        return Err(AcquisitionHandoffError::NotLocallyIngested);
    }
    if acquisition.receipt_authority != RECEIPT_AUTHORITY {
        return Err(AcquisitionHandoffError::NonCandidateAuthority);
    }

    let provider_receipt_ref = format!(
        "provider:{:?}:{}",
        acquisition.provider, acquisition.canonical_bytes_digest
    );
    let record = SourceRevisionRecord {
        source_revision_ref: acquisition.source_revision_ref.clone(),
        canonical_bytes_digest: acquisition.canonical_bytes_digest.clone(),
        canonical_text_digest: canonical_text_digest.into(),
        provider_receipt_ref: provider_receipt_ref.clone(),
        jurisdiction_ref,
        source_role_ref: source_role_ref.into(),
        authority_candidate_ref,
        parsed_pnf_ref: parsed_pnf_ref.into(),
        citation_topology_ref: citation_topology_ref.into(),
        assessment_receipt_refs: Vec::new(),
    };
    world
        .append_source(record)
        .map_err(AcquisitionHandoffError::WorldExtension)?;

    Ok(AcquiredAuthorityHandoff {
        source_revision_ref: acquisition.source_revision_ref.clone(),
        source_identity_ref: acquisition.source_identity_ref.clone(),
        provider_receipt_ref,
        network_requests_used_to_acquire: acquisition.network_requests_used_to_acquire,
        candidate_only: true,
    })
}

pub fn acquisition_handoff_is_semantic_payment(_handoff: &AcquiredAuthorityHandoff) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_governed_legal_provider::{LegalProvider, LocalIngestionReceipt};

    #[test]
    fn official_acquisition_joins_existing_world_only_after_local_ingestion() {
        let acquisition = LocalIngestionReceipt {
            provider: LegalProvider::HighCourtAustralia,
            source_identity_ref: "case:[2026]-HCA-19".into(),
            source_revision_ref: "source:hca:2026:19:rev:fixture".into(),
            explicit_reference: "https://www.hcourt.gov.au/example".into(),
            canonical_bytes_digest: "sha256:bytes".into(),
            locally_ingested: true,
            network_requests_used_to_acquire: 1,
            receipt_authority: RECEIPT_AUTHORITY,
        };
        let mut world = ResearchWorldSnapshot {
            snapshot_ref: "world:before-official".into(),
            authority: "experimental_candidate_only",
            ..ResearchWorldSnapshot::default()
        };
        let handoff = handoff_locally_ingested_authority(
            &mut world,
            &acquisition,
            "sha256:text",
            "pnf:hca:2026:19",
            "citations:hca:2026:19",
            Some("AU".into()),
            "primary-case",
            Some("authority:[2026]-HCA-19".into()),
        )
        .unwrap();

        assert!(world
            .source_revisions
            .contains_key("source:hca:2026:19:rev:fixture"));
        assert_eq!(handoff.network_requests_used_to_acquire, 1);
        assert!(!acquisition_handoff_is_semantic_payment(&handoff));
    }
}
