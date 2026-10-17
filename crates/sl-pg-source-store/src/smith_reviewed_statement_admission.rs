//! Narrow Smith [2026] HCA 37 reviewed-evidence -> statement/observation admission bridge.
//!
//! This module does not create observations, candidate PNF, reviewed evidence, or review
//! decisions. It only projects an already-reviewed Smith evidence coordinate into the existing
//! normalized `StatementObservationLink` ABI when the exact review is accepted.

use thiserror::Error;

use crate::{
    canonical_statement_observation_link_ref, StatementObservationDisposition,
    StatementObservationLink,
};

pub const SMITH_APPLICANT_STATEMENT_REF: &str =
    "statement:sha256:5c77ce0e974cd9b7d48919f041502589c79953a52991e9f35172d0ee2a4cf1d9";
pub const SMITH_RESPONDENT_STATEMENT_REF: &str =
    "statement:sha256:73b6abb92e79985ffe28c856472afd24d637809c628d1f51da8353be0ba449de";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmithReviewedAdmissionCandidate {
    pub reviewed_evidence_ref: String,
    pub statement_ref: String,
    pub source_revision_ref: String,
    pub candidate_pnf_batch_ref: String,
    pub observation_ref: String,
    pub review_receipt_ref: String,
    pub review_semantic_ref: String,
    pub review_action_ref: String,
    pub review_effect_ref: String,
    pub review_effect_value_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SmithReviewedStatementAdmissionError {
    #[error("reviewed Smith admission coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("statement is not one of the two exact Smith Matter statements: {0}")]
    WrongSmithStatement(String),
    #[error("review receipt is not an explicit accept/status_changed/accepted decision")]
    ReviewNotAccepted,
    #[error("review semantic_ref does not equal the reopened reviewed-evidence observation")]
    ReviewObservationMismatch,
    #[error("reviewed evidence crossed the candidate-only/non-promotion boundary")]
    PromotionBoundaryCrossed,
}

fn required(name: &'static str, value: &str) -> Result<(), SmithReviewedStatementAdmissionError> {
    if value.trim().is_empty() {
        Err(SmithReviewedStatementAdmissionError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

pub fn is_exact_smith_statement(statement_ref: &str) -> bool {
    matches!(
        statement_ref,
        SMITH_APPLICANT_STATEMENT_REF | SMITH_RESPONDENT_STATEMENT_REF
    )
}

pub fn smith_reviewed_statement_link_from_candidate(
    value: &SmithReviewedAdmissionCandidate,
) -> Result<StatementObservationLink, SmithReviewedStatementAdmissionError> {
    for (name, coordinate) in [
        ("reviewed_evidence_ref", value.reviewed_evidence_ref.as_str()),
        ("statement_ref", value.statement_ref.as_str()),
        ("source_revision_ref", value.source_revision_ref.as_str()),
        ("candidate_pnf_batch_ref", value.candidate_pnf_batch_ref.as_str()),
        ("observation_ref", value.observation_ref.as_str()),
        ("review_receipt_ref", value.review_receipt_ref.as_str()),
        ("review_semantic_ref", value.review_semantic_ref.as_str()),
        ("review_action_ref", value.review_action_ref.as_str()),
        ("review_effect_ref", value.review_effect_ref.as_str()),
    ] {
        required(name, coordinate)?;
    }
    if !is_exact_smith_statement(&value.statement_ref) {
        return Err(SmithReviewedStatementAdmissionError::WrongSmithStatement(
            value.statement_ref.clone(),
        ));
    }
    if value.review_semantic_ref != value.observation_ref {
        return Err(SmithReviewedStatementAdmissionError::ReviewObservationMismatch);
    }
    if value.review_action_ref != "accept"
        || value.review_effect_ref != "status_changed"
        || value.review_effect_value_ref.as_deref() != Some("accepted")
    {
        return Err(SmithReviewedStatementAdmissionError::ReviewNotAccepted);
    }
    if !value.candidate_only
        || value.creates_semantic_authority
        || value.applicability_promoted
        || value.claim_truth_promoted
    {
        return Err(SmithReviewedStatementAdmissionError::PromotionBoundaryCrossed);
    }

    Ok(StatementObservationLink {
        link_ref: canonical_statement_observation_link_ref(
            &value.statement_ref,
            &value.candidate_pnf_batch_ref,
            &value.observation_ref,
        ),
        statement_ref: value.statement_ref.clone(),
        candidate_pnf_ref: value.candidate_pnf_batch_ref.clone(),
        observation_ref: value.observation_ref.clone(),
        parser_receipt_ref: None,
        parse_review_ref: None,
        admission_receipt_ref: Some(value.review_receipt_ref.clone()),
        disposition: StatementObservationDisposition::SemanticallyAdmitted,
        qualification_ref: None,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
