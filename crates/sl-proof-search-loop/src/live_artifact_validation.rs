//! Rust-native validation for retained Cullen research artifacts.
//!
//! These checks replace the Python validators formerly used by the R6/R7 lane.
//! They operate on typed Rust values before serialization, so a failing invariant
//! stops artifact production rather than validating it in a second language.

use crate::judgment_candidates::CitationOccurrenceCandidate;
use crate::residual_review_shortlist::ResidualShortlistedCitation;

pub const ROBINSON_REPORTED: &str = "[2018] AC 736";
pub const MODBURY_REPORTED: &str = "(2000) 205 CLR 254";
pub const MALLONLAND_REPORTED: &str = "(2024) 98 ALJR 956";
pub const MALLONLAND_PARALLEL: &str = "418 ALR 639";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveArtifactValidationError {
    BodyAndRefinedDigestCollapsed,
    NoFootnotes,
    NoAnchors,
    TooFewCandidates,
    CandidateAlreadyReviewed,
    CandidateNotCandidateOnly,
    CandidateLostCanonicalDigest,
    CandidateLostSourceRevision,
    CandidateMissingLocator,
    CandidateMissingCitation,
    AnchorArityMismatch,
    NoAnchoredFootnoteCandidate,
    MallonlandCalibrationMissing,
    RobinsonPositiveActAnchorMissing,
    ModburyPositiveActAnchorMissing,
    UnexpectedInputCandidateCount,
    EmptyShortlist,
    ShortlistItemAlreadyReviewed,
    ShortlistItemNotCandidateOnly,
    ShortlistItemMissingCriterion,
    ShortlistItemMissingAnchor,
    RobinsonMissingFromShortlist,
    ModburyMissingFromShortlist,
    MallonlandIncorrectlyShortlisted,
    RobinsonShortlistContextMissing,
    ModburyShortlistContextMissing,
}

pub fn validate_cullen_review_queue_values(
    body_only_digest: &str,
    refined_digest: &str,
    source_revision_ref: &str,
    footnote_count: usize,
    anchor_count: usize,
    candidates: &[CitationOccurrenceCandidate],
) -> Result<(), LiveArtifactValidationError> {
    if body_only_digest == refined_digest {
        return Err(LiveArtifactValidationError::BodyAndRefinedDigestCollapsed);
    }
    if footnote_count == 0 {
        return Err(LiveArtifactValidationError::NoFootnotes);
    }
    if anchor_count == 0 {
        return Err(LiveArtifactValidationError::NoAnchors);
    }
    if candidates.len() <= 1 {
        return Err(LiveArtifactValidationError::TooFewCandidates);
    }

    let mut anchored_footnote = false;
    let mut mallonland_reported = false;
    let mut mallonland_parallel = false;
    let mut robinson_positive = false;
    let mut modbury_positive = false;

    for candidate in candidates {
        if candidate.reviewed {
            return Err(LiveArtifactValidationError::CandidateAlreadyReviewed);
        }
        if !candidate.candidate_only {
            return Err(LiveArtifactValidationError::CandidateNotCandidateOnly);
        }
        if candidate.canonical_text_sha256 != refined_digest {
            return Err(LiveArtifactValidationError::CandidateLostCanonicalDigest);
        }
        if candidate.source_revision_ref != source_revision_ref {
            return Err(LiveArtifactValidationError::CandidateLostSourceRevision);
        }
        if candidate.paragraph_locator_ref.is_empty() {
            return Err(LiveArtifactValidationError::CandidateMissingLocator);
        }
        if candidate.citation_text.is_empty() {
            return Err(LiveArtifactValidationError::CandidateMissingCitation);
        }
        if candidate.anchor_paragraph_locator_refs.len() != candidate.anchor_paragraph_texts.len() {
            return Err(LiveArtifactValidationError::AnchorArityMismatch);
        }
        if candidate.paragraph_locator_ref.contains("#footnote-")
            && !candidate.anchor_paragraph_locator_refs.is_empty()
        {
            anchored_footnote = true;
        }
        if candidate.citation_text == MALLONLAND_REPORTED {
            mallonland_reported = true;
        }
        if candidate.citation_text == MALLONLAND_PARALLEL {
            mallonland_parallel = true;
        }
        let context = candidate.anchor_paragraph_texts.join(" ").to_ascii_lowercase();
        if candidate.citation_text == ROBINSON_REPORTED
            && context.contains("positive acts in creating risk")
        {
            robinson_positive = true;
        }
        if candidate.citation_text == MODBURY_REPORTED
            && context.contains("positive acts in creating risk")
        {
            modbury_positive = true;
        }
    }

    if !anchored_footnote {
        return Err(LiveArtifactValidationError::NoAnchoredFootnoteCandidate);
    }
    if !mallonland_reported || !mallonland_parallel {
        return Err(LiveArtifactValidationError::MallonlandCalibrationMissing);
    }
    if !robinson_positive {
        return Err(LiveArtifactValidationError::RobinsonPositiveActAnchorMissing);
    }
    if !modbury_positive {
        return Err(LiveArtifactValidationError::ModburyPositiveActAnchorMissing);
    }
    Ok(())
}

pub fn validate_cullen_shortlist_values(
    input_candidate_count: usize,
    shortlist: &[ResidualShortlistedCitation],
) -> Result<(), LiveArtifactValidationError> {
    if input_candidate_count != 190 {
        return Err(LiveArtifactValidationError::UnexpectedInputCandidateCount);
    }
    if shortlist.is_empty() {
        return Err(LiveArtifactValidationError::EmptyShortlist);
    }

    let mut robinson_contexts = Vec::new();
    let mut modbury_contexts = Vec::new();
    let mut robinson_seen = false;
    let mut modbury_seen = false;

    for item in shortlist {
        let candidate = &item.candidate;
        if candidate.reviewed {
            return Err(LiveArtifactValidationError::ShortlistItemAlreadyReviewed);
        }
        if !candidate.candidate_only {
            return Err(LiveArtifactValidationError::ShortlistItemNotCandidateOnly);
        }
        if item.matched_criterion_refs.is_empty() {
            return Err(LiveArtifactValidationError::ShortlistItemMissingCriterion);
        }
        if candidate.anchor_paragraph_texts.is_empty()
            || candidate.anchor_paragraph_locator_refs.is_empty()
        {
            return Err(LiveArtifactValidationError::ShortlistItemMissingAnchor);
        }
        match candidate.citation_text.as_str() {
            ROBINSON_REPORTED => {
                robinson_seen = true;
                robinson_contexts.extend(candidate.anchor_paragraph_texts.iter().cloned());
            }
            MODBURY_REPORTED => {
                modbury_seen = true;
                modbury_contexts.extend(candidate.anchor_paragraph_texts.iter().cloned());
            }
            MALLONLAND_REPORTED => {
                return Err(LiveArtifactValidationError::MallonlandIncorrectlyShortlisted)
            }
            _ => {}
        }
    }

    if !robinson_seen {
        return Err(LiveArtifactValidationError::RobinsonMissingFromShortlist);
    }
    if !modbury_seen {
        return Err(LiveArtifactValidationError::ModburyMissingFromShortlist);
    }

    let robinson = robinson_contexts.join(" ").to_ascii_lowercase();
    if !robinson.contains("positive negligent conduct")
        && !robinson.contains("positive acts in creating risk")
    {
        return Err(LiveArtifactValidationError::RobinsonShortlistContextMissing);
    }

    let modbury = modbury_contexts.join(" ").to_ascii_lowercase();
    if !modbury.contains("careless acts causing personal injury")
        && !modbury.contains("positive acts in creating risk")
    {
        return Err(LiveArtifactValidationError::ModburyShortlistContextMissing);
    }
    Ok(())
}
