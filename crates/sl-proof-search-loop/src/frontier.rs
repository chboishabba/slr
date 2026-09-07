use sensiblaw_proof_search_scheduler::{CandidateMove, ProofGap};
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

    #[test]
    fn one_move_may_target_multiple_open_residuals() {
        let candidate = FrontierCandidateMove {
            target_residual_refs: vec!["r:a".into(), "r:b".into()],
            move_: CandidateMove {
                move_ref: "move:shared".into(),
                strategy: ExecutionStrategy::PersistedAuthorityReceipt,
                source_ref: Some("source:1".into()),
                provider_operation_ref: "local".into(),
                cost: ExecutionCostVector::default(),
                value: ProofValueVector::default(),
                admissible: true,
                calibration_ref: "test".into(),
            },
            expected_whole_frontier_reduction: 2,
            shared_dependency_gain: 1,
        };
        assert!(validate_frontier_move(&frontier(), &candidate));
    }
}
