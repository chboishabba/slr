//! Strict REAL-MATTER-1 resume path after the human legal review decision.
//!
//! This layer adds no new semantic inference. It reopens the durable legal
//! review decision, resolves the existing reviewed-evidence request, persists
//! reviewed evidence + LegalIR through their owners, then projects the reviewed
//! support relation into the existing challengeable legal-follow graph.
//!
//! REL is intentionally downstream: a relational comparison still requires a
//! second observation and explicit consumer contract and must not be invented
//! by this orchestration layer.

use thiserror::Error;

use crate::{
    materialize_reviewed_legal_evidence, persist_reviewed_legal_follow_projection,
    resolve_reviewed_legal_evidence_from_decision, DatabaseConfig,
    DecisionBackedReviewedLegalEvidenceError, DecisionBackedReviewedLegalEvidenceSelection,
    ReviewedLegalEvidenceMaterialization, ReviewedLegalEvidenceMaterializationError,
    ReviewedLegalFollowProjectionError, ReviewedLegalFollowProjectionReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMatterReviewedResumeReceipt {
    pub review_decision_ref: String,
    pub reviewed_evidence_ref: String,
    pub proposition_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub exact_span_ref: String,
    pub legal_ir_semantic_build_ref: String,
    pub legal_ir_projection_ref: String,
    pub legal_ir_observation_ref: String,
    pub legal_ir_graph_revision_ref: String,
    pub legal_follow_projection_ref: String,
    pub legal_follow_support_edge_ref: String,
    pub exact_native_source_reopened: bool,
    pub review_role_explicit: bool,
    pub normative_order_explicit: bool,
    pub derived_only: bool,
    pub challengeable: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum RealMatterReviewedResumeError {
    #[error(transparent)]
    DecisionRequest(#[from] DecisionBackedReviewedLegalEvidenceError),
    #[error(transparent)]
    ReviewedEvidence(#[from] ReviewedLegalEvidenceMaterializationError),
    #[error(transparent)]
    LegalFollow(#[from] ReviewedLegalFollowProjectionError),
    #[error("reviewed real-matter resume crossed a non-promotion boundary")]
    PromotionBoundary,
}

pub fn resume_real_matter_from_review_decision(
    config: &DatabaseConfig,
    selection: &DecisionBackedReviewedLegalEvidenceSelection,
) -> Result<RealMatterReviewedResumeReceipt, RealMatterReviewedResumeError> {
    let request = resolve_reviewed_legal_evidence_from_decision(config, selection)?;
    let materialized = materialize_reviewed_legal_evidence(config, &request)?;
    require_non_promoting(&materialized)?;

    let support = materialized.reviewed_evidence.as_legal_ir_support();
    let follow = persist_reviewed_legal_follow_projection(config, &support, &materialized.legal_ir)?;
    require_follow_non_promoting(&follow)?;

    Ok(RealMatterReviewedResumeReceipt {
        review_decision_ref: selection.review_decision_ref.clone(),
        reviewed_evidence_ref: materialized.reviewed_evidence.reviewed_evidence_ref.clone(),
        proposition_ref: materialized.reviewed_evidence.proposition_ref.clone(),
        source_revision_ref: materialized.reviewed_evidence.source_revision_ref.clone(),
        document_ref: materialized.reviewed_evidence.document_ref.clone(),
        exact_span_ref: materialized.reviewed_evidence.exact_span_ref.clone(),
        legal_ir_semantic_build_ref: materialized.legal_ir.semantic_build_ref.clone(),
        legal_ir_projection_ref: materialized.legal_ir.projection_ref.clone(),
        legal_ir_observation_ref: materialized.legal_ir.observation_ref.clone(),
        legal_ir_graph_revision_ref: materialized.legal_ir.graph_revision_ref.clone(),
        legal_follow_projection_ref: follow.projection_ref,
        legal_follow_support_edge_ref: follow.support_edge_ref,
        exact_native_source_reopened: materialized.exact_native_source_reopened,
        review_role_explicit: materialized.review_role_explicit,
        normative_order_explicit: materialized.normative_order_explicit,
        derived_only: follow.derived_only,
        challengeable: follow.challengeable,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

fn require_non_promoting(
    value: &ReviewedLegalEvidenceMaterialization,
) -> Result<(), RealMatterReviewedResumeError> {
    if !value.exact_native_source_reopened
        || !value.review_role_explicit
        || !value.normative_order_explicit
        || value.creates_semantic_authority
        || value.creates_legal_authority
        || value.applicability_promoted
        || value.claim_truth_promoted
    {
        return Err(RealMatterReviewedResumeError::PromotionBoundary);
    }
    Ok(())
}

fn require_follow_non_promoting(
    value: &ReviewedLegalFollowProjectionReceipt,
) -> Result<(), RealMatterReviewedResumeError> {
    if !value.derived_only
        || !value.challengeable
        || value.creates_semantic_authority
        || value.creates_claim_truth
        || value.creates_applicability
    {
        return Err(RealMatterReviewedResumeError::PromotionBoundary);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_receipt_keeps_rel_outside_this_boundary() {
        // Type-level/documentation regression: this receipt owns reviewed
        // evidence + LegalIR + legal-follow only. No comparison/residual/INV
        // coordinate is present here; those require a second observation and
        // consumer contract downstream.
        let _ = std::any::TypeId::of::<RealMatterReviewedResumeReceipt>();
    }
}
