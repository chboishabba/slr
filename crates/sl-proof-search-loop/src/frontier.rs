use sensiblaw_proof_search_scheduler::{dominates, CandidateMove, ProofGap};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResidualStatus {
    Open,
    SatisfiedCandidate,
    Contested,
    AuthorityBlocked,
    Underidentified,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProofResidual {
    pub residual_ref: String,
    pub proposition_ref: String,
    pub producer_class_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub authority_requirement_ref: Option<String>,
    pub salience: u64,
    pub dependency_refs: Vec<String>,
    pub status: ResidualStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofFrontier {
    pub consumer_ref: String,
    pub frontier_ref: String,
    pub residuals: Vec<ProofResidual>,
    pub satisfied_payment_refs: Vec<String>,
    pub contested_coordinate_refs: Vec<String>,
    pub authority_blocked_refs: Vec<String>,
    pub authority: &'static str,
}

impl ProofFrontier {
    pub fn open_residuals(&self) -> impl Iterator<Item = &ProofResidual> {
        self.residuals.iter().filter(|r| r.status == ResidualStatus::Open)
    }

    pub fn to_gaps(&self) -> Vec<ProofGap> {
        self.open_residuals()
            .map(|r| ProofGap {
                consumer_ref: self.consumer_ref.clone(),
                residual_ref: r.residual_ref.clone(),
                missing_proposition_ref: r.proposition_ref.clone(),
                required_producer_ref: r.producer_class_ref.clone(),
                jurisdiction_ref: r.jurisdiction_ref.clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierCandidateMove {
    pub target_residual_refs: Vec<String>,
    pub move_: CandidateMove,
    pub expected_whole_frontier_reduction: u64,
    pub shared_dependency_gain: u64,
}

pub fn validate_frontier_move(frontier: &ProofFrontier, candidate: &FrontierCandidateMove) -> bool {
    let open: BTreeSet<&str> = frontier.open_residuals().map(|r| r.residual_ref.as_str()).collect();
    !candidate.target_residual_refs.is_empty()
        && candidate
            .target_residual_refs
            .iter()
            .all(|r| open.contains(r.as_str()))
}

pub fn rankable_candidates<'a>(
    frontier: &ProofFrontier,
    candidates: &'a [FrontierCandidateMove],
) -> Vec<&'a FrontierCandidateMove> {
    candidates
        .iter()
        .filter(|c| validate_frontier_move(frontier, c))
        .collect()
}

fn frontier_dominates(left: &FrontierCandidateMove, right: &FrontierCandidateMove) -> bool {
    let no_less_frontier_gain = left.expected_whole_frontier_reduction >= right.expected_whole_frontier_reduction;
    let no_less_shared_gain = left.shared_dependency_gain >= right.shared_dependency_gain;
    let scheduler_no_worse = dominates(&left.move_, &right.move_) || left.move_ == right.move_;
    let strictly_better = left.expected_whole_frontier_reduction > right.expected_whole_frontier_reduction
        || left.shared_dependency_gain > right.shared_dependency_gain
        || dominates(&left.move_, &right.move_);
    no_less_frontier_gain && no_less_shared_gain && scheduler_no_worse && strictly_better
}

pub fn frontier_pareto<'a>(
    frontier: &ProofFrontier,
    candidates: &'a [FrontierCandidateMove],
    minimum_whole_frontier_reduction: u64,
) -> Vec<&'a FrontierCandidateMove> {
    let rankable: Vec<&FrontierCandidateMove> = rankable_candidates(frontier, candidates)
        .into_iter()
        .filter(|c| c.move_.admissible)
        .filter(|c| c.expected_whole_frontier_reduction >= minimum_whole_frontier_reduction)
        .collect();
    rankable
        .iter()
        .copied()
        .filter(|candidate| {
            !rankable.iter().copied().any(|other| {
                other.move_.move_ref != candidate.move_.move_ref && frontier_dominates(other, candidate)
            })
        })
        .collect()
}

pub fn select_frontier_move<'a>(
    frontier: &ProofFrontier,
    candidates: &'a [FrontierCandidateMove],
    minimum_whole_frontier_reduction: u64,
) -> Option<&'a FrontierCandidateMove> {
    let mut p = frontier_pareto(frontier, candidates, minimum_whole_frontier_reduction);
    p.sort_by(|a, b| {
        b.expected_whole_frontier_reduction
            .cmp(&a.expected_whole_frontier_reduction)
            .then_with(|| b.shared_dependency_gain.cmp(&a.shared_dependency_gain))
            .then_with(|| a.move_.cost.network_requests.cmp(&b.move_.cost.network_requests))
            .then_with(|| a.move_.move_ref.cmp(&b.move_.move_ref))
    });
    p.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_proof_search_scheduler::{ExecutionCostVector, ExecutionStrategy, ProofValueVector};

    fn frontier() -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:pabai".into(),
            frontier_ref: "frontier:1".into(),
            residuals: vec![
                ProofResidual {
                    residual_ref: "r:a".into(),
                    proposition_ref: "p:a".into(),
                    producer_class_ref: "producer:a".into(),
                    jurisdiction_ref: Some("AU".into()),
                    authority_requirement_ref: Some("primary-case".into()),
                    salience: 5,
                    dependency_refs: vec!["dep:shared".into()],
                    status: ResidualStatus::Open,
                },
                ProofResidual {
                    residual_ref: "r:b".into(),
                    proposition_ref: "p:b".into(),
                    producer_class_ref: "producer:b".into(),
                    jurisdiction_ref: Some("AU".into()),
                    authority_requirement_ref: None,
                    salience: 3,
                    dependency_refs: vec!["dep:shared".into()],
                    status: ResidualStatus::Open,
                },
            ],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    fn move_(id: &str, reduction: u64, network: u64) -> FrontierCandidateMove {
        FrontierCandidateMove {
            target_residual_refs: vec!["r:a".into(), "r:b".into()],
            move_: CandidateMove {
                move_ref: id.into(),
                strategy: ExecutionStrategy::PersistedAuthorityReceipt,
                source_ref: Some(format!("source:{id}")),
                provider_operation_ref: "local".into(),
                cost: ExecutionCostVector { network_requests: network, ..ExecutionCostVector::default() },
                value: ProofValueVector { expected_proof_reduction: reduction, authority_fitness: 3, ..ProofValueVector::default() },
                admissible: true,
                calibration_ref: "test".into(),
            },
            expected_whole_frontier_reduction: reduction,
            shared_dependency_gain: 1,
        }
    }

    #[test]
    fn one_move_may_target_multiple_open_residuals() {
        assert!(validate_frontier_move(&frontier(), &move_("shared", 2, 0)));
    }

    #[test]
    fn selector_prefers_higher_frontier_reduction_before_network_tie_break() {
        let low = move_("low", 1, 0);
        let high = move_("high", 3, 0);
        assert_eq!(select_frontier_move(&frontier(), &[low, high], 1).unwrap().move_.move_ref, "high");
    }
}
