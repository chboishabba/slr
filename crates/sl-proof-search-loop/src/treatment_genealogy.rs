//! Reviewed authority-treatment genealogy.
//!
//! This module assembles a temporal genealogy only from explicitly reviewed
//! proposition-level citation-use edges. Discovery, CitedBy traversal, lexical
//! hints, QIDs, and source acquisition remain upstream candidates and never
//! become treatment edges by themselves.

use crate::reasoning::{CitationUse, PropositionReasoningEdge};
use crate::review_unit_review::ReviewedCitationReviewUnitReceipt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalTreatmentEdge {
    pub citing_document_ref: String,
    pub cited_document_ref: String,
    pub citing_proposition_ref: String,
    pub cited_proposition_ref: String,
    pub citation_use: CitationUse,
    pub pinpoint_ref: Option<String>,
    pub court_ref: Option<String>,
    pub jurisdiction_ref: Option<String>,
    pub temporal_ref: Option<String>,
    pub reviewer_ref: String,
    pub source_revision_ref: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalTreatmentGenealogy {
    pub root_authority_ref: String,
    pub as_at: String,
    pub edges: Vec<TemporalTreatmentEdge>,
    pub unresolved_temporal_edges: usize,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreatmentGenealogyError {
    EmptyRoot,
    EmptyAsAt,
    UnreviewedEdge,
    NonCandidateEdge,
    RootNotReferenced,
}

fn treatment_edge(
    receipt: &ReviewedCitationReviewUnitReceipt,
) -> Result<TemporalTreatmentEdge, TreatmentGenealogyError> {
    let edge: &PropositionReasoningEdge = &receipt.edge;
    if !edge.reviewed {
        return Err(TreatmentGenealogyError::UnreviewedEdge);
    }
    if !edge.candidate_only {
        return Err(TreatmentGenealogyError::NonCandidateEdge);
    }
    Ok(TemporalTreatmentEdge {
        citing_document_ref: edge.citing_document_ref.clone(),
        cited_document_ref: edge.cited_document_ref.clone(),
        citing_proposition_ref: edge.citing_proposition_ref.clone(),
        cited_proposition_ref: edge.cited_proposition_ref.clone(),
        citation_use: edge.citation_use,
        pinpoint_ref: edge.pinpoint_ref.clone(),
        court_ref: edge.court_ref.clone(),
        jurisdiction_ref: edge.jurisdiction_ref.clone(),
        temporal_ref: edge.temporal_ref.clone(),
        reviewer_ref: receipt.reviewer_ref.clone(),
        source_revision_ref: receipt.source_revision_ref.clone(),
        candidate_only: true,
        creates_legal_authority: false,
    })
}

pub fn build_temporal_treatment_genealogy(
    root_authority_ref: &str,
    as_at: &str,
    receipts: &[ReviewedCitationReviewUnitReceipt],
) -> Result<TemporalTreatmentGenealogy, TreatmentGenealogyError> {
    if root_authority_ref.trim().is_empty() {
        return Err(TreatmentGenealogyError::EmptyRoot);
    }
    if as_at.trim().is_empty() {
        return Err(TreatmentGenealogyError::EmptyAsAt);
    }

    let mut edges = receipts
        .iter()
        .map(treatment_edge)
        .collect::<Result<Vec<_>, _>>()?;

    edges.retain(|edge| {
        edge.cited_document_ref == root_authority_ref
            || edge.citing_document_ref == root_authority_ref
            || receipts.iter().any(|receipt| {
                receipt.edge.citing_document_ref == edge.cited_document_ref
                    || receipt.edge.cited_document_ref == edge.citing_document_ref
            })
    });

    if !receipts.is_empty()
        && !edges.iter().any(|edge| {
            edge.cited_document_ref == root_authority_ref
                || edge.citing_document_ref == root_authority_ref
        })
    {
        return Err(TreatmentGenealogyError::RootNotReferenced);
    }

    edges.sort_by(|left, right| {
        left.temporal_ref
            .cmp(&right.temporal_ref)
            .then(left.citing_document_ref.cmp(&right.citing_document_ref))
            .then(left.pinpoint_ref.cmp(&right.pinpoint_ref))
    });
    let unresolved_temporal_edges = edges
        .iter()
        .filter(|edge| edge.temporal_ref.as_deref().is_none_or(str::is_empty))
        .count();

    Ok(TemporalTreatmentGenealogy {
        root_authority_ref: root_authority_ref.to_string(),
        as_at: as_at.to_string(),
        edges,
        unresolved_temporal_edges,
        candidate_only: true,
        creates_legal_authority: false,
    })
}

pub const fn genealogy_is_current_law_conclusion(
    _genealogy: &TemporalTreatmentGenealogy,
) -> bool {
    false
}

pub const fn treatment_count_is_authority_weight(
    _genealogy: &TemporalTreatmentGenealogy,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::{ReasoningRole};
    use crate::review_unit_review::ReviewedCitationReviewUnitReceipt;

    fn receipt(citing: &str, cited: &str, year: &str, use_: CitationUse) -> ReviewedCitationReviewUnitReceipt {
        ReviewedCitationReviewUnitReceipt {
            review_unit_ref: format!("review-unit:{citing}:{cited}"),
            document_ref: citing.into(),
            source_revision_ref: format!("revision:{citing}"),
            canonical_text_sha256: "sha256:fixture".into(),
            citation_text: "[1988] HCA 7".into(),
            citation_locator_refs: vec![format!("{citing}#citation")],
            anchor_paragraph_locator_refs: vec![format!("{citing}#paragraph")],
            selected_anchor_paragraph_locator_ref: format!("{citing}#paragraph"),
            matched_criterion_refs: vec!["criterion:estoppel:treatment".into()],
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["evidence:fixture".into()],
            edge: PropositionReasoningEdge {
                citing_document_ref: citing.into(),
                citing_proposition_ref: format!("prop:{citing}"),
                cited_document_ref: cited.into(),
                cited_proposition_ref: format!("prop:{cited}"),
                citation_use: use_,
                reasoning_role: ReasoningRole::Rule,
                condition_coordinates: vec![],
                pinpoint_ref: Some(format!("{citing}#paragraph")),
                judge_or_speaker_ref: None,
                court_ref: Some("HCA".into()),
                jurisdiction_ref: Some("AU".into()),
                temporal_ref: Some(year.into()),
                outcome_ref: None,
                remedy_ref: None,
                burden_refs: vec![],
                exception_refs: vec![],
                lexical_realisation: "fixture reviewed treatment".into(),
                reviewed: true,
                candidate_only: true,
            },
            receipt_authority: "experimental_candidate_only",
        }
    }

    #[test]
    fn reviewed_treatment_edges_form_temporal_genealogy_without_truth_promotion() {
        let receipts = vec![
            receipt("case:sidhu", "case:waltons", "2014", CitationUse::Applied),
            receipt("case:cosmopolitan", "case:waltons", "2016", CitationUse::Distinguished),
        ];
        let genealogy = build_temporal_treatment_genealogy(
            "case:waltons",
            "2026-09-20",
            &receipts,
        )
        .unwrap();
        assert_eq!(genealogy.edges.len(), 2);
        assert_eq!(genealogy.edges[0].temporal_ref.as_deref(), Some("2014"));
        assert_eq!(genealogy.edges[1].temporal_ref.as_deref(), Some("2016"));
        assert!(!genealogy.creates_legal_authority);
        assert!(!genealogy_is_current_law_conclusion(&genealogy));
        assert!(!treatment_count_is_authority_weight(&genealogy));
    }
}
