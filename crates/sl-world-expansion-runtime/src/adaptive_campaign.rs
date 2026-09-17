//! Adaptive one-hop Mabo campaign projections.
//!
//! This module owns no review inference and no semantic authority. It projects
//! the current identity diagnosis and durable-but-unexpanded QID surface into
//! the canonical proof-frontier Pareto selector, and parses one already-
//! acquired target manifestation into the finite bounded Wikidata context
//! surface used by the campaign.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use sensiblaw_pg_source_store::{
    bounded_wikidata_relation_type, review_bounded_wikidata_candidate,
    ContextReviewDecision, DiscoveryIdentityBaseline, LatentWorldRows, ReviewedContextEdge,
};
use sensiblaw_proof_search_loop::frontier::{
    select_frontier_move, FrontierCandidateMove, ProofFrontier, ProofResidual, ResidualStatus,
};
use sensiblaw_proof_search_loop::world_expansion_runner::{
    RecurrentRunBlocker, RecurrentRunBlockerKind,
};
use sensiblaw_proof_search_scheduler::{
    CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector,
};
use sensiblaw_route_selector::{decode_route_candidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{emit_candidates_from_rdf, AcquiredEntityRdf};
use thiserror::Error;

use crate::{MaboConsumerDiagnosis, MaboIdentityReviewPlan};

const CONTEXT_EXPANSION_PRODUCER: &str = "producer:wikidata-bounded-context-expansion";
const CONTEXT_EXPANSION_RESIDUAL_PREFIX: &str = "residual:mabo:context-expansion:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboAdaptiveSelection {
    pub residual_ref: String,
    pub representation_ref: String,
    pub move_ref: String,
    pub shared_dependency_gain: u64,
}

fn reviewed_frontier_moves(plan: &MaboIdentityReviewPlan) -> Vec<FrontierCandidateMove> {
    plan.matched
        .iter()
        .map(|planned| {
            let shared_dependency_gain =
                u64::try_from(planned.row.relation_type_refs.len().max(1)).unwrap_or(u64::MAX);
            FrontierCandidateMove {
                target_residual_refs: vec![planned.row.residual_ref.clone()],
                move_: CandidateMove {
                    move_ref: format!("move:mabo-adaptive:{}", planned.row.residual_ref),
                    strategy: ExecutionStrategy::GovernedExactAuthorityFetch,
                    source_ref: planned.row.source_revision_refs.first().cloned(),
                    provider_operation_ref: "wikidata:exact-reviewed-reacquisition".into(),
                    cost: ExecutionCostVector {
                        network_requests: 1,
                        operator_review_cost: 0,
                        ..ExecutionCostVector::default()
                    },
                    value: ProofValueVector {
                        expected_proof_reduction: 1,
                        coverage_gain: shared_dependency_gain,
                        ..ProofValueVector::default()
                    },
                    admissible: true,
                    calibration_ref: "mabo-adaptive-current-frontier:v1".into(),
                },
                expected_whole_frontier_reduction: 1,
                shared_dependency_gain,
            }
        })
        .collect()
}

/// Compatibility selector for already-matched reviewed identity rows.
#[must_use]
pub fn select_next_reviewed_mabo_gap(
    frontier: &ProofFrontier,
    plan: &MaboIdentityReviewPlan,
) -> Option<MaboAdaptiveSelection> {
    let candidates = reviewed_frontier_moves(plan);
    let selected = select_frontier_move(frontier, &candidates, 1)?;
    let residual_ref = selected.target_residual_refs.first()?.clone();
    let planned = plan
        .matched
        .iter()
        .find(|planned| planned.row.residual_ref == residual_ref)?;
    Some(MaboAdaptiveSelection {
        residual_ref,
        representation_ref: planned.row.representation_ref.clone(),
        move_ref: selected.move_.move_ref.clone(),
        shared_dependency_gain: selected.shared_dependency_gain,
    })
}

fn valid_qid(value: &str) -> bool {
    value
        .strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

#[must_use]
pub fn diagnose_mabo_context_expansion_frontier(
    baseline: &DiscoveryIdentityBaseline,
    world: &LatentWorldRows,
    expanded_source_refs: &BTreeSet<String>,
    frontier_ref: impl Into<String>,
) -> ProofFrontier {
    let current_world_refs = world
        .visited_refs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut dependencies: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for edge in &world.edges {
        if edge.relation_ref.starts_with("context:wikidata:") {
            dependencies
                .entry(edge.to_ref.as_str())
                .or_default()
                .insert(edge.relation_ref.clone());
        }
    }

    let mut residuals = baseline
        .representation_identity_class_refs
        .keys()
        .filter(|representation_ref| valid_qid(representation_ref))
        .filter(|representation_ref| current_world_refs.contains(representation_ref.as_str()))
        .filter(|representation_ref| !expanded_source_refs.contains(*representation_ref))
        .map(|representation_ref| {
            let dependency_refs = dependencies
                .get(representation_ref.as_str())
                .map(|refs| refs.iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            ProofResidual {
                residual_ref: format!("{CONTEXT_EXPANSION_RESIDUAL_PREFIX}{representation_ref}"),
                proposition_ref: format!("mabo:bounded-context-expansion:{representation_ref}"),
                producer_class_ref: CONTEXT_EXPANSION_PRODUCER.into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: None,
                salience: u64::try_from(dependency_refs.len().max(1)).unwrap_or(u64::MAX),
                dependency_refs,
                status: ResidualStatus::Open,
            }
        })
        .collect::<Vec<_>>();
    residuals.sort_by(|left, right| left.residual_ref.cmp(&right.residual_ref));

    ProofFrontier {
        consumer_ref: "consumer:mabo-reviewed-context-expansion".into(),
        frontier_ref: frontier_ref.into(),
        residuals,
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboContextExpansionSelection {
    pub residual_ref: String,
    pub source_qid: String,
    pub move_ref: String,
    pub shared_dependency_gain: u64,
}

fn context_expansion_moves(frontier: &ProofFrontier) -> Vec<FrontierCandidateMove> {
    frontier
        .open_residuals()
        .filter(|residual| residual.producer_class_ref == CONTEXT_EXPANSION_PRODUCER)
        .filter_map(|residual| {
            let source_qid = residual
                .residual_ref
                .strip_prefix(CONTEXT_EXPANSION_RESIDUAL_PREFIX)?;
            if !valid_qid(source_qid) {
                return None;
            }
            let shared_dependency_gain =
                u64::try_from(residual.dependency_refs.len().max(1)).unwrap_or(u64::MAX);
            Some(FrontierCandidateMove {
                target_residual_refs: vec![residual.residual_ref.clone()],
                move_: CandidateMove {
                    move_ref: format!("move:mabo-context-expand:{source_qid}"),
                    strategy: ExecutionStrategy::GovernedExactAuthorityFetch,
                    source_ref: Some(source_qid.to_owned()),
                    provider_operation_ref: "wikidata:latest-coordinate-then-exact-revision".into(),
                    cost: ExecutionCostVector {
                        network_requests: 2,
                        parser_pnf_cost: 1,
                        semantic_assessment_cost: 1,
                        operator_review_cost: 1,
                        ..ExecutionCostVector::default()
                    },
                    value: ProofValueVector {
                        expected_proof_reduction: 1,
                        discriminative_value: shared_dependency_gain,
                        coverage_gain: shared_dependency_gain,
                        ..ProofValueVector::default()
                    },
                    admissible: true,
                    calibration_ref: "mabo-adaptive-context-expansion:v1".into(),
                },
                expected_whole_frontier_reduction: 1,
                shared_dependency_gain,
            })
        })
        .collect()
}

#[must_use]
pub fn select_next_mabo_context_expansion(
    frontier: &ProofFrontier,
) -> Option<MaboContextExpansionSelection> {
    let candidates = context_expansion_moves(frontier);
    let selected = select_frontier_move(frontier, &candidates, 1)?;
    let residual_ref = selected.target_residual_refs.first()?.clone();
    let source_qid = residual_ref
        .strip_prefix(CONTEXT_EXPANSION_RESIDUAL_PREFIX)?
        .to_owned();
    Some(MaboContextExpansionSelection {
        residual_ref,
        source_qid,
        move_ref: selected.move_.move_ref.clone(),
        shared_dependency_gain: selected.shared_dependency_gain,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaboAdaptiveDecision {
    Identity(MaboAdaptiveSelection),
    ContextExpansion(MaboContextExpansionSelection),
}

fn identity_moves(diagnosis: &MaboConsumerDiagnosis) -> Vec<FrontierCandidateMove> {
    diagnosis
        .rows
        .iter()
        .map(|row| {
            let shared_dependency_gain =
                u64::try_from(row.relation_type_refs.len().max(1)).unwrap_or(u64::MAX);
            FrontierCandidateMove {
                target_residual_refs: vec![row.residual_ref.clone()],
                move_: CandidateMove {
                    move_ref: format!("move:mabo-identity:{}", row.representation_ref),
                    strategy: ExecutionStrategy::GovernedExactAuthorityFetch,
                    source_ref: row.source_revision_refs.first().cloned(),
                    provider_operation_ref: "wikidata:identity-review-then-exact-reacquisition".into(),
                    cost: ExecutionCostVector {
                        network_requests: 1,
                        operator_review_cost: 1,
                        ..ExecutionCostVector::default()
                    },
                    value: ProofValueVector {
                        expected_proof_reduction: 1,
                        discriminative_value: shared_dependency_gain,
                        coverage_gain: shared_dependency_gain,
                        ..ProofValueVector::default()
                    },
                    admissible: true,
                    calibration_ref: "mabo-adaptive-identity:v1".into(),
                },
                expected_whole_frontier_reduction: 1,
                shared_dependency_gain,
            }
        })
        .collect()
}

/// Rank the current semantic work before looking at review manifests.
///
/// The review manifest is an execution authority, not a scheduler prior. Both
/// identity gaps and durable-source expansion gaps are projected into one fresh
/// frontier and one canonical Pareto selection on every iteration.
#[must_use]
pub fn select_next_mabo_adaptive_decision(
    diagnosis: &MaboConsumerDiagnosis,
    baseline: &DiscoveryIdentityBaseline,
    world: &LatentWorldRows,
    expanded_source_refs: &BTreeSet<String>,
    frontier_ref: impl Into<String>,
) -> Option<MaboAdaptiveDecision> {
    let expansion_frontier = diagnose_mabo_context_expansion_frontier(
        baseline,
        world,
        expanded_source_refs,
        "frontier:mabo:context-expansion:adaptive",
    );
    let mut residuals = diagnosis.residuals.clone();
    for residual in &mut residuals {
        if let Some(row) = diagnosis
            .rows
            .iter()
            .find(|row| row.residual_ref == residual.residual_ref)
        {
            residual.dependency_refs = row.relation_type_refs.clone();
        }
    }
    residuals.extend(expansion_frontier.residuals.clone());
    let frontier = ProofFrontier {
        consumer_ref: "consumer:mabo-adaptive-world-expansion".into(),
        frontier_ref: frontier_ref.into(),
        residuals,
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    let mut candidates = identity_moves(diagnosis);
    candidates.extend(context_expansion_moves(&expansion_frontier));
    let selected = select_frontier_move(&frontier, &candidates, 1)?;
    let residual_ref = selected.target_residual_refs.first()?.clone();

    if let Some(row) = diagnosis.rows.iter().find(|row| row.residual_ref == residual_ref) {
        return Some(MaboAdaptiveDecision::Identity(MaboAdaptiveSelection {
            residual_ref,
            representation_ref: row.representation_ref.clone(),
            move_ref: selected.move_.move_ref.clone(),
            shared_dependency_gain: selected.shared_dependency_gain,
        }));
    }
    let source_qid = residual_ref
        .strip_prefix(CONTEXT_EXPANSION_RESIDUAL_PREFIX)?
        .to_owned();
    Some(MaboAdaptiveDecision::ContextExpansion(
        MaboContextExpansionSelection {
            residual_ref,
            source_qid,
            move_ref: selected.move_.move_ref.clone(),
            shared_dependency_gain: selected.shared_dependency_gain,
        },
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParsedBoundedContextCandidate {
    pub candidate_id: String,
    pub source_qid: String,
    pub target_qid: String,
    pub property_ref: String,
    pub source_revision_ref: String,
}

#[derive(Debug, Error)]
pub enum AdaptiveTargetContextError {
    #[error("candidate provider parse failed: {0}")]
    Provider(String),
    #[error("route decode failed: {0}")]
    Route(String),
    #[error("acquired target artifact is promoting or not candidate-only")]
    PromotingArtifact,
}

pub fn parse_bounded_target_context(
    acquired: &AcquiredEntityRdf,
) -> Result<Vec<ParsedBoundedContextCandidate>, AdaptiveTargetContextError> {
    if !acquired.candidate_only || acquired.semantic_promotion {
        return Err(AdaptiveTargetContextError::PromotingArtifact);
    }

    let mut encoded = Vec::new();
    emit_candidates_from_rdf(
        &acquired.qid,
        Cursor::new(acquired.rdf_bytes.as_slice()),
        &mut encoded,
    )
    .map_err(|error| AdaptiveTargetContextError::Provider(error.to_string()))?;

    let mut cursor = Cursor::new(encoded);
    let mut candidates = Vec::new();
    loop {
        let route = decode_route_candidate(&mut cursor)
            .map_err(|error| AdaptiveTargetContextError::Route(error.to_string()))?;
        let Some(route) = route else {
            break;
        };
        if route.route_family != RouteFamily::WikidataProperty
            || route.source_ref != acquired.qid
            || bounded_wikidata_relation_type(&route.property_ref).is_none()
        {
            continue;
        }
        candidates.push(ParsedBoundedContextCandidate {
            candidate_id: route.candidate_id,
            source_qid: route.source_ref,
            target_qid: route.target_ref,
            property_ref: route.property_ref,
            source_revision_ref: acquired.source_revision_ref.clone(),
        });
    }

    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

pub fn prepare_reviewed_target_context(
    candidates: &[ParsedBoundedContextCandidate],
    review_decision: ContextReviewDecision,
) -> Result<Vec<ReviewedContextEdge>, RecurrentRunBlocker> {
    if review_decision != ContextReviewDecision::Reviewed {
        return Err(RecurrentRunBlocker::new(
            RecurrentRunBlockerKind::ContextReviewRequired,
            "campaign:outgoing-context-review-required",
        ));
    }

    candidates
        .iter()
        .map(|candidate| {
            review_bounded_wikidata_candidate(
                candidate.source_revision_ref.clone(),
                candidate.candidate_id.clone(),
                candidate.source_qid.clone(),
                candidate.target_qid.clone(),
                candidate.property_ref.clone(),
                review_decision,
            )
            .map_err(|error| {
                RecurrentRunBlocker::new(
                    RecurrentRunBlockerKind::Other,
                    format!("campaign:bounded-context-review:{error}"),
                )
            })
        })
        .collect()
}
