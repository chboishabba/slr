//! Residual-indexed pre-review shortlisting for anchored judgment citations.
//!
//! The application supplies the discriminator coordinates.  This module only
//! selects citation occurrences whose preserved body-anchor text matches one or
//! more supplied criteria for the exact residual/proposition under review.
//!
//! Shortlist membership is an observation/refinement result only.  It does not
//! establish proposition correspondence, citation treatment, ratio, authority,
//! applicability, proof payment or consumer closure.

use crate::judgment_candidates::CitationOccurrenceCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualAnchorCriterion {
    pub criterion_ref: String,
    pub residual_ref: String,
    pub proposition_ref: String,
    pub required_anchor_phrases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualCitationReviewDemand {
    pub residual_ref: String,
    pub proposition_ref: String,
    pub criteria: Vec<ResidualAnchorCriterion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualShortlistedCitation {
    pub candidate: CitationOccurrenceCandidate,
    pub matched_criterion_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResidualReviewShortlistError {
    MissingResidual,
    MissingProposition,
    MissingCriteria,
    CriterionResidualMismatch,
    CriterionPropositionMismatch,
    EmptyCriterionRef,
    EmptyRequiredPhrase,
}

fn anchor_matches_criterion(
    anchor_text: &str,
    criterion: &ResidualAnchorCriterion,
) -> bool {
    let lower = anchor_text.to_ascii_lowercase();
    criterion
        .required_anchor_phrases
        .iter()
        .all(|phrase| lower.contains(&phrase.to_ascii_lowercase()))
}

pub fn shortlist_anchored_citations_for_residual(
    candidates: &[CitationOccurrenceCandidate],
    demand: &ResidualCitationReviewDemand,
) -> Result<Vec<ResidualShortlistedCitation>, ResidualReviewShortlistError> {
    if demand.residual_ref.is_empty() {
        return Err(ResidualReviewShortlistError::MissingResidual);
    }
    if demand.proposition_ref.is_empty() {
        return Err(ResidualReviewShortlistError::MissingProposition);
    }
    if demand.criteria.is_empty() {
        return Err(ResidualReviewShortlistError::MissingCriteria);
    }
    for criterion in &demand.criteria {
        if criterion.residual_ref != demand.residual_ref {
            return Err(ResidualReviewShortlistError::CriterionResidualMismatch);
        }
        if criterion.proposition_ref != demand.proposition_ref {
            return Err(ResidualReviewShortlistError::CriterionPropositionMismatch);
        }
        if criterion.criterion_ref.is_empty() {
            return Err(ResidualReviewShortlistError::EmptyCriterionRef);
        }
        if criterion.required_anchor_phrases.is_empty()
            || criterion.required_anchor_phrases.iter().any(String::is_empty)
        {
            return Err(ResidualReviewShortlistError::EmptyRequiredPhrase);
        }
    }

    let mut out = Vec::new();
    for candidate in candidates {
        if !candidate.candidate_only || candidate.reviewed {
            continue;
        }
        let mut matched = Vec::new();
        for criterion in &demand.criteria {
            if candidate
                .anchor_paragraph_texts
                .iter()
                .any(|anchor| anchor_matches_criterion(anchor, criterion))
            {
                matched.push(criterion.criterion_ref.clone());
            }
        }
        matched.sort();
        matched.dedup();
        if !matched.is_empty() {
            out.push(ResidualShortlistedCitation {
                candidate: candidate.clone(),
                matched_criterion_refs: matched,
            });
        }
    }
    Ok(out)
}

pub const fn shortlist_membership_is_semantic_payment(
    _candidate: &ResidualShortlistedCitation,
) -> bool {
    false
}

pub const fn shortlist_membership_is_citation_treatment(
    _candidate: &ResidualShortlistedCitation,
) -> bool {
    false
}

pub const fn shortlist_membership_is_current_authority(
    _candidate: &ResidualShortlistedCitation,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(citation: &str, anchor: &str) -> CitationOccurrenceCandidate {
        CitationOccurrenceCandidate {
            document_ref: "document:hca:[2026]-HCA-19:docx".into(),
            source_revision_ref: "source-revision:fixture".into(),
            canonical_text_sha256: "sha256:fixture".into(),
            paragraph_ordinal: 1,
            paragraph_locator_ref: "document:hca:[2026]-HCA-19:docx#footnote-1".into(),
            reported_paragraph_label: Some("footnote:1".into()),
            citation_text: citation.into(),
            paragraph_text: citation.into(),
            anchor_paragraph_locator_refs: vec!["document:hca:[2026]-HCA-19:docx#paragraph-64".into()],
            anchor_paragraph_texts: vec![anchor.into()],
            lexical_treatment_hints: vec![],
            reviewed: false,
            candidate_only: true,
        }
    }

    fn demand() -> ResidualCitationReviewDemand {
        ResidualCitationReviewDemand {
            residual_ref: "residual:cullen-positive-operational-act".into(),
            proposition_ref: "prop:cullen-positive-operational-duty".into(),
            criteria: vec![ResidualAnchorCriterion {
                criterion_ref: "criterion:positive-act-vs-omission".into(),
                residual_ref: "residual:cullen-positive-operational-act".into(),
                proposition_ref: "prop:cullen-positive-operational-duty".into(),
                required_anchor_phrases: vec![
                    "positive acts in creating risk".into(),
                    "omission to act".into(),
                ],
            }],
        }
    }

    #[test]
    fn anchored_context_can_shortlist_without_promoting_semantics() {
        let candidates = vec![
            candidate(
                "[2018] AC 736",
                "liability would be based upon their positive acts in creating risk, not upon any alleged omission to act",
            ),
            candidate(
                "(2024) 98 ALJR 956",
                "salient features should be considered in a different doctrinal lane",
            ),
        ];
        let shortlisted = shortlist_anchored_citations_for_residual(&candidates, &demand()).unwrap();
        assert_eq!(shortlisted.len(), 1);
        assert_eq!(shortlisted[0].candidate.citation_text, "[2018] AC 736");
        assert_eq!(
            shortlisted[0].matched_criterion_refs,
            vec!["criterion:positive-act-vs-omission"]
        );
        assert!(!shortlist_membership_is_semantic_payment(&shortlisted[0]));
        assert!(!shortlist_membership_is_citation_treatment(&shortlisted[0]));
        assert!(!shortlist_membership_is_current_authority(&shortlisted[0]));
    }

    #[test]
    fn criterion_cannot_be_reused_for_another_residual() {
        let mut bad = demand();
        bad.criteria[0].residual_ref = "residual:other".into();
        assert_eq!(
            shortlist_anchored_citations_for_residual(&[], &bad),
            Err(ResidualReviewShortlistError::CriterionResidualMismatch)
        );
    }
}
