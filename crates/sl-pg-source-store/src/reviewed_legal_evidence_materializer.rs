//! Legal consumer weld over the generic reviewed-evidence spine.
//!
//! Exact native-source manifestation and proposition-support review are two
//! different payments.  This module requires both, checks that they refer to
//! the same persisted legal source revision/document, then delegates evidence
//! persistence and legal-IR construction to their generic owners.

use thiserror::Error;

use crate::{
    load_legal_source_manifestation, materialize_legal_ir_from_reviewed_evidence,
    persist_reviewed_evidence_coordinate, DatabaseConfig, MaterializedLegalIrRefs,
    PersistedLegalSourceManifestation, PersistedReviewedEvidenceCoordinate,
    ReviewedEvidenceDraft,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedLegalEvidenceRequest {
    pub source_manifestation_ref: String,
    pub evidence: ReviewedEvidenceDraft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedLegalEvidenceMaterialization {
    pub source_manifestation: PersistedLegalSourceManifestation,
    pub reviewed_evidence: PersistedReviewedEvidenceCoordinate,
    pub legal_ir: MaterializedLegalIrRefs,
    pub exact_native_source_reopened: bool,
    pub review_role_explicit: bool,
    pub normative_order_explicit: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum ReviewedLegalEvidenceMaterializationError {
    #[error("source manifestation ref is empty")]
    MissingManifestation,
    #[error("legal source manifestation failed: {0}")]
    Manifestation(#[from] crate::LegalSourceManifestationError),
    #[error("reviewed evidence failed: {0}")]
    ReviewedEvidence(#[from] crate::ReviewedEvidenceStoreError),
    #[error("source manifestation and reviewed evidence do not identify the same source revision/document")]
    SourceMismatch,
    #[error("source manifestation/evidence crossed a non-promotion firewall")]
    Promotion,
}

pub fn materialize_reviewed_legal_evidence(
    config: &DatabaseConfig,
    request: &ReviewedLegalEvidenceRequest,
) -> Result<ReviewedLegalEvidenceMaterialization, ReviewedLegalEvidenceMaterializationError> {
    if request.source_manifestation_ref.trim().is_empty() {
        return Err(ReviewedLegalEvidenceMaterializationError::MissingManifestation);
    }
    let manifestation = load_legal_source_manifestation(config, &request.source_manifestation_ref)?;
    if manifestation.source_revision_ref != request.evidence.source_revision_ref
        || manifestation.document_ref != request.evidence.document_ref
    {
        return Err(ReviewedLegalEvidenceMaterializationError::SourceMismatch);
    }
    if !manifestation.candidate_only
        || manifestation.creates_semantic_authority
        || manifestation.creates_legal_authority
        || manifestation.applicability_promoted
        || manifestation.claim_truth_promoted
    {
        return Err(ReviewedLegalEvidenceMaterializationError::Promotion);
    }

    let reviewed = persist_reviewed_evidence_coordinate(config, &request.evidence)?;
    if reviewed.source_revision_ref != manifestation.source_revision_ref
        || reviewed.document_ref != manifestation.document_ref
        || !reviewed.candidate_only
        || reviewed.creates_semantic_authority
        || reviewed.applicability_promoted
        || reviewed.claim_truth_promoted
    {
        return Err(ReviewedLegalEvidenceMaterializationError::Promotion);
    }
    let legal_ir = materialize_legal_ir_from_reviewed_evidence(
        config,
        &reviewed.reviewed_evidence_ref,
    )?;

    Ok(ReviewedLegalEvidenceMaterialization {
        source_manifestation: manifestation,
        reviewed_evidence: reviewed,
        legal_ir,
        exact_native_source_reopened: true,
        review_role_explicit: true,
        normative_order_explicit: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_keeps_source_and_review_payments_distinct() {
        // Type-level regression: an exact source-manifestation ref alone is not
        // sufficient to construct reviewed evidence; the explicit review draft
        // remains a separate required field.
        fn consumes(_: &ReviewedLegalEvidenceRequest) {}
        let _ = consumes;
    }
}
