//! Explicit review boundary for one GWB ambiguity-directed hop.
//!
//! A pending bundle is evidence for review, not review authority. A real
//! assignment is bound to the exact post-diagnosis frontier, selected move,
//! source manifestation, and evidence digest.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::gwb_ambiguity_campaign::{
    GwbCompiledAmbiguityFrontier, GwbInvestigationCandidate,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GwbReviewOutcome {
    Resolved,
    SameObject,
    NewRelatedObject,
    NewConceptualParent,
    SharedSuperclass,
    BridgeClass,
    ConditionalDistinction,
    NewEvidentiarySource,
    WrongType,
    Duplicate,
    IrrelevantToResidual,
    Empty,
    NoSupport,
    Abstain,
}

impl GwbReviewOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Resolved => "resolved",
            Self::SameObject => "same-object",
            Self::NewRelatedObject => "new-related-object",
            Self::NewConceptualParent => "new-conceptual-parent",
            Self::SharedSuperclass => "shared-superclass",
            Self::BridgeClass => "bridge-class",
            Self::ConditionalDistinction => "conditional-distinction",
            Self::NewEvidentiarySource => "new-evidentiary-source",
            Self::WrongType => "wrong-type",
            Self::Duplicate => "duplicate",
            Self::IrrelevantToResidual => "irrelevant-to-residual",
            Self::Empty => "empty",
            Self::NoSupport => "no-support",
            Self::Abstain => "abstain",
        }
    }

    const fn is_negative(self) -> bool {
        matches!(
            self,
            Self::WrongType
                | Self::Duplicate
                | Self::IrrelevantToResidual
                | Self::Empty
                | Self::NoSupport
                | Self::Abstain
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GwbResidualEffect {
    KeepOpen,
    CloseReviewed,
}

impl GwbResidualEffect {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KeepOpen => "keep-open",
            Self::CloseReviewed => "close-reviewed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbReviewAssignment {
    pub hop_index: usize,
    pub frontier_sha256: String,
    pub selected_move_ref: String,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
    pub outcome: GwbReviewOutcome,
    pub residual_effect: GwbResidualEffect,
    pub review_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedReviewedGwbHop {
    pub hop_index: usize,
    pub frontier_sha256: String,
    pub selected: GwbInvestigationCandidate,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
    pub outcome: GwbReviewOutcome,
    pub residual_effect: GwbResidualEffect,
    pub review_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GwbReviewParseError {
    #[error("GWB review manifest line {line_number} has {field_count} fields; expected 8")]
    InvalidFieldCount {
        line_number: usize,
        field_count: usize,
    },
    #[error("GWB review manifest line {line_number} has an empty {field_name}")]
    EmptyField {
        line_number: usize,
        field_name: &'static str,
    },
    #[error("invalid GWB hop index: {0}")]
    InvalidHopIndex(String),
    #[error("invalid GWB frontier sha256: {0}")]
    InvalidFrontierDigest(String),
    #[error("invalid GWB evidence digest: {0}")]
    InvalidEvidenceDigest(String),
    #[error("invalid GWB review outcome: {0}")]
    InvalidOutcome(String),
    #[error("invalid GWB residual effect: {0}")]
    InvalidResidualEffect(String),
    #[error("conflicting GWB review assignment for hop {hop_index}, move {selected_move_ref}")]
    ConflictingAssignment {
        hop_index: usize,
        selected_move_ref: String,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GwbReviewPreparationError {
    #[error("review hop mismatch: expected {expected}, observed {observed}")]
    HopMismatch { expected: usize, observed: usize },
    #[error("review frontier digest mismatch: expected {expected}, observed {observed}")]
    FrontierMismatch { expected: String, observed: String },
    #[error("review selected move mismatch: expected {expected}, observed {observed}")]
    MoveMismatch { expected: String, observed: String },
    #[error("review source revision mismatch: expected {expected}, observed {observed}")]
    SourceRevisionMismatch { expected: String, observed: String },
    #[error("review evidence digest mismatch: expected {expected}, observed {observed}")]
    EvidenceDigestMismatch { expected: String, observed: String },
    #[error("negative/empty GWB outcome may not close its residual")]
    NegativeOutcomeMayNotCloseResidual,
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hash_bool(hasher: &mut Sha256, value: bool) {
    hash_field(hasher, if value { "true" } else { "false" });
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_sha256_ref(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[must_use]
pub fn gwb_frontier_sha256(compiled: &GwbCompiledAmbiguityFrontier) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"gwb-ambiguity-frontier:v1\0");
    hash_field(&mut hasher, &compiled.frontier.consumer_ref);
    hash_field(&mut hasher, &compiled.frontier.frontier_ref);
    hash_field(&mut hasher, compiled.frontier.authority);

    let mut residuals = compiled.frontier.residuals.clone();
    residuals.sort();
    for residual in residuals {
        hash_field(&mut hasher, "residual");
        hash_field(&mut hasher, &residual.residual_ref);
        hash_field(&mut hasher, &residual.proposition_ref);
        hash_field(&mut hasher, &residual.producer_class_ref);
        hash_field(&mut hasher, &residual.salience.to_string());
        let mut dependencies = residual.dependency_refs.clone();
        dependencies.sort();
        dependencies.dedup();
        for dependency in dependencies {
            hash_field(&mut hasher, &dependency);
        }
    }

    for investigation in compiled.investigations.values() {
        hash_field(&mut hasher, "investigation");
        hash_field(&mut hasher, &investigation.move_ref);
        hash_field(&mut hasher, investigation.investigation_kind.as_str());
        hash_field(&mut hasher, &investigation.producer_ref);
        hash_field(
            &mut hasher,
            investigation.source_ref.as_deref().unwrap_or(""),
        );
        hash_field(
            &mut hasher,
            investigation.source_revision_ref.as_deref().unwrap_or(""),
        );
        hash_field(
            &mut hasher,
            investigation.target_ref.as_deref().unwrap_or(""),
        );
        hash_field(
            &mut hasher,
            investigation.property_ref.as_deref().unwrap_or(""),
        );
        hash_field(
            &mut hasher,
            &investigation.expected_residual_contraction.to_string(),
        );
        hash_field(&mut hasher, &investigation.ambiguity_reduction.to_string());
        hash_field(&mut hasher, &investigation.type_closure_gain.to_string());
        hash_field(
            &mut hasher,
            &investigation.cross_surface_gap_gain.to_string(),
        );
        hash_field(&mut hasher, &investigation.source_support_gain.to_string());
        hash_field(
            &mut hasher,
            &investigation.shared_dependency_gain.to_string(),
        );
        hash_field(&mut hasher, &investigation.network_requests.to_string());
        hash_bool(&mut hasher, investigation.admissible);
    }

    format!("sha256:{}", hex(&hasher.finalize()))
}

/// Render a stable, entirely-commented review bundle. Feeding it back to the
/// manifest parser produces zero review assignments.
pub fn pending_gwb_review_bundle(
    hop_index: usize,
    compiled: &GwbCompiledAmbiguityFrontier,
    selected: &GwbInvestigationCandidate,
    source_revision_ref: &str,
    evidence_digest_ref: &str,
) -> Result<String, GwbReviewParseError> {
    if source_revision_ref.trim().is_empty() {
        return Err(GwbReviewParseError::EmptyField {
            line_number: 0,
            field_name: "source_revision_ref",
        });
    }
    if !valid_sha256_ref(evidence_digest_ref) {
        return Err(GwbReviewParseError::InvalidEvidenceDigest(
            evidence_digest_ref.to_owned(),
        ));
    }
    let frontier = gwb_frontier_sha256(compiled);
    let mut out = String::new();
    out.push_str("# status=GwbReviewRequired\n");
    out.push_str(&format!("# hop_index={hop_index}\n"));
    out.push_str(&format!("# frontier_sha256={frontier}\n"));
    out.push_str(&format!("# selected_move={}\n", selected.move_ref));
    out.push_str(&format!(
        "# investigation_kind={}\n",
        selected.investigation_kind.as_str()
    ));
    out.push_str(&format!(
        "# selected_source={}\n",
        selected.source_ref.as_deref().unwrap_or("")
    ));
    out.push_str(&format!(
        "# selected_target={}\n",
        selected.target_ref.as_deref().unwrap_or("")
    ));
    out.push_str(&format!(
        "# selected_property={}\n",
        selected.property_ref.as_deref().unwrap_or("")
    ));
    out.push_str(&format!("# source_revision={source_revision_ref}\n"));
    out.push_str(&format!("# evidence_digest={evidence_digest_ref}\n"));
    out.push_str(&format!(
        "# target_residuals={}\n",
        selected.target_residual_refs.join(",")
    ));
    out.push_str(&format!(
        "# review_manifest_template\t{hop_index}\t{frontier}\t{}\t{source_revision_ref}\t{evidence_digest_ref}\t<outcome>\t<residual-effect>\t<review-ref>\n",
        selected.move_ref
    ));
    Ok(out)
}

fn parse_outcome(value: &str) -> Option<GwbReviewOutcome> {
    match value {
        "resolved" => Some(GwbReviewOutcome::Resolved),
        "same-object" => Some(GwbReviewOutcome::SameObject),
        "new-related-object" => Some(GwbReviewOutcome::NewRelatedObject),
        "new-conceptual-parent" => Some(GwbReviewOutcome::NewConceptualParent),
        "shared-superclass" => Some(GwbReviewOutcome::SharedSuperclass),
        "bridge-class" => Some(GwbReviewOutcome::BridgeClass),
        "conditional-distinction" => Some(GwbReviewOutcome::ConditionalDistinction),
        "new-evidentiary-source" => Some(GwbReviewOutcome::NewEvidentiarySource),
        "wrong-type" => Some(GwbReviewOutcome::WrongType),
        "duplicate" => Some(GwbReviewOutcome::Duplicate),
        "irrelevant-to-residual" => Some(GwbReviewOutcome::IrrelevantToResidual),
        "empty" => Some(GwbReviewOutcome::Empty),
        "no-support" => Some(GwbReviewOutcome::NoSupport),
        "abstain" => Some(GwbReviewOutcome::Abstain),
        _ => None,
    }
}

fn parse_residual_effect(value: &str) -> Option<GwbResidualEffect> {
    match value {
        "keep-open" => Some(GwbResidualEffect::KeepOpen),
        "close-reviewed" => Some(GwbResidualEffect::CloseReviewed),
        _ => None,
    }
}

pub fn parse_gwb_review_tsv(
    input: &str,
) -> Result<Vec<GwbReviewAssignment>, GwbReviewParseError> {
    let mut assignments = BTreeMap::new();
    for (offset, raw_line) in input.lines().enumerate() {
        let line_number = offset + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').map(str::trim).collect::<Vec<_>>();
        if fields.len() != 8 {
            return Err(GwbReviewParseError::InvalidFieldCount {
                line_number,
                field_count: fields.len(),
            });
        }
        for (field_name, value) in [
            ("hop_index", fields[0]),
            ("frontier_sha256", fields[1]),
            ("selected_move_ref", fields[2]),
            ("source_revision_ref", fields[3]),
            ("evidence_digest_ref", fields[4]),
            ("outcome", fields[5]),
            ("residual_effect", fields[6]),
            ("review_ref", fields[7]),
        ] {
            if value.is_empty() {
                return Err(GwbReviewParseError::EmptyField {
                    line_number,
                    field_name,
                });
            }
        }
        let hop_index = fields[0]
            .parse::<usize>()
            .map_err(|_| GwbReviewParseError::InvalidHopIndex(fields[0].to_owned()))?;
        if !valid_sha256_ref(fields[1]) {
            return Err(GwbReviewParseError::InvalidFrontierDigest(
                fields[1].to_owned(),
            ));
        }
        if !valid_sha256_ref(fields[4]) {
            return Err(GwbReviewParseError::InvalidEvidenceDigest(
                fields[4].to_owned(),
            ));
        }
        let outcome = parse_outcome(fields[5])
            .ok_or_else(|| GwbReviewParseError::InvalidOutcome(fields[5].to_owned()))?;
        let residual_effect = parse_residual_effect(fields[6]).ok_or_else(|| {
            GwbReviewParseError::InvalidResidualEffect(fields[6].to_owned())
        })?;
        let assignment = GwbReviewAssignment {
            hop_index,
            frontier_sha256: fields[1].to_ascii_lowercase(),
            selected_move_ref: fields[2].to_owned(),
            source_revision_ref: fields[3].to_owned(),
            evidence_digest_ref: fields[4].to_ascii_lowercase(),
            outcome,
            residual_effect,
            review_ref: fields[7].to_owned(),
        };
        let key = (assignment.hop_index, assignment.selected_move_ref.clone());
        if let Some(existing) = assignments.get(&key) {
            if existing != &assignment {
                return Err(GwbReviewParseError::ConflictingAssignment {
                    hop_index: assignment.hop_index,
                    selected_move_ref: assignment.selected_move_ref,
                });
            }
            continue;
        }
        assignments.insert(key, assignment);
    }
    Ok(assignments.into_values().collect())
}

pub fn prepare_reviewed_gwb_hop(
    hop_index: usize,
    compiled: &GwbCompiledAmbiguityFrontier,
    selected: &GwbInvestigationCandidate,
    source_revision_ref: &str,
    evidence_digest_ref: &str,
    assignment: &GwbReviewAssignment,
) -> Result<PreparedReviewedGwbHop, GwbReviewPreparationError> {
    if assignment.hop_index != hop_index {
        return Err(GwbReviewPreparationError::HopMismatch {
            expected: hop_index,
            observed: assignment.hop_index,
        });
    }
    let frontier = gwb_frontier_sha256(compiled);
    if assignment.frontier_sha256 != frontier {
        return Err(GwbReviewPreparationError::FrontierMismatch {
            expected: frontier,
            observed: assignment.frontier_sha256.clone(),
        });
    }
    if assignment.selected_move_ref != selected.move_ref {
        return Err(GwbReviewPreparationError::MoveMismatch {
            expected: selected.move_ref.clone(),
            observed: assignment.selected_move_ref.clone(),
        });
    }
    if assignment.source_revision_ref != source_revision_ref {
        return Err(GwbReviewPreparationError::SourceRevisionMismatch {
            expected: source_revision_ref.to_owned(),
            observed: assignment.source_revision_ref.clone(),
        });
    }
    if assignment.evidence_digest_ref != evidence_digest_ref {
        return Err(GwbReviewPreparationError::EvidenceDigestMismatch {
            expected: evidence_digest_ref.to_owned(),
            observed: assignment.evidence_digest_ref.clone(),
        });
    }
    if assignment.outcome.is_negative()
        && assignment.residual_effect == GwbResidualEffect::CloseReviewed
    {
        return Err(GwbReviewPreparationError::NegativeOutcomeMayNotCloseResidual);
    }

    Ok(PreparedReviewedGwbHop {
        hop_index,
        frontier_sha256: assignment.frontier_sha256.clone(),
        selected: selected.clone(),
        source_revision_ref: source_revision_ref.to_owned(),
        evidence_digest_ref: evidence_digest_ref.to_owned(),
        outcome: assignment.outcome,
        residual_effect: assignment.residual_effect,
        review_ref: assignment.review_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
