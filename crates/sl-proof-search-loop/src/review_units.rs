//! Deterministic quotienting of residual citation occurrences into review units.
//!
//! A unit groups exact duplicate observations of one citation at the same
//! preserved body-anchor context.  The occurrence locators remain in the
//! unit, so multiplicity is retained even when one review decision can cover
//! several observations.  Different anchor contexts never merge merely
//! because they cite the same authority.

use crate::residual_review_shortlist::ResidualShortlistedCitation;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationReviewUnit {
    pub review_unit_ref: String,
    pub citation_text: String,
    pub source_revision_ref: String,
    pub citation_locator_refs: Vec<String>,
    pub anchor_paragraph_locator_refs: Vec<String>,
    pub anchor_paragraph_texts: Vec<String>,
    pub matched_criterion_refs: Vec<String>,
    pub reviewed: bool,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReviewUnitKey {
    citation_text: String,
    source_revision_ref: String,
    anchor_locator_refs: Vec<String>,
    anchor_texts: Vec<String>,
}

fn unit_ref(key: &ReviewUnitKey) -> String {
    let mut bytes = Vec::new();
    for value in [
        &key.citation_text,
        &key.source_revision_ref,
        &key.anchor_locator_refs.join("\u{1f}"),
        &key.anchor_texts.join("\u{1f}"),
    ] {
        bytes.extend_from_slice(value.len().to_string().as_bytes());
        bytes.push(b':');
        bytes.extend_from_slice(value.as_bytes());
    }
    format!("review-unit:sha256:{:x}", Sha256::digest(bytes))
}

/// Quotient shortlisted occurrences by exact authority + body-anchor context.
///
/// All input occurrences are retained as locator refs.  This is a review
/// scheduling quotient only: it does not compile a treatment, reasoning edge,
/// semantic payment, authority status or consumer closure.
pub fn cluster_shortlisted_citations(
    shortlisted: &[ResidualShortlistedCitation],
) -> Vec<CitationReviewUnit> {
    let mut grouped: BTreeMap<ReviewUnitKey, CitationReviewUnit> = BTreeMap::new();
    for item in shortlisted {
        let candidate = &item.candidate;
        let key = ReviewUnitKey {
            citation_text: candidate.citation_text.clone(),
            source_revision_ref: candidate.source_revision_ref.clone(),
            anchor_locator_refs: candidate.anchor_paragraph_locator_refs.clone(),
            anchor_texts: candidate.anchor_paragraph_texts.clone(),
        };
        let entry = grouped.entry(key.clone()).or_insert_with(|| CitationReviewUnit {
            review_unit_ref: unit_ref(&key),
            citation_text: key.citation_text.clone(),
            source_revision_ref: key.source_revision_ref.clone(),
            citation_locator_refs: Vec::new(),
            anchor_paragraph_locator_refs: key.anchor_locator_refs.clone(),
            anchor_paragraph_texts: key.anchor_texts.clone(),
            matched_criterion_refs: Vec::new(),
            reviewed: false,
            candidate_only: true,
        });
        let occurrence_ref = format!("{}::{}", candidate.paragraph_locator_ref, candidate.citation_text);
        if !entry.citation_locator_refs.contains(&occurrence_ref) {
            entry.citation_locator_refs.push(occurrence_ref);
        }
        entry
            .matched_criterion_refs
            .extend(item.matched_criterion_refs.iter().cloned());
        entry.matched_criterion_refs.sort();
        entry.matched_criterion_refs.dedup();
    }
    for unit in grouped.values_mut() {
        unit.citation_locator_refs.sort();
    }
    grouped.into_values().collect()
}

pub const fn review_unit_is_semantic_payment(_unit: &CitationReviewUnit) -> bool {
    false
}

pub const fn review_unit_is_citation_treatment(_unit: &CitationReviewUnit) -> bool {
    false
}

pub const fn review_unit_is_current_authority(_unit: &CitationReviewUnit) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judgment_candidates::CitationOccurrenceCandidate;

    fn item(citation: &str, locator: &str, anchor: &str) -> ResidualShortlistedCitation {
        ResidualShortlistedCitation {
            candidate: CitationOccurrenceCandidate {
                document_ref: "document:cullen".into(),
                source_revision_ref: "source:revision-1".into(),
                canonical_text_sha256: "sha256:text".into(),
                paragraph_ordinal: 1,
                paragraph_locator_ref: locator.into(),
                reported_paragraph_label: None,
                citation_text: citation.into(),
                paragraph_text: citation.into(),
                anchor_paragraph_locator_refs: vec![anchor.into()],
                anchor_paragraph_texts: vec!["positive acts in creating risk".into()],
                lexical_treatment_hints: vec![],
                reviewed: false,
                candidate_only: true,
            },
            matched_criterion_refs: vec!["criterion:positive-act".into()],
        }
    }

    #[test]
    fn duplicate_occurrences_share_a_unit_and_retain_locators() {
        let units = cluster_shortlisted_citations(&[
            item("[2018] AC 736", "document:cullen#footnote-43", "document:cullen#paragraph-89"),
            item("[2018] AC 736", "document:cullen#footnote-44", "document:cullen#paragraph-89"),
        ]);
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].citation_locator_refs.len(), 2);
        assert!(units[0].candidate_only);
        assert!(!review_unit_is_semantic_payment(&units[0]));
    }

    #[test]
    fn same_authority_at_different_body_anchors_stays_separate() {
        let units = cluster_shortlisted_citations(&[
            item("[2018] AC 736", "document:cullen#footnote-43", "document:cullen#paragraph-89"),
            item("[2018] AC 736", "document:cullen#footnote-105", "document:cullen#paragraph-148"),
        ]);
        assert_eq!(units.len(), 2);
        assert_ne!(units[0].review_unit_ref, units[1].review_unit_ref);
    }
}
