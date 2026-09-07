//! Experimental offline-first proof-search scheduler for SensibLaw.
//!
//! This crate implements only the candidate-selection surface that is already
//! represented in the Agda proof-search work.  It performs no network I/O and
//! grants no semantic, legal, admission, or publication authority.
//!
//! Canonical ordering:
//!
//! proof gap
//! -> declared candidate research moves
//! -> minimum useful proof-reduction filter
//! -> Pareto frontier over cost + proof value
//! -> deterministic selected strategy receipt
//!
//! Live AustLII/JADE work may be represented as a candidate strategy, but this
//! crate cannot execute it.  A live selection returns a typed requirement for a
//! separately governed adapter.

use sensiblaw_legal_follow_plan::{LegalSourcePlan, PlanState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionStrategy {
    LocalFixture,
    PersistedAuthorityReceipt,
    LocalWorldGraph,
    GovernedLiveReferenceSearch,
    GovernedExactAuthorityFetch,
    GovernedBoundedCitationFollow,
}

impl ExecutionStrategy {
    pub const fn is_live(self) -> bool {
        matches!(
            self,
            Self::GovernedLiveReferenceSearch
                | Self::GovernedExactAuthorityFetch
                | Self::GovernedBoundedCitationFollow
        )
    }

    const fn tie_break_rank(self) -> u8 {
        match self {
            Self::LocalFixture => 0,
            Self::PersistedAuthorityReceipt => 1,
            Self::LocalWorldGraph => 2,
            Self::GovernedLiveReferenceSearch => 3,
            Self::GovernedExactAuthorityFetch => 4,
            Self::GovernedBoundedCitationFollow => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticAuthorityStatus {
    ExperimentalCandidateOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionPermission {
    OfflineExecutionAllowed,
    GovernedLiveAdapterRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExecutionCostVector {
    pub network_requests: u64,
    pub minimum_pacing_seconds: u64,
    pub citation_depth: u64,
    pub maximum_new_documents: u64,
    pub cache_misses: u64,
    pub local_bytes_read_cost: u64,
    pub parser_pnf_cost: u64,
    pub semantic_assessment_cost: u64,
    pub operator_review_cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProofValueVector {
    pub expected_proof_reduction: u64,
    pub discriminative_value: u64,
    pub authority_fitness: u64,
    pub novelty: u64,
    pub coverage_gain: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofGap {
    pub consumer_ref: String,
    pub residual_ref: String,
    pub missing_proposition_ref: String,
    pub required_producer_ref: String,
    pub jurisdiction_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMove {
    pub move_ref: String,
    pub strategy: ExecutionStrategy,
    pub source_ref: Option<String>,
    pub provider_operation_ref: String,
    pub cost: ExecutionCostVector,
    pub value: ProofValueVector,
    pub admissible: bool,
    pub calibration_ref: String,
}

impl CandidateMove {
    pub const fn execution_permission(&self) -> ExecutionPermission {
        if self.strategy.is_live() {
            ExecutionPermission::GovernedLiveAdapterRequired
        } else {
            ExecutionPermission::OfflineExecutionAllowed
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerPolicy {
    pub minimum_expected_proof_reduction: u64,
}

impl Default for SchedulerPolicy {
    fn default() -> Self {
        Self {
            minimum_expected_proof_reduction: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateMoveReceipt {
    pub consumer_ref: String,
    pub residual_ref: String,
    pub selected_move_ref: String,
    pub selected_strategy: ExecutionStrategy,
    pub execution_permission: ExecutionPermission,
    pub semantic_authority: SemanticAuthorityStatus,
    pub frontier_move_refs: Vec<String>,
    pub threshold: u64,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleError {
    NoAdmissibleMoveMeetsThreshold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflineExecutionError {
    GovernedLiveAdapterRequired,
}

fn no_more_cost(left: &ExecutionCostVector, right: &ExecutionCostVector) -> bool {
    left.network_requests <= right.network_requests
        && left.minimum_pacing_seconds <= right.minimum_pacing_seconds
        && left.citation_depth <= right.citation_depth
        && left.maximum_new_documents <= right.maximum_new_documents
        && left.cache_misses <= right.cache_misses
        && left.local_bytes_read_cost <= right.local_bytes_read_cost
        && left.parser_pnf_cost <= right.parser_pnf_cost
        && left.semantic_assessment_cost <= right.semantic_assessment_cost
        && left.operator_review_cost <= right.operator_review_cost
}

fn no_less_value(left: &ProofValueVector, right: &ProofValueVector) -> bool {
    left.expected_proof_reduction >= right.expected_proof_reduction
        && left.discriminative_value >= right.discriminative_value
        && left.authority_fitness >= right.authority_fitness
        && left.novelty >= right.novelty
        && left.coverage_gain >= right.coverage_gain
}

fn strictly_better_somewhere(left: &CandidateMove, right: &CandidateMove) -> bool {
    left.cost != right.cost || left.value != right.value
}

pub fn dominates(left: &CandidateMove, right: &CandidateMove) -> bool {
    no_more_cost(&left.cost, &right.cost)
        && no_less_value(&left.value, &right.value)
        && strictly_better_somewhere(left, right)
}

pub fn threshold_candidates(
    candidates: &[CandidateMove],
    policy: SchedulerPolicy,
) -> Vec<&CandidateMove> {
    candidates
        .iter()
        .filter(|candidate| candidate.admissible)
        .filter(|candidate| {
            candidate.value.expected_proof_reduction
                >= policy.minimum_expected_proof_reduction
        })
        .collect()
}

pub fn pareto_frontier<'a>(candidates: &[&'a CandidateMove]) -> Vec<&'a CandidateMove> {
    candidates
        .iter()
        .copied()
        .filter(|candidate| {
            !candidates
                .iter()
                .copied()
                .any(|other| other.move_ref != candidate.move_ref && dominates(other, candidate))
        })
        .collect()
}

fn deterministic_frontier_order(left: &&CandidateMove, right: &&CandidateMove) -> std::cmp::Ordering {
    left.strategy
        .tie_break_rank()
        .cmp(&right.strategy.tie_break_rank())
        .then_with(|| right.value.expected_proof_reduction.cmp(&left.value.expected_proof_reduction))
        .then_with(|| right.value.authority_fitness.cmp(&left.value.authority_fitness))
        .then_with(|| left.cost.network_requests.cmp(&right.cost.network_requests))
        .then_with(|| left.cost.minimum_pacing_seconds.cmp(&right.cost.minimum_pacing_seconds))
        .then_with(|| left.move_ref.cmp(&right.move_ref))
}

pub fn schedule(
    gap: &ProofGap,
    candidates: &[CandidateMove],
    policy: SchedulerPolicy,
) -> Result<CandidateMoveReceipt, ScheduleError> {
    let thresholded = threshold_candidates(candidates, policy);
    if thresholded.is_empty() {
        return Err(ScheduleError::NoAdmissibleMoveMeetsThreshold);
    }

    let mut frontier = pareto_frontier(&thresholded);
    frontier.sort_by(deterministic_frontier_order);
    let selected = frontier
        .first()
        .copied()
        .expect("non-empty threshold set must have a Pareto frontier");

    Ok(CandidateMoveReceipt {
        consumer_ref: gap.consumer_ref.clone(),
        residual_ref: gap.residual_ref.clone(),
        selected_move_ref: selected.move_ref.clone(),
        selected_strategy: selected.strategy,
        execution_permission: selected.execution_permission(),
        semantic_authority: SemanticAuthorityStatus::ExperimentalCandidateOnly,
        frontier_move_refs: frontier.iter().map(|item| item.move_ref.clone()).collect(),
        threshold: policy.minimum_expected_proof_reduction,
        receipt_authority: "experimental_candidate_only",
    })
}

/// The scheduler can authorize only offline execution. A live result is a
/// typed handoff requirement, never an implicit network call.
pub fn require_offline_execution(
    receipt: &CandidateMoveReceipt,
) -> Result<(), OfflineExecutionError> {
    match receipt.execution_permission {
        ExecutionPermission::OfflineExecutionAllowed => Ok(()),
        ExecutionPermission::GovernedLiveAdapterRequired => {
            Err(OfflineExecutionError::GovernedLiveAdapterRequired)
        }
    }
}

/// Convert an already-typed legal-follow plan into a scheduler move without
/// performing acquisition. Persisted plans become offline candidates;
/// acquisition-required plans become governed-live candidate moves only.
pub fn move_from_legal_source_plan(
    plan: &LegalSourcePlan,
    expected_proof_reduction: u64,
    authority_fitness: u64,
) -> Option<CandidateMove> {
    match plan.state {
        PlanState::ReadyPersisted => Some(CandidateMove {
            move_ref: format!("legal-follow:persisted:{}", plan.demand_ref),
            strategy: ExecutionStrategy::PersistedAuthorityReceipt,
            source_ref: plan.selected_source_revision_refs.first().cloned(),
            provider_operation_ref: "persisted_authority_receipt".into(),
            cost: ExecutionCostVector {
                local_bytes_read_cost: 1,
                parser_pnf_cost: 1,
                semantic_assessment_cost: 1,
                ..ExecutionCostVector::default()
            },
            value: ProofValueVector {
                expected_proof_reduction,
                authority_fitness,
                ..ProofValueVector::default()
            },
            admissible: true,
            calibration_ref: "sl-legal-follow-plan:ready-persisted".into(),
        }),
        PlanState::BlockedAcquisitionRequired => Some(CandidateMove {
            move_ref: format!("legal-follow:live-candidate:{}", plan.demand_ref),
            strategy: ExecutionStrategy::GovernedLiveReferenceSearch,
            source_ref: None,
            provider_operation_ref: "governed_live_reference_search".into(),
            cost: ExecutionCostVector {
                network_requests: 1,
                minimum_pacing_seconds: 4,
                maximum_new_documents: 1,
                cache_misses: 1,
                parser_pnf_cost: 1,
                semantic_assessment_cost: 1,
                ..ExecutionCostVector::default()
            },
            value: ProofValueVector {
                expected_proof_reduction,
                authority_fitness,
                novelty: 1,
                coverage_gain: 1,
                ..ProofValueVector::default()
            },
            admissible: true,
            calibration_ref: "sl-legal-follow-plan:acquisition-required-candidate-only".into(),
        }),
        PlanState::BlockedMissingContext => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_legal_follow_plan::{AuthorityLevel, SourceRole};

    fn gap() -> ProofGap {
        ProofGap {
            consumer_ref: "consumer:pabai-duty-route".into(),
            residual_ref: "residual:authority-treatment".into(),
            missing_proposition_ref: "prop:current-treatment-of-authority".into(),
            required_producer_ref: "authority-treatment-producer".into(),
            jurisdiction_ref: Some("AU".into()),
        }
    }

    fn move_(
        move_ref: &str,
        strategy: ExecutionStrategy,
        proof_reduction: u64,
        authority_fitness: u64,
        network_requests: u64,
        pacing_seconds: u64,
    ) -> CandidateMove {
        CandidateMove {
            move_ref: move_ref.into(),
            strategy,
            source_ref: None,
            provider_operation_ref: "fixture".into(),
            cost: ExecutionCostVector {
                network_requests,
                minimum_pacing_seconds: pacing_seconds,
                parser_pnf_cost: 1,
                semantic_assessment_cost: 1,
                ..ExecutionCostVector::default()
            },
            value: ProofValueVector {
                expected_proof_reduction: proof_reduction,
                authority_fitness,
                discriminative_value: 1,
                ..ProofValueVector::default()
            },
            admissible: true,
            calibration_ref: "test".into(),
        }
    }

    #[test]
    fn minimum_proof_reduction_filters_cheap_but_useless_move() {
        let cheap = move_(
            "local:cheap",
            ExecutionStrategy::LocalFixture,
            0,
            1,
            0,
            0,
        );
        let useful = move_(
            "persisted:useful",
            ExecutionStrategy::PersistedAuthorityReceipt,
            2,
            2,
            0,
            0,
        );
        let receipt = schedule(
            &gap(),
            &[cheap, useful],
            SchedulerPolicy {
                minimum_expected_proof_reduction: 1,
            },
        )
        .expect("useful move should survive threshold");
        assert_eq!(receipt.selected_move_ref, "persisted:useful");
    }

    #[test]
    fn equivalent_live_move_is_dominated_by_offline_move() {
        let local = move_(
            "local",
            ExecutionStrategy::PersistedAuthorityReceipt,
            2,
            3,
            0,
            0,
        );
        let live = move_(
            "live",
            ExecutionStrategy::GovernedLiveReferenceSearch,
            2,
            3,
            1,
            4,
        );
        assert!(dominates(&local, &live));
        let receipt = schedule(&gap(), &[local, live], SchedulerPolicy::default())
            .expect("offline move should be schedulable");
        assert_eq!(
            receipt.selected_strategy,
            ExecutionStrategy::PersistedAuthorityReceipt
        );
        assert_eq!(
            receipt.execution_permission,
            ExecutionPermission::OfflineExecutionAllowed
        );
    }

    #[test]
    fn live_high_value_move_can_remain_pareto_but_cannot_execute_here() {
        let local = move_(
            "local",
            ExecutionStrategy::PersistedAuthorityReceipt,
            1,
            2,
            0,
            0,
        );
        let live = move_(
            "live",
            ExecutionStrategy::GovernedLiveReferenceSearch,
            4,
            5,
            1,
            4,
        );
        let candidates = [local.clone(), live.clone()];
        let thresholded = threshold_candidates(&candidates, SchedulerPolicy::default());
        let frontier = pareto_frontier(&thresholded);
        assert_eq!(frontier.len(), 2);

        let live_only = schedule(
            &gap(),
            &[live],
            SchedulerPolicy {
                minimum_expected_proof_reduction: 4,
            },
        )
        .expect("live strategy may be selected as a candidate");
        assert_eq!(
            live_only.execution_permission,
            ExecutionPermission::GovernedLiveAdapterRequired
        );
        assert_eq!(
            require_offline_execution(&live_only),
            Err(OfflineExecutionError::GovernedLiveAdapterRequired)
        );
        assert_eq!(
            live_only.semantic_authority,
            SemanticAuthorityStatus::ExperimentalCandidateOnly
        );
    }

    #[test]
    fn ready_persisted_legal_follow_plan_becomes_offline_candidate() {
        let plan = LegalSourcePlan {
            demand_ref: "d:authority".into(),
            jurisdiction_ref: Some("AU".into()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: vec!["au:hca".into()],
            requested_facets: vec!["legal.authority_unresolved".into()],
            temporal_refs: vec!["current".into()],
            state: PlanState::ReadyPersisted,
            blocked_reasons: Vec::new(),
            selected_source_revision_refs: vec!["source:hca:rev:1".into()],
            authority: "acquisition_plan_only",
        };
        let candidate = move_from_legal_source_plan(&plan, 3, 5)
            .expect("ready persisted plan should compile to a move");
        assert_eq!(
            candidate.strategy,
            ExecutionStrategy::PersistedAuthorityReceipt
        );
        assert_eq!(candidate.cost.network_requests, 0);
        assert_eq!(candidate.source_ref.as_deref(), Some("source:hca:rev:1"));
    }

    #[test]
    fn acquisition_required_plan_is_live_candidate_not_network_execution() {
        let plan = LegalSourcePlan {
            demand_ref: "d:missing-authority".into(),
            jurisdiction_ref: Some("AU".into()),
            source_roles: vec![SourceRole::PrimaryCaseLaw],
            authority_levels: vec![AuthorityLevel::Official],
            provider_profile_refs: Vec::new(),
            requested_facets: vec!["legal.authority_unresolved".into()],
            temporal_refs: Vec::new(),
            state: PlanState::BlockedAcquisitionRequired,
            blocked_reasons: vec!["compatible_persisted_legal_source_absent".into()],
            selected_source_revision_refs: Vec::new(),
            authority: "acquisition_plan_only",
        };
        let candidate = move_from_legal_source_plan(&plan, 2, 4)
            .expect("acquisition-required plan should become a live candidate");
        assert_eq!(
            candidate.strategy,
            ExecutionStrategy::GovernedLiveReferenceSearch
        );
        assert_eq!(candidate.cost.network_requests, 1);
        assert_eq!(candidate.cost.minimum_pacing_seconds, 4);
        assert_eq!(
            candidate.execution_permission(),
            ExecutionPermission::GovernedLiveAdapterRequired
        );
    }
}
