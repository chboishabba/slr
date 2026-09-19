//! Deterministic receipts for adaptive Mabo trajectory selection.
//!
//! These receipts are runtime evidence only. They prove neither claim truth nor
//! legal authority. Their purpose is to make the recurrence topology auditable:
//! a selection for cycle i+1 must name the committed cycle i transition as its
//! predecessor and must be bound to the freshly reconstructed world/frontier
//! digests observed after that commit.

use sensiblaw_pg_source_store::{LatentWorldEdgeRow, LatentWorldRows};
use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const POST_COMMIT_SELECTION_ORIGIN: &str = "post-commit-current-world-diagnosis";

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update(value.as_bytes());
    hasher.update([0]);
}

fn hash_bool(hasher: &mut Sha256, value: bool) {
    hash_field(hasher, if value { "true" } else { "false" });
}

fn status_ref(status: ResidualStatus) -> &'static str {
    match status {
        ResidualStatus::Open => "open",
        ResidualStatus::SatisfiedCandidate => "satisfied-candidate",
        ResidualStatus::Contested => "contested",
        ResidualStatus::AuthorityBlocked => "authority-blocked",
        ResidualStatus::Underidentified => "underidentified",
    }
}

fn canonical_edge(mut edge: LatentWorldEdgeRow) -> LatentWorldEdgeRow {
    edge.provenance_refs.sort();
    edge.provenance_refs.dedup();
    edge
}

fn canonical_residual(mut residual: ProofResidual) -> ProofResidual {
    residual.dependency_refs.sort();
    residual.dependency_refs.dedup();
    residual
}

/// Deterministic digest of the bounded, already-persisted world view used for
/// one adaptive diagnosis. Set-like coordinates are sorted before hashing.
#[must_use]
pub fn latent_world_digest(world: &LatentWorldRows) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mabo-adaptive-latent-world:v1\0");
    hash_field(&mut hasher, &world.seed_ref);
    hash_field(&mut hasher, &world.requested_max_hops.to_string());
    hash_field(&mut hasher, &world.deepest_observed_hop.to_string());
    hash_bool(&mut hasher, world.frontier_exhausted);
    hash_bool(&mut hasher, world.creates_semantic_authority);
    hash_bool(&mut hasher, world.applicability_promoted);
    hash_bool(&mut hasher, world.claim_truth_promoted);

    let mut visited = world.visited_refs.clone();
    visited.sort();
    visited.dedup();
    for value in visited {
        hash_field(&mut hasher, "visited");
        hash_field(&mut hasher, &value);
    }

    let mut frontier = world.frontier_refs.clone();
    frontier.sort();
    frontier.dedup();
    for value in frontier {
        hash_field(&mut hasher, "frontier");
        hash_field(&mut hasher, &value);
    }

    let mut residuals = world.residual_refs.clone();
    residuals.sort();
    residuals.dedup();
    for value in residuals {
        hash_field(&mut hasher, "world-residual");
        hash_field(&mut hasher, &value);
    }

    let mut edges = world
        .edges
        .clone()
        .into_iter()
        .map(canonical_edge)
        .collect::<Vec<_>>();
    edges.sort_by(|left, right| {
        (
            left.from_ref.as_str(),
            left.to_ref.as_str(),
            left.relation_ref.as_str(),
            &left.provenance_refs,
        )
            .cmp(&(
                right.from_ref.as_str(),
                right.to_ref.as_str(),
                right.relation_ref.as_str(),
                &right.provenance_refs,
            ))
    });
    edges.dedup();
    for edge in edges {
        hash_field(&mut hasher, "edge");
        hash_field(&mut hasher, &edge.from_ref);
        hash_field(&mut hasher, &edge.to_ref);
        hash_field(&mut hasher, &edge.relation_ref);
        for provenance in edge.provenance_refs {
            hash_field(&mut hasher, &provenance);
        }
    }

    format!("sha256:{}", hex_digest(&hasher.finalize()))
}

/// Deterministic digest of the exact consumer-relative frontier supplied to the
/// Pareto selector.
#[must_use]
pub fn proof_frontier_digest(frontier: &ProofFrontier) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mabo-adaptive-proof-frontier:v1\0");
    hash_field(&mut hasher, &frontier.consumer_ref);
    hash_field(&mut hasher, &frontier.frontier_ref);
    hash_field(&mut hasher, frontier.authority);

    let mut residuals = frontier
        .residuals
        .clone()
        .into_iter()
        .map(canonical_residual)
        .collect::<Vec<_>>();
    residuals.sort();
    residuals.dedup();
    for residual in residuals {
        hash_field(&mut hasher, "residual");
        hash_field(&mut hasher, &residual.residual_ref);
        hash_field(&mut hasher, &residual.proposition_ref);
        hash_field(&mut hasher, &residual.producer_class_ref);
        hash_field(
            &mut hasher,
            residual.jurisdiction_ref.as_deref().unwrap_or(""),
        );
        hash_field(
            &mut hasher,
            residual.authority_requirement_ref.as_deref().unwrap_or(""),
        );
        hash_field(&mut hasher, &residual.salience.to_string());
        hash_field(&mut hasher, status_ref(residual.status));
        for dependency in residual.dependency_refs {
            hash_field(&mut hasher, &dependency);
        }
    }

    for (tag, values) in [
        ("satisfied", &frontier.satisfied_payment_refs),
        ("contested", &frontier.contested_coordinate_refs),
        ("authority-blocked", &frontier.authority_blocked_refs),
    ] {
        let mut canonical = values.clone();
        canonical.sort();
        canonical.dedup();
        for value in canonical {
            hash_field(&mut hasher, tag);
            hash_field(&mut hasher, &value);
        }
    }

    format!("sha256:{}", hex_digest(&hasher.finalize()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveSelectionReceipt {
    pub schema_version: String,
    pub cycle_index: usize,
    pub world_digest: String,
    pub frontier_digest: String,
    pub selected_residual_ref: String,
    pub selected_move_ref: String,
    pub selected_producer_lane_ref: String,
    pub prior_commit_ref: Option<String>,
    pub commit_ref: Option<String>,
    pub review_or_payment_ref: Option<String>,
    pub world_delta_ref: Option<String>,
    pub selection_origin: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveTrajectoryLink {
    pub previous_cycle_index: usize,
    pub next_cycle_index: usize,
    pub previous_commit_ref: String,
    pub previous_world_digest: String,
    pub next_world_digest: String,
    pub next_frontier_digest: String,
    pub fresh_post_commit_selection: bool,
    pub precomputed_execution_authority: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdaptiveTrajectoryLinkError {
    #[error("previous cycle has no committed transition")]
    PreviousCycleNotCommitted,
    #[error("cycle indices are not consecutive: previous={previous}, next={next}")]
    NonConsecutiveCycle { previous: usize, next: usize },
    #[error("next selection prior commit mismatch: expected {expected}, observed {observed:?}")]
    PriorCommitMismatch {
        expected: String,
        observed: Option<String>,
    },
    #[error("next selection was not produced by post-commit current-world diagnosis")]
    InvalidSelectionOrigin,
    #[error("trajectory receipt attempted semantic promotion")]
    SemanticPromotion,
}

/// Link two consecutive adaptive selections.
///
/// This is intentionally stronger than "different selected residual": the next
/// residual may be of the same kind. What matters is that the next selection
/// is explicitly rooted in the prior committed transition and a new
/// current-world/current-frontier diagnosis, never a queued execution
/// authority from the previous cycle.
pub fn link_adaptive_cycles(
    previous: &AdaptiveSelectionReceipt,
    next: &AdaptiveSelectionReceipt,
) -> Result<AdaptiveTrajectoryLink, AdaptiveTrajectoryLinkError> {
    let previous_commit_ref = previous
        .commit_ref
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or(AdaptiveTrajectoryLinkError::PreviousCycleNotCommitted)?;
    if next.cycle_index != previous.cycle_index.saturating_add(1) {
        return Err(AdaptiveTrajectoryLinkError::NonConsecutiveCycle {
            previous: previous.cycle_index,
            next: next.cycle_index,
        });
    }
    if next.prior_commit_ref.as_deref() != Some(previous_commit_ref.as_str()) {
        return Err(AdaptiveTrajectoryLinkError::PriorCommitMismatch {
            expected: previous_commit_ref,
            observed: next.prior_commit_ref.clone(),
        });
    }
    if next.selection_origin != POST_COMMIT_SELECTION_ORIGIN {
        return Err(AdaptiveTrajectoryLinkError::InvalidSelectionOrigin);
    }
    if !previous.candidate_only
        || !next.candidate_only
        || previous.creates_semantic_authority
        || next.creates_semantic_authority
        || previous.applicability_promoted
        || next.applicability_promoted
        || previous.claim_truth_promoted
        || next.claim_truth_promoted
    {
        return Err(AdaptiveTrajectoryLinkError::SemanticPromotion);
    }

    Ok(AdaptiveTrajectoryLink {
        previous_cycle_index: previous.cycle_index,
        next_cycle_index: next.cycle_index,
        previous_commit_ref,
        previous_world_digest: previous.world_digest.clone(),
        next_world_digest: next.world_digest.clone(),
        next_frontier_digest: next.frontier_digest.clone(),
        fresh_post_commit_selection: true,
        precomputed_execution_authority: false,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
