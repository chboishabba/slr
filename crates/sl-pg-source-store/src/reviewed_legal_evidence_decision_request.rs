//! Strict REAL-MATTER-1 reviewed-evidence entrypoint.
//!
//! Unlike the compatibility `ReviewedLegalEvidenceSelection`, this surface does
//! not let the caller restate review receipt, evidence role, normative order,
//! proposition, source manifestation, or candidate coordinates after review.
//! Those coordinates are reopened from one durable human review decision.

use thiserror::Error;

use crate::{
    load_legal_evidence_review_decision, resolve_reviewed_legal_evidence_request,
    DatabaseConfig, LegalEvidenceReviewDecisionError, ReviewedLegalEvidenceRequest,
    ReviewedLegalEvidenceRequestError, ReviewedLegalEvidenceSelection,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionBackedReviewedLegalEvidenceSelection {
    pub review_decision_ref: String,
    pub pnf_build_ref: String,
    pub refined_pnf_graph_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub structural_signature_ref: String,
    pub predicate_ref: String,
    pub legal_system_refs: Vec<String>,
    pub jurisdiction_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub author_ref: String,
    pub institution_ref: Option<String>,
}

#[derive(Debug, Error)]
pub enum DecisionBackedReviewedLegalEvidenceError {
    #[error(transparent)]
    Decision(#[from] LegalEvidenceReviewDecisionError),
    #[error(transparent)]
    Request(#[from] ReviewedLegalEvidenceRequestError),
    #[error("required decision-backed reviewed-evidence coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
}

pub fn resolve_reviewed_legal_evidence_from_decision(
    config: &DatabaseConfig,
    selection: &DecisionBackedReviewedLegalEvidenceSelection,
) -> Result<ReviewedLegalEvidenceRequest, DecisionBackedReviewedLegalEvidenceError> {
    validate_selection(selection)?;
    let decision = load_legal_evidence_review_decision(config, &selection.review_decision_ref)?;

    let compatibility = ReviewedLegalEvidenceSelection {
        source_manifestation_ref: decision.source_manifestation_ref,
        review_receipt_ref: decision.review_receipt_ref,
        observation_ref: decision.observation_ref,
        consumer_ref: decision.consumer_ref,
        requirement_ref: decision.requirement_ref,
        evidence_role_ref: decision.evidence_role_ref,
        normative_order_ref: decision.normative_order_ref,
        proposition_ref: decision.proposition_ref,
        candidate_pnf_batch_ref: decision.candidate_pnf_batch_ref,
        candidate_factor_ref: decision.candidate_factor_ref,
        pnf_build_ref: selection.pnf_build_ref.clone(),
        refined_pnf_graph_ref: selection.refined_pnf_graph_ref.clone(),
        pnf_factor_ref: selection.pnf_factor_ref.clone(),
        pnf_revision_ref: selection.pnf_revision_ref.clone(),
        structural_signature_ref: selection.structural_signature_ref.clone(),
        predicate_ref: selection.predicate_ref.clone(),
        legal_system_refs: selection.legal_system_refs.clone(),
        jurisdiction_refs: selection.jurisdiction_refs.clone(),
        temporal_refs: selection.temporal_refs.clone(),
        author_ref: selection.author_ref.clone(),
        institution_ref: selection.institution_ref.clone(),
    };
    Ok(resolve_reviewed_legal_evidence_request(config, &compatibility)?)
}

fn validate_selection(
    selection: &DecisionBackedReviewedLegalEvidenceSelection,
) -> Result<(), DecisionBackedReviewedLegalEvidenceError> {
    for (name, value) in [
        ("review_decision_ref", selection.review_decision_ref.as_str()),
        ("pnf_build_ref", selection.pnf_build_ref.as_str()),
        ("refined_pnf_graph_ref", selection.refined_pnf_graph_ref.as_str()),
        ("pnf_factor_ref", selection.pnf_factor_ref.as_str()),
        ("pnf_revision_ref", selection.pnf_revision_ref.as_str()),
        ("structural_signature_ref", selection.structural_signature_ref.as_str()),
        ("predicate_ref", selection.predicate_ref.as_str()),
        ("author_ref", selection.author_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(DecisionBackedReviewedLegalEvidenceError::EmptyCoordinate(name));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_selection_cannot_restate_role_or_normative_order() {
        let value = DecisionBackedReviewedLegalEvidenceSelection {
            review_decision_ref: "decision:1".into(),
            pnf_build_ref: "build:1".into(),
            refined_pnf_graph_ref: "graph:1".into(),
            pnf_factor_ref: "factor:1".into(),
            pnf_revision_ref: "revision:1".into(),
            structural_signature_ref: "signature:1".into(),
            predicate_ref: "predicate:1".into(),
            legal_system_refs: vec!["legal-system:au".into()],
            jurisdiction_refs: vec!["AU".into()],
            temporal_refs: vec![],
            author_ref: "reviewer-selected-author".into(),
            institution_ref: None,
        };
        assert!(validate_selection(&value).is_ok());
    }
}
