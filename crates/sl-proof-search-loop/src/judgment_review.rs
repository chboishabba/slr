//! Explicit review gate from source-located citation candidates to the existing
//! proposition reasoning graph.
//!
//! Extraction does not promote itself.  A reviewed decision must identify the
//! exact candidate locator/citation and explicitly supply proposition identities,
//! citation use, reasoning role, conditions and reviewer evidence.

use crate::judgment_candidates::CitationOccurrenceCandidate;
use crate::reasoning::{
    CitationUse, ConditionCoordinate, PropositionReasoningEdge, ReasoningRole,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedCitationTreatmentDecision {
    pub candidate_paragraph_locator_ref: String,
    pub candidate_citation_text: String,
    pub citing_proposition_ref: String,
    pub cited_document_ref: String,
    pub cited_proposition_ref: String,
    pub citation_use: CitationUse,
    pub reasoning_role: ReasoningRole,
    pub condition_coordinates: Vec<ConditionCoordinate>,
    pub judge_or_speaker_ref: Option<String>,
    pub court_ref: Option<String>,
    pub jurisdiction_ref: Option<String>,
    pub temporal_ref: Option<String>,
    pub outcome_ref: Option<String>,
    pub remedy_ref: Option<String>,
    pub burden_refs: Vec<String>,
    pub exception_refs: Vec<String>,
    pub lexical_realisation: String,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CitationReviewError {
    CandidateNotCandidateOnly,
    CandidateAlreadyReviewed,
    LocatorMismatch,
    CitationMismatch,
    MissingCitingProposition,
    MissingCitedDocument,
    MissingCitedProposition,
    MissingReviewer,
    MissingEvidence,
}

pub fn compile_reviewed_candidate_edge(
    candidate: &CitationOccurrenceCandidate,
    decision: &ReviewedCitationTreatmentDecision,
) -> Result<PropositionReasoningEdge, CitationReviewError> {
    if !candidate.candidate_only {
        return Err(CitationReviewError::CandidateNotCandidateOnly);
    }
    if candidate.reviewed {
        return Err(CitationReviewError::CandidateAlreadyReviewed);
    }
    if decision.candidate_paragraph_locator_ref != candidate.paragraph_locator_ref {
        return Err(CitationReviewError::LocatorMismatch);
    }
    if decision.candidate_citation_text != candidate.citation_text {
        return Err(CitationReviewError::CitationMismatch);
    }
    if decision.citing_proposition_ref.is_empty() {
        return Err(CitationReviewError::MissingCitingProposition);
    }
    if decision.cited_document_ref.is_empty() {
        return Err(CitationReviewError::MissingCitedDocument);
    }
    if decision.cited_proposition_ref.is_empty() {
        return Err(CitationReviewError::MissingCitedProposition);
    }
    if decision.reviewer_ref.is_empty() {
        return Err(CitationReviewError::MissingReviewer);
    }
    if decision.evidence_refs.is_empty() || decision.evidence_refs.iter().any(String::is_empty) {
        return Err(CitationReviewError::MissingEvidence);
    }

    Ok(PropositionReasoningEdge {
        citing_document_ref: candidate.document_ref.clone(),
        citing_proposition_ref: decision.citing_proposition_ref.clone(),
        cited_document_ref: decision.cited_document_ref.clone(),
        cited_proposition_ref: decision.cited_proposition_ref.clone(),
        citation_use: decision.citation_use,
        reasoning_role: decision.reasoning_role,
        condition_coordinates: decision.condition_coordinates.clone(),
        pinpoint_ref: Some(candidate.paragraph_locator_ref.clone()),
        judge_or_speaker_ref: decision.judge_or_speaker_ref.clone(),
        court_ref: decision.court_ref.clone(),
        jurisdiction_ref: decision.jurisdiction_ref.clone(),
        temporal_ref: decision.temporal_ref.clone(),
        outcome_ref: decision.outcome_ref.clone(),
        remedy_ref: decision.remedy_ref.clone(),
        burden_refs: decision.burden_refs.clone(),
        exception_refs: decision.exception_refs.clone(),
        lexical_realisation: decision.lexical_realisation.clone(),
        reviewed: true,
        candidate_only: true,
    })
}

pub const fn lexical_hint_can_auto_compile_reviewed_edge() -> bool {
    false
}

pub const fn reviewed_edge_is_binding_authority(
    _edge: &PropositionReasoningEdge,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judgment_candidates::extract_judgment_citation_candidates;
    use crate::reasoning::{ConditionKind, ConditionCoordinate};

    fn candidate() -> CitationOccurrenceCandidate {
        extract_judgment_citation_candidates(
            "document:hca:[2026]-HCA-19:docx",
            "source-revision:fixture",
            "sha256:text-fixture",
            "[42] The Court applied Mallonland [2024] HCA 25 in this setting.\n",
        )
        .into_iter()
        .next()
        .unwrap()
    }

    fn decision(candidate: &CitationOccurrenceCandidate) -> ReviewedCitationTreatmentDecision {
        ReviewedCitationTreatmentDecision {
            candidate_paragraph_locator_ref: candidate.paragraph_locator_ref.clone(),
            candidate_citation_text: candidate.citation_text.clone(),
            citing_proposition_ref: "prop:cullen:reviewed-step".into(),
            cited_document_ref: "case:[2024]-HCA-25".into(),
            cited_proposition_ref: "prop:mallonland:salient-features".into(),
            citation_use: CitationUse::Applied,
            reasoning_role: ReasoningRole::Rule,
            condition_coordinates: vec![ConditionCoordinate {
                kind: ConditionKind::Legal,
                condition_ref: "condition:reviewed-salient-feature-context".into(),
            }],
            judge_or_speaker_ref: Some("speaker:fixture".into()),
            court_ref: Some("HCA".into()),
            jurisdiction_ref: Some("AU".into()),
            temporal_ref: Some("2026".into()),
            outcome_ref: None,
            remedy_ref: None,
            burden_refs: vec![],
            exception_refs: vec![],
            lexical_realisation: "applied Mallonland".into(),
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec![candidate.paragraph_locator_ref.clone()],
        }
    }

    #[test]
    fn explicit_review_can_compile_candidate_into_existing_reasoning_edge() {
        let candidate = candidate();
        let edge = compile_reviewed_candidate_edge(&candidate, &decision(&candidate)).unwrap();
        assert_eq!(edge.citation_use, CitationUse::Applied);
        assert_eq!(edge.reasoning_role, ReasoningRole::Rule);
        assert_eq!(edge.pinpoint_ref.as_deref(), Some(candidate.paragraph_locator_ref.as_str()));
        assert!(edge.reviewed);
        assert!(edge.candidate_only);
        assert!(!lexical_hint_can_auto_compile_reviewed_edge());
        assert!(!reviewed_edge_is_binding_authority(&edge));
    }

    #[test]
    fn wrong_locator_cannot_review_a_different_occurrence() {
        let candidate = candidate();
        let mut decision = decision(&candidate);
        decision.candidate_paragraph_locator_ref = "document:other#paragraph-7".into();
        assert_eq!(
            compile_reviewed_candidate_edge(&candidate, &decision),
            Err(CitationReviewError::LocatorMismatch)
        );
    }
}
