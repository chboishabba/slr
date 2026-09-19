//! Explicit review manifests for one exact parsed bounded-context source.
//!
//! A context review is bound to `(source revision, deterministic candidate-set
//! digest)`. It cannot be replayed against a different Wikidata manifestation
//! or against a changed bounded candidate set.

use std::collections::BTreeMap;

use sensiblaw_pg_source_store::{
    review_bounded_wikidata_candidate, ContextReviewDecision, ReviewedContextEdge,
    ReviewedSourceExpansionInput,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::adaptive_campaign::ParsedBoundedContextCandidate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextExpansionReviewAssignment {
    pub source_revision_ref: String,
    pub candidate_set_sha256: String,
    pub review_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedReviewedContextExpansion {
    pub edges: Vec<ReviewedContextEdge>,
    pub expansion: ReviewedSourceExpansionInput,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ContextExpansionReviewError {
    #[error("context review manifest line {line_number} has {field_count} fields; expected 3")]
    InvalidFieldCount {
        line_number: usize,
        field_count: usize,
    },
    #[error("context review manifest line {line_number} has an empty {field_name}")]
    EmptyField {
        line_number: usize,
        field_name: &'static str,
    },
    #[error("invalid candidate-set sha256: {0}")]
    InvalidDigest(String),
    #[error("conflicting context review assignment for source revision {source_revision_ref}")]
    ConflictingAssignment { source_revision_ref: String },
    #[error("invalid exact Wikidata source revision: {0}")]
    InvalidSourceRevision(String),
    #[error("candidate source revision does not match reviewed source revision")]
    CandidateSourceRevisionMismatch,
    #[error("candidate source QID does not match reviewed source revision")]
    CandidateSourceMismatch,
    #[error("context candidate set digest mismatch: expected {expected}, observed {observed}")]
    CandidateSetDigestMismatch { expected: String, observed: String },
    #[error("bounded context review failed: {0}")]
    BoundedReview(String),
}

fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn source_qid_from_revision(source_revision_ref: &str) -> Result<String, ContextExpansionReviewError> {
    let fields = source_revision_ref.split(':').collect::<Vec<_>>();
    let valid = fields.len() == 4
        && fields[0] == "wikidata"
        && fields[1]
            .strip_prefix('Q')
            .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        && fields[2] == "oldid"
        && fields[3]
            .parse::<u64>()
            .is_ok_and(|revision_id| revision_id > 0);
    if !valid {
        return Err(ContextExpansionReviewError::InvalidSourceRevision(
            source_revision_ref.to_owned(),
        ));
    }
    Ok(fields[1].to_owned())
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Deterministic digest of the entire finite bounded candidate set for one
/// exact source manifestation. Candidate ordering is deliberately irrelevant.
pub fn bounded_context_candidate_set_sha256(
    source_revision_ref: &str,
    candidates: &[ParsedBoundedContextCandidate],
) -> Result<String, ContextExpansionReviewError> {
    let source_qid = source_qid_from_revision(source_revision_ref)?;
    let mut canonical = candidates.to_vec();
    canonical.sort();
    canonical.dedup();

    for candidate in &canonical {
        if candidate.source_revision_ref != source_revision_ref {
            return Err(ContextExpansionReviewError::CandidateSourceRevisionMismatch);
        }
        if candidate.source_qid != source_qid {
            return Err(ContextExpansionReviewError::CandidateSourceMismatch);
        }
    }

    let mut hasher = Sha256::new();
    hasher.update(b"mabo-bounded-context-candidate-set:v1\0");
    hasher.update(source_revision_ref.as_bytes());
    hasher.update([0]);
    for candidate in &canonical {
        for value in [
            candidate.candidate_id.as_str(),
            candidate.source_qid.as_str(),
            candidate.target_qid.as_str(),
            candidate.property_ref.as_str(),
        ] {
            hasher.update(value.as_bytes());
            hasher.update([0]);
        }
    }
    Ok(hex_digest(&hasher.finalize()))
}

/// Format:
/// `source_revision_ref<TAB>candidate_set_sha256<TAB>review_ref`.
pub fn parse_mabo_context_review_tsv(
    input: &str,
) -> Result<Vec<ContextExpansionReviewAssignment>, ContextExpansionReviewError> {
    let mut assignments: BTreeMap<String, ContextExpansionReviewAssignment> = BTreeMap::new();
    for (offset, raw_line) in input.lines().enumerate() {
        let line_number = offset + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(ContextExpansionReviewError::InvalidFieldCount {
                line_number,
                field_count: fields.len(),
            });
        }
        for (field_name, value) in [
            ("source_revision_ref", fields[0]),
            ("candidate_set_sha256", fields[1]),
            ("review_ref", fields[2]),
        ] {
            if value.is_empty() {
                return Err(ContextExpansionReviewError::EmptyField {
                    line_number,
                    field_name,
                });
            }
        }
        source_qid_from_revision(fields[0])?;
        if !valid_sha256_hex(fields[1]) {
            return Err(ContextExpansionReviewError::InvalidDigest(fields[1].to_owned()));
        }
        let assignment = ContextExpansionReviewAssignment {
            source_revision_ref: fields[0].to_owned(),
            candidate_set_sha256: fields[1].to_ascii_lowercase(),
            review_ref: fields[2].to_owned(),
        };
        if let Some(existing) = assignments.get(&assignment.source_revision_ref) {
            if existing != &assignment {
                return Err(ContextExpansionReviewError::ConflictingAssignment {
                    source_revision_ref: assignment.source_revision_ref,
                });
            }
            continue;
        }
        assignments.insert(assignment.source_revision_ref.clone(), assignment);
    }
    Ok(assignments.into_values().collect())
}

pub fn matching_context_review<'a>(
    source_revision_ref: &str,
    candidate_set_sha256: &str,
    assignments: &'a [ContextExpansionReviewAssignment],
) -> Option<&'a ContextExpansionReviewAssignment> {
    assignments.iter().find(|assignment| {
        assignment.source_revision_ref == source_revision_ref
            && assignment.candidate_set_sha256 == candidate_set_sha256
    })
}

/// Verify a context-set review and prepare both the reviewed edges and the
/// durable source-expansion receipt. The latter closes even an empty bounded
/// candidate set, preventing endless reacquisition on restart.
pub fn prepare_reviewed_context_expansion(
    candidates: &[ParsedBoundedContextCandidate],
    assignment: &ContextExpansionReviewAssignment,
) -> Result<PreparedReviewedContextExpansion, ContextExpansionReviewError> {
    let source_qid = source_qid_from_revision(&assignment.source_revision_ref)?;
    let observed = bounded_context_candidate_set_sha256(&assignment.source_revision_ref, candidates)?;
    if observed != assignment.candidate_set_sha256 {
        return Err(ContextExpansionReviewError::CandidateSetDigestMismatch {
            expected: assignment.candidate_set_sha256.clone(),
            observed,
        });
    }

    let mut canonical = candidates.to_vec();
    canonical.sort();
    canonical.dedup();
    let edges = canonical
        .iter()
        .map(|candidate| {
            review_bounded_wikidata_candidate(
                candidate.source_revision_ref.clone(),
                candidate.candidate_id.clone(),
                candidate.source_qid.clone(),
                candidate.target_qid.clone(),
                candidate.property_ref.clone(),
                ContextReviewDecision::Reviewed,
            )
            .map_err(|error| ContextExpansionReviewError::BoundedReview(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PreparedReviewedContextExpansion {
        edges,
        expansion: ReviewedSourceExpansionInput {
            source_ref: source_qid,
            source_revision_ref: assignment.source_revision_ref.clone(),
            review_ref: assignment.review_ref.clone(),
            bounded_candidate_count: canonical.len(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
    })
}
