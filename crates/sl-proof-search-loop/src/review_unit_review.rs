//! Explicit review of citation review units into the existing reasoning graph.
//!
//! Review-unit quotienting is only workload reduction.  A unit can enter the
//! reasoning graph only through an explicit decision bound to the exact unit,
//! source revision, citation, and one preserved body anchor.  The resulting
//! receipt retains every occurrence locator so quotienting never destroys
//! source provenance.

use crate::reasoning::{
    CitationUse, ConditionCoordinate, PropositionReasoningEdge, ReasoningGraphDelta, ReasoningRole,
};
use crate::review_units::CitationReviewUnit;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedCitationReviewUnitDecision {
    pub review_unit_ref: String,
    pub source_revision_ref: String,
    pub citation_text: String,
    pub selected_anchor_paragraph_locator_ref: String,
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
pub struct ReviewedCitationReviewUnitReceipt {
    pub review_unit_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub citation_text: String,
    pub citation_locator_refs: Vec<String>,
    pub anchor_paragraph_locator_refs: Vec<String>,
    pub selected_anchor_paragraph_locator_ref: String,
    pub matched_criterion_refs: Vec<String>,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub edge: PropositionReasoningEdge,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewUnitDecisionError {
    UnitNotCandidateOnly,
    UnitAlreadyReviewed,
    ReviewUnitMismatch,
    SourceRevisionMismatch,
    CitationMismatch,
    AnchorNotInUnit,
    MissingCitingProposition,
    MissingCitedDocument,
    MissingCitedProposition,
    MissingReviewer,
    MissingEvidence,
}

pub fn compile_reviewed_unit_receipt(
    unit: &CitationReviewUnit,
    decision: &ReviewedCitationReviewUnitDecision,
) -> Result<ReviewedCitationReviewUnitReceipt, ReviewUnitDecisionError> {
    if !unit.candidate_only {
        return Err(ReviewUnitDecisionError::UnitNotCandidateOnly);
    }
    if unit.reviewed {
        return Err(ReviewUnitDecisionError::UnitAlreadyReviewed);
    }
    if decision.review_unit_ref != unit.review_unit_ref {
        return Err(ReviewUnitDecisionError::ReviewUnitMismatch);
    }
    if decision.source_revision_ref != unit.source_revision_ref {
        return Err(ReviewUnitDecisionError::SourceRevisionMismatch);
    }
    if decision.citation_text != unit.citation_text {
        return Err(ReviewUnitDecisionError::CitationMismatch);
    }
    if !unit
        .anchor_paragraph_locator_refs
        .contains(&decision.selected_anchor_paragraph_locator_ref)
    {
        return Err(ReviewUnitDecisionError::AnchorNotInUnit);
    }
    if decision.citing_proposition_ref.is_empty() {
        return Err(ReviewUnitDecisionError::MissingCitingProposition);
    }
    if decision.cited_document_ref.is_empty() {
        return Err(ReviewUnitDecisionError::MissingCitedDocument);
    }
    if decision.cited_proposition_ref.is_empty() {
        return Err(ReviewUnitDecisionError::MissingCitedProposition);
    }
    if decision.reviewer_ref.is_empty() {
        return Err(ReviewUnitDecisionError::MissingReviewer);
    }
    if decision.evidence_refs.is_empty() || decision.evidence_refs.iter().any(String::is_empty) {
        return Err(ReviewUnitDecisionError::MissingEvidence);
    }

    let edge = PropositionReasoningEdge {
        citing_document_ref: unit.document_ref.clone(),
        citing_proposition_ref: decision.citing_proposition_ref.clone(),
        cited_document_ref: decision.cited_document_ref.clone(),
        cited_proposition_ref: decision.cited_proposition_ref.clone(),
        citation_use: decision.citation_use,
        reasoning_role: decision.reasoning_role,
        condition_coordinates: decision.condition_coordinates.clone(),
        pinpoint_ref: Some(decision.selected_anchor_paragraph_locator_ref.clone()),
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
    };

    Ok(ReviewedCitationReviewUnitReceipt {
        review_unit_ref: unit.review_unit_ref.clone(),
        document_ref: unit.document_ref.clone(),
        source_revision_ref: unit.source_revision_ref.clone(),
        canonical_text_sha256: unit.canonical_text_sha256.clone(),
        citation_text: unit.citation_text.clone(),
        citation_locator_refs: unit.citation_locator_refs.clone(),
        anchor_paragraph_locator_refs: unit.anchor_paragraph_locator_refs.clone(),
        selected_anchor_paragraph_locator_ref: decision.selected_anchor_paragraph_locator_ref.clone(),
        matched_criterion_refs: unit.matched_criterion_refs.clone(),
        reviewer_ref: decision.reviewer_ref.clone(),
        evidence_refs: decision.evidence_refs.clone(),
        edge,
        receipt_authority: "experimental_candidate_only",
    })
}

pub fn reviewed_unit_receipts_to_reasoning_delta(
    receipts: &[ReviewedCitationReviewUnitReceipt],
) -> ReasoningGraphDelta {
    ReasoningGraphDelta::from_edges(receipts.iter().map(|receipt| receipt.edge.clone()).collect())
}

pub const fn review_unit_receipt_is_binding_authority(
    _receipt: &ReviewedCitationReviewUnitReceipt,
) -> bool {
    false
}

pub const fn review_unit_receipt_is_consumer_closure(
    _receipt: &ReviewedCitationReviewUnitReceipt,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judgment_candidates::CitationOccurrenceCandidate;
    use crate::residual_review_shortlist::ResidualShortlistedCitation;
    use crate::review_units::cluster_shortlisted_citations;
    use crate::reasoning::{ConditionKind, ConditionCoordinate};

    fn unit() -> CitationReviewUnit {
        let item = |locator: &str| ResidualShortlistedCitation {
            candidate: CitationOccurrenceCandidate {
                document_ref: "document:cullen".into(),
                source_revision_ref: "source:revision-1".into(),
                canonical_text_sha256: "sha256:text".into(),
                paragraph_ordinal: 89,
                paragraph_locator_ref: locator.into(),
                reported_paragraph_label: Some("[89]".into()),
                citation_text: "[2018] AC 736".into(),
                paragraph_text: "[2018] AC 736".into(),
                anchor_paragraph_locator_refs: vec!["document:cullen#paragraph-89".into()],
                anchor_paragraph_texts: vec!["positive negligent conduct causing physical injury".into()],
                lexical_treatment_hints: vec![],
                reviewed: false,
                candidate_only: true,
            },
            matched_criterion_refs: vec!["criterion:cullen:positive-negligent-conduct".into()],
        };
        cluster_shortlisted_citations(&[
            item("document:cullen#footnote-43"),
            item("document:cullen#footnote-44"),
        ])
        .into_iter()
        .next()
        .unwrap()
    }

    fn decision(unit: &CitationReviewUnit) -> ReviewedCitationReviewUnitDecision {
        ReviewedCitationReviewUnitDecision {
            review_unit_ref: unit.review_unit_ref.clone(),
            source_revision_ref: unit.source_revision_ref.clone(),
            citation_text: unit.citation_text.clone(),
            selected_anchor_paragraph_locator_ref: "document:cullen#paragraph-89".into(),
            citing_proposition_ref: "prop:cullen:reviewed-positive-conduct-step".into(),
            cited_document_ref: "case:robinson-v-chief-constable-west-yorkshire".into(),
            cited_proposition_ref: "prop:robinson:reviewed-positive-conduct-rule".into(),
            citation_use: CitationUse::Applied,
            reasoning_role: ReasoningRole::Rule,
            condition_coordinates: vec![ConditionCoordinate {
                kind: ConditionKind::Legal,
                condition_ref: "condition:positive-conduct".into(),
            }],
            judge_or_speaker_ref: Some("speaker:fixture".into()),
            court_ref: Some("HCA".into()),
            jurisdiction_ref: Some("AU".into()),
            temporal_ref: Some("2026".into()),
            outcome_ref: None,
            remedy_ref: None,
            burden_refs: vec![],
            exception_refs: vec![],
            lexical_realisation: "positive negligent conduct".into(),
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["evidence:fixture-review".into()],
        }
    }

    #[test]
    fn reviewed_unit_preserves_all_occurrence_locators_while_compiling_one_edge() {
        let unit = unit();
        let receipt = compile_reviewed_unit_receipt(&unit, &decision(&unit)).unwrap();
        assert_eq!(receipt.citation_locator_refs.len(), 2);
        assert_eq!(receipt.edge.pinpoint_ref.as_deref(), Some("document:cullen#paragraph-89"));
        assert!(receipt.edge.reviewed);
        assert!(receipt.edge.candidate_only);
        assert!(!review_unit_receipt_is_binding_authority(&receipt));
        assert!(!review_unit_receipt_is_consumer_closure(&receipt));
    }

    #[test]
    fn reviewer_must_choose_an_anchor_owned_by_the_unit() {
        let unit = unit();
        let mut decision = decision(&unit);
        decision.selected_anchor_paragraph_locator_ref = "document:cullen#paragraph-148".into();
        assert_eq!(
            compile_reviewed_unit_receipt(&unit, &decision),
            Err(ReviewUnitDecisionError::AnchorNotInUnit)
        );
    }

    #[test]
    fn reviewed_unit_receipts_feed_existing_reasoning_delta_without_losing_candidate_boundary() {
        let unit = unit();
        let receipt = compile_reviewed_unit_receipt(&unit, &decision(&unit)).unwrap();
        let delta = reviewed_unit_receipts_to_reasoning_delta(&[receipt]);
        assert_eq!(delta.edges.len(), 1);
        assert_eq!(delta.delta_authority, "experimental_candidate_only");
        assert_eq!(delta.discovered_citation_refs.len(), 1);
    }
}
