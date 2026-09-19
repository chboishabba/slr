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
    AdaptiveNegativeAssessmentRow, ContextReviewDecision, DiscoveryIdentityBaseline,
    LatentWorldRows, ReviewedContextEdge,
};
use sensiblaw_proof_search_loop::frontier::{
    select_frontier_move, FrontierCandidateMove, ProofFrontier, ProofResidual, ResidualStatus,
};
use sensiblaw_proof_search_loop::world_expansion::WorldExpansionPolicy;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    RecurrentRunBlocker, RecurrentRunBlockerKind,
};
use sensiblaw_proof_search_scheduler::{
    CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector,
};
use sensiblaw_route_selector::{decode_route_candidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{emit_candidates_from_rdf, AcquiredEntityRdf};
use thiserror::Error;

use crate::{MaboConsumerDiagnosis, MaboIdentityReviewPlan, MABO_NOVEL_IDENTITY_TARGET};

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

/// Campaign completion is seed-scoped discovery-lineage cardinality, not the
/// global world-identity quotient shared by every research campaign.
#[must_use]
pub fn mabo_target_complete(durable_campaign_identity_classes: usize) -> bool {
    durable_campaign_identity_classes >= MABO_NOVEL_IDENTITY_TARGET
}

#[must_use]
pub fn mabo_remaining_adaptive_world_expansion_policy(
    durable_campaign_identity_classes: usize,
) -> WorldExpansionPolicy {
    WorldExpansionPolicy {
        target_novel_objects: MABO_NOVEL_IDENTITY_TARGET
            .saturating_sub(durable_campaign_identity_classes),
        minimum_expected_residual_contraction: 1,
    }
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
pub struct MaboTypedProducerSelection {
    pub residual_ref: String,
    pub residual_class: sensiblaw_proof_search_loop::world_expansion::ResidualClass,
    pub producer_lane: sensiblaw_proof_search_loop::world_expansion::ProducerLane,
    pub move_ref: String,
    pub source_ref: Option<String>,
    pub provider_operation_ref: String,
    pub shared_dependency_gain: u64,
    pub diagnosis_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaboAdaptiveDecision {
    Identity(MaboAdaptiveSelection),
    ContextExpansion(MaboContextExpansionSelection),
    TypedProducer(MaboTypedProducerSelection),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedAdaptiveResidualMove {
    pub residual: ProofResidual,
    pub residual_class: sensiblaw_proof_search_loop::world_expansion::ResidualClass,
    pub producer_lane: sensiblaw_proof_search_loop::world_expansion::ProducerLane,
    pub move_ref: String,
    pub source_ref: Option<String>,
    pub provider_operation_ref: String,
    pub expected_whole_frontier_reduction: u64,
    pub shared_dependency_gain: u64,
    pub network_requests: u64,
    pub operator_review_cost: u64,
    pub admissible: bool,
    pub diagnosis_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DurableAdaptiveNegativeKind {
    WrongType,
    Duplicate,
    IrrelevantToResidual,
    FailedFactorsThrough,
    Inadmissible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableAdaptiveNegativeAssessment {
    pub residual_ref: String,
    pub move_ref: String,
    pub kind: DurableAdaptiveNegativeKind,
    pub assessment_ref: String,
    pub source_revision_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl DurableAdaptiveNegativeAssessment {
    #[must_use]
    pub const fn is_non_promoting(&self) -> bool {
        self.candidate_only
            && !self.creates_semantic_authority
            && !self.applicability_promoted
            && !self.claim_truth_promoted
    }
}

#[must_use]
pub fn durable_negative_assessments_from_rows(
    rows: &[AdaptiveNegativeAssessmentRow],
) -> Vec<DurableAdaptiveNegativeAssessment> {
    rows.iter()
        .filter_map(|row| {
            if !row.candidate_only
                || !row.makes_move_inadmissible
                || row.satisfies_residual
                || row.creates_semantic_authority
                || row.applicability_promoted
                || row.claim_truth_promoted
            {
                return None;
            }
            let kind = match row.kind_ref.as_str() {
                "wrong-type" => DurableAdaptiveNegativeKind::WrongType,
                "duplicate" => DurableAdaptiveNegativeKind::Duplicate,
                "irrelevant-to-residual" => DurableAdaptiveNegativeKind::IrrelevantToResidual,
                "failed-factors-through" => DurableAdaptiveNegativeKind::FailedFactorsThrough,
                "inadmissible" => DurableAdaptiveNegativeKind::Inadmissible,
                _ => return None,
            };
            Some(DurableAdaptiveNegativeAssessment {
                residual_ref: row.residual_ref.clone(),
                move_ref: row.move_ref.clone(),
                kind,
                assessment_ref: row.assessment_ref.clone(),
                source_revision_ref: row.source_revision_ref.clone(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TypedAdaptiveMoveMetadata {
    residual_class: sensiblaw_proof_search_loop::world_expansion::ResidualClass,
    producer_lane: sensiblaw_proof_search_loop::world_expansion::ProducerLane,
    source_ref: Option<String>,
    provider_operation_ref: String,
    diagnosis_reference: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboAdaptiveFrontierCompilation {
    pub frontier: ProofFrontier,
    pub candidates: Vec<FrontierCandidateMove>,
    typed_move_metadata: BTreeMap<String, TypedAdaptiveMoveMetadata>,
}

/// Compile the whole current Mabo adaptive work surface before review lookup.
///
/// This is the exact frontier/candidate pair consumed by the Pareto selector.
/// Exposing it makes the selection provenance auditable and gives later legal,
/// provenance and diagnostic residual compilers one composition point without
/// introducing another scheduler.
#[must_use]
pub fn compile_mabo_adaptive_frontier(
    diagnosis: &MaboConsumerDiagnosis,
    baseline: &DiscoveryIdentityBaseline,
    world: &LatentWorldRows,
    expanded_source_refs: &BTreeSet<String>,
    frontier_ref: impl Into<String>,
) -> MaboAdaptiveFrontierCompilation {
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
    MaboAdaptiveFrontierCompilation {
        frontier,
        candidates,
        typed_move_metadata: BTreeMap::new(),
    }
}

fn decision_from_compilation(
    diagnosis: &MaboConsumerDiagnosis,
    compiled: &MaboAdaptiveFrontierCompilation,
) -> Option<MaboAdaptiveDecision> {
    let selected = select_frontier_move(&compiled.frontier, &compiled.candidates, 1)?;
    let residual_ref = selected.target_residual_refs.first()?.clone();

    if let Some(row) = diagnosis.rows.iter().find(|row| row.residual_ref == residual_ref) {
        return Some(MaboAdaptiveDecision::Identity(MaboAdaptiveSelection {
            residual_ref,
            representation_ref: row.representation_ref.clone(),
            move_ref: selected.move_.move_ref.clone(),
            shared_dependency_gain: selected.shared_dependency_gain,
        }));
    }
    if let Some(source_qid) = residual_ref
        .strip_prefix(CONTEXT_EXPANSION_RESIDUAL_PREFIX)
        .map(ToOwned::to_owned)
    {
        return Some(MaboAdaptiveDecision::ContextExpansion(
            MaboContextExpansionSelection {
                residual_ref,
                source_qid,
                move_ref: selected.move_.move_ref.clone(),
                shared_dependency_gain: selected.shared_dependency_gain,
            },
        ));
    }
    let metadata = compiled
        .typed_move_metadata
        .get(&selected.move_.move_ref)?;
    Some(MaboAdaptiveDecision::TypedProducer(MaboTypedProducerSelection {
        residual_ref,
        residual_class: metadata.residual_class,
        producer_lane: metadata.producer_lane,
        move_ref: selected.move_.move_ref.clone(),
        source_ref: metadata.source_ref.clone(),
        provider_operation_ref: metadata.provider_operation_ref.clone(),
        shared_dependency_gain: selected.shared_dependency_gain,
        diagnosis_reference: metadata.diagnosis_reference.clone(),
    }))
}

/// Add heterogeneous consumer-derived residuals to the exact current Mabo
/// frontier while preserving the existing canonical Pareto scheduler.
///
/// Durable negative assessments are candidate/move constraints only. They do
/// not mark the target residual satisfied. Invalid/promoting negative receipts
/// are ignored rather than gaining suppression authority.
#[must_use]
pub fn compile_mabo_heterogeneous_frontier(
    diagnosis: &MaboConsumerDiagnosis,
    baseline: &DiscoveryIdentityBaseline,
    world: &LatentWorldRows,
    expanded_source_refs: &BTreeSet<String>,
    additional_moves: &[TypedAdaptiveResidualMove],
    negative_assessments: &[DurableAdaptiveNegativeAssessment],
    frontier_ref: impl Into<String>,
) -> MaboAdaptiveFrontierCompilation {
    let mut compiled = compile_mabo_adaptive_frontier(
        diagnosis,
        baseline,
        world,
        expanded_source_refs,
        frontier_ref,
    );

    let mut residual_refs = compiled
        .frontier
        .residuals
        .iter()
        .map(|residual| residual.residual_ref.clone())
        .collect::<BTreeSet<_>>();

    for additional in additional_moves {
        if residual_refs.insert(additional.residual.residual_ref.clone()) {
            compiled.frontier.residuals.push(additional.residual.clone());
        }

        let suppressed = negative_assessments.iter().any(|assessment| {
            assessment.is_non_promoting()
                && assessment.residual_ref == additional.residual.residual_ref
                && assessment.move_ref == additional.move_ref
        });
        let admissible = additional.admissible && !suppressed;

        compiled.candidates.push(FrontierCandidateMove {
            target_residual_refs: vec![additional.residual.residual_ref.clone()],
            move_: CandidateMove {
                move_ref: additional.move_ref.clone(),
                strategy: ExecutionStrategy::GovernedExactAuthorityFetch,
                source_ref: additional.source_ref.clone(),
                provider_operation_ref: additional.provider_operation_ref.clone(),
                cost: ExecutionCostVector {
                    network_requests: additional.network_requests,
                    operator_review_cost: additional.operator_review_cost,
                    ..ExecutionCostVector::default()
                },
                value: ProofValueVector {
                    expected_proof_reduction: additional.expected_whole_frontier_reduction,
                    discriminative_value: additional.shared_dependency_gain,
                    coverage_gain: additional.shared_dependency_gain,
                    ..ProofValueVector::default()
                },
                admissible,
                calibration_ref: "mabo-adaptive-heterogeneous:v1".into(),
            },
            expected_whole_frontier_reduction: additional.expected_whole_frontier_reduction,
            shared_dependency_gain: additional.shared_dependency_gain,
        });
        compiled.typed_move_metadata.insert(
            additional.move_ref.clone(),
            TypedAdaptiveMoveMetadata {
                residual_class: additional.residual_class,
                producer_lane: additional.producer_lane,
                source_ref: additional.source_ref.clone(),
                provider_operation_ref: additional.provider_operation_ref.clone(),
                diagnosis_reference: additional.diagnosis_reference.clone(),
            },
        );
    }

    compiled.frontier.residuals.sort();
    compiled.frontier.residuals.dedup();
    compiled.candidates.sort_by(|left, right| {
        left.move_
            .move_ref
            .cmp(&right.move_.move_ref)
            .then_with(|| left.target_residual_refs.cmp(&right.target_residual_refs))
    });
    compiled.candidates.dedup_by(|left, right| {
        left.move_.move_ref == right.move_.move_ref
            && left.target_residual_refs == right.target_residual_refs
    });

    compiled
}

#[must_use]
pub fn select_next_mabo_heterogeneous_decision(
    diagnosis: &MaboConsumerDiagnosis,
    compiled: &MaboAdaptiveFrontierCompilation,
) -> Option<MaboAdaptiveDecision> {
    decision_from_compilation(diagnosis, compiled)
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
    let compiled = compile_mabo_adaptive_frontier(
        diagnosis,
        baseline,
        world,
        expanded_source_refs,
        frontier_ref,
    );
    decision_from_compilation(diagnosis, &compiled)
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
