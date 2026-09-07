//! Offline return path for the experimental SensibLaw proof-search scheduler.
//!
//! This crate consumes only already-local artefacts / persisted authority
//! receipts. It does not fetch, publish, admit, or grant legal/semantic authority.
//! The loop is:
//! selected offline move -> local artefact -> evidential/PNF bridge -> reviewed
//! correspondence -> frontier delta -> sparse wake -> optional reschedule.

use sensiblaw_evidential_reopen::{
    sparse_wake, CorrespondenceStatus, EvidentialBridgeReceipt, ReverseDependency,
    ReviewedCorrespondenceReceipt, SourceCoordinate, SourceCoordinateKind, WakeRequest,
};
use sensiblaw_proof_search_scheduler::{
    require_offline_execution, schedule, CandidateMove, CandidateMoveReceipt, ProofGap,
    SchedulerPolicy, SemanticAuthorityStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalArtifactReceipt {
    pub source_revision_ref: String,
    pub document_ref: String,
    pub local_artifact_ref: String,
    pub bytes_digest_ref: String,
    pub locally_ingested: bool,
    pub network_requests: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssessmentGrade {
    SupportsTargetCandidate,
    DefeatsTargetCandidate,
    ComparatorOnly,
    Ambiguous,
    NoContribution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontierDisposition {
    Narrowed,
    ClosedCandidate,
    Contested,
    Underidentified,
    Unchanged,
    AuthorityBlocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierDelta {
    pub consumer_ref: String,
    pub prior_residual_ref: String,
    pub disposition: FrontierDisposition,
    pub changed_coordinates: Vec<SourceCoordinate>,
    pub observed_proof_reduction: u64,
    pub semantic_authority: SemanticAuthorityStatus,
    pub delta_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineAssessmentReceipt {
    pub selected_move_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub proposition_ref: String,
    pub grade: AssessmentGrade,
    pub correspondence_status: CorrespondenceStatus,
    pub world_truth_claimed: bool,
    pub legal_holding_claimed: bool,
    pub frontier_delta: FrontierDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfflineLoopError {
    SelectedMoveRequiresGovernedLiveAdapter,
    LocalArtifactPerformedNetworkWork,
    LocalArtifactNotIngested,
    BridgeDocumentMismatch,
    CorrespondenceDocumentMismatch,
    CorrespondenceGraphMismatch,
    TargetPropositionMismatch,
    UnsupportedCorrespondenceGrade,
    NoAdmissibleMoveMeetsThreshold,
}

fn disposition_for(grade: AssessmentGrade) -> FrontierDisposition {
    match grade {
        AssessmentGrade::SupportsTargetCandidate => FrontierDisposition::Narrowed,
        AssessmentGrade::DefeatsTargetCandidate => FrontierDisposition::Contested,
        AssessmentGrade::ComparatorOnly => FrontierDisposition::Narrowed,
        AssessmentGrade::Ambiguous => FrontierDisposition::Underidentified,
        AssessmentGrade::NoContribution => FrontierDisposition::Unchanged,
    }
}

pub fn assess_offline_result(
    gap: &ProofGap,
    selected: &CandidateMoveReceipt,
    artifact: &LocalArtifactReceipt,
    bridge: &EvidentialBridgeReceipt,
    correspondence: &ReviewedCorrespondenceReceipt,
    grade: AssessmentGrade,
    observed_proof_reduction: u64,
) -> Result<OfflineAssessmentReceipt, OfflineLoopError> {
    require_offline_execution(selected)
        .map_err(|_| OfflineLoopError::SelectedMoveRequiresGovernedLiveAdapter)?;
    if artifact.network_requests != 0 {
        return Err(OfflineLoopError::LocalArtifactPerformedNetworkWork);
    }
    if !artifact.locally_ingested {
        return Err(OfflineLoopError::LocalArtifactNotIngested);
    }
    if bridge.document_ref != artifact.document_ref {
        return Err(OfflineLoopError::BridgeDocumentMismatch);
    }
    if correspondence.bridge_document_ref != bridge.document_ref {
        return Err(OfflineLoopError::CorrespondenceDocumentMismatch);
    }
    if correspondence.graph_ref != bridge.graph_ref {
        return Err(OfflineLoopError::CorrespondenceGraphMismatch);
    }
    if correspondence.proposition_ref != gap.missing_proposition_ref {
        return Err(OfflineLoopError::TargetPropositionMismatch);
    }

    let compatible = match grade {
        AssessmentGrade::SupportsTargetCandidate => {
            correspondence.status == CorrespondenceStatus::ReviewedSupported
        }
        AssessmentGrade::DefeatsTargetCandidate
        | AssessmentGrade::ComparatorOnly
        | AssessmentGrade::Ambiguous
        | AssessmentGrade::NoContribution => true,
    };
    if !compatible {
        return Err(OfflineLoopError::UnsupportedCorrespondenceGrade);
    }

    let changed_coordinates = if matches!(grade, AssessmentGrade::NoContribution) {
        Vec::new()
    } else {
        vec![SourceCoordinate {
            kind: SourceCoordinateKind::Proposition,
            stable_ref: correspondence.proposition_ref.clone(),
        }]
    };

    let frontier_delta = FrontierDelta {
        consumer_ref: gap.consumer_ref.clone(),
        prior_residual_ref: gap.residual_ref.clone(),
        disposition: disposition_for(grade),
        changed_coordinates,
        observed_proof_reduction,
        semantic_authority: SemanticAuthorityStatus::ExperimentalCandidateOnly,
        delta_authority: "experimental_candidate_only",
    };

    Ok(OfflineAssessmentReceipt {
        selected_move_ref: selected.selected_move_ref.clone(),
        source_revision_ref: artifact.source_revision_ref.clone(),
        document_ref: artifact.document_ref.clone(),
        proposition_ref: correspondence.proposition_ref.clone(),
        grade,
        correspondence_status: correspondence.status,
        world_truth_claimed: correspondence.world_truth_claimed,
        legal_holding_claimed: correspondence.legal_holding_claimed,
        frontier_delta,
    })
}

pub fn wake_from_delta(
    delta: &FrontierDelta,
    dependencies: &[ReverseDependency],
) -> Vec<WakeRequest> {
    sparse_wake(&delta.changed_coordinates, dependencies)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterationReceipt {
    pub assessment: OfflineAssessmentReceipt,
    pub wake_requests: Vec<WakeRequest>,
    pub next_move: Option<CandidateMoveReceipt>,
    pub iteration_authority: &'static str,
}

pub fn reschedule_after_delta(
    gap: &ProofGap,
    assessment: OfflineAssessmentReceipt,
    dependencies: &[ReverseDependency],
    candidates: &[CandidateMove],
    policy: SchedulerPolicy,
) -> Result<IterationReceipt, OfflineLoopError> {
    let wake_requests = wake_from_delta(&assessment.frontier_delta, dependencies);
    let terminal = matches!(
        assessment.frontier_delta.disposition,
        FrontierDisposition::ClosedCandidate | FrontierDisposition::AuthorityBlocked
    );
    let next_move = if terminal {
        None
    } else {
        Some(
            schedule(gap, candidates, policy)
                .map_err(|_| OfflineLoopError::NoAdmissibleMoveMeetsThreshold)?,
        )
    };

    Ok(IterationReceipt {
        assessment,
        wake_requests,
        next_move,
        iteration_authority: "experimental_candidate_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_evidential_reopen::{ConsumerFibreKey, Horizon};
    use sensiblaw_proof_search_scheduler::{
        CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector,
    };

    fn gap() -> ProofGap {
        ProofGap {
            consumer_ref: "consumer:pabai-duty-route".into(),
            residual_ref: "residual:positive-operational-act-comparator".into(),
            missing_proposition_ref: "prop:cullen-positive-operational-act".into(),
            required_producer_ref: "comparator-producer".into(),
            jurisdiction_ref: Some("AU".into()),
        }
    }

    fn selected() -> CandidateMoveReceipt {
        CandidateMoveReceipt {
            consumer_ref: gap().consumer_ref,
            residual_ref: gap().residual_ref,
            selected_move_ref: "move:persisted-cullen-comparator".into(),
            selected_strategy: ExecutionStrategy::PersistedAuthorityReceipt,
            execution_permission: sensiblaw_proof_search_scheduler::ExecutionPermission::OfflineExecutionAllowed,
            semantic_authority: SemanticAuthorityStatus::ExperimentalCandidateOnly,
            frontier_move_refs: vec!["move:persisted-cullen-comparator".into()],
            threshold: 1,
            receipt_authority: "experimental_candidate_only",
        }
    }

    fn artifact() -> LocalArtifactReceipt {
        LocalArtifactReceipt {
            source_revision_ref: "source:cullen:rev:1".into(),
            document_ref: "doc:cullen".into(),
            local_artifact_ref: "fixture:cullen".into(),
            bytes_digest_ref: "sha256:cullen".into(),
            locally_ingested: true,
            network_requests: 0,
        }
    }

    fn bridge() -> EvidentialBridgeReceipt {
        EvidentialBridgeReceipt::new(
            "run:offline-cullen".into(),
            "doc:cullen".into(),
            "sha256:cullen".into(),
            "parser:fixture".into(),
            "numeric-pnf:fixture".into(),
            "graph:cullen".into(),
            vec!["residual:positive-operational-act-comparator".into()],
            "local-fixture-pnf".into(),
            true,
            false,
            false,
        )
        .unwrap()
    }

    fn correspondence(bridge: &EvidentialBridgeReceipt) -> ReviewedCorrespondenceReceipt {
        ReviewedCorrespondenceReceipt::supported(
            bridge,
            "span:cullen:1".into(),
            "prop:cullen-positive-operational-act".into(),
            "reviewer:fixture".into(),
            vec!["source:cullen:rev:1".into()],
        )
    }

    #[test]
    fn persisted_receipt_reenters_pnf_and_narrows_frontier_without_authority_claim() {
        let bridge = bridge();
        let receipt = assess_offline_result(
            &gap(),
            &selected(),
            &artifact(),
            &bridge,
            &correspondence(&bridge),
            AssessmentGrade::ComparatorOnly,
            2,
        )
        .unwrap();
        assert_eq!(receipt.frontier_delta.disposition, FrontierDisposition::Narrowed);
        assert_eq!(receipt.frontier_delta.observed_proof_reduction, 2);
        assert!(!receipt.world_truth_claimed);
        assert!(!receipt.legal_holding_claimed);
        assert_eq!(receipt.frontier_delta.delta_authority, "experimental_candidate_only");
    }

    #[test]
    fn local_return_path_rejects_any_network_work() {
        let bridge = bridge();
        let mut bad_artifact = artifact();
        bad_artifact.network_requests = 1;
        assert_eq!(
            assess_offline_result(
                &gap(),
                &selected(),
                &bad_artifact,
                &bridge,
                &correspondence(&bridge),
                AssessmentGrade::ComparatorOnly,
                1,
            ),
            Err(OfflineLoopError::LocalArtifactPerformedNetworkWork)
        );
    }

    #[test]
    fn delta_wakes_only_declared_consumer_dependency() {
        let bridge = bridge();
        let receipt = assess_offline_result(
            &gap(),
            &selected(),
            &artifact(),
            &bridge,
            &correspondence(&bridge),
            AssessmentGrade::ComparatorOnly,
            2,
        )
        .unwrap();
        let source = receipt.frontier_delta.changed_coordinates[0].clone();
        let interested = ConsumerFibreKey {
            demand_ref: "d:pabai".into(),
            consumer_ref: gap().consumer_ref,
            query_ref: "q:pabai".into(),
            policy_ref: "policy:offline".into(),
        };
        let dependencies = vec![ReverseDependency {
            source,
            consumer: interested.clone(),
            minimum_horizon: Horizon::H6.code(),
        }];
        let wakes = wake_from_delta(&receipt.frontier_delta, &dependencies);
        assert_eq!(wakes.len(), 1);
        assert_eq!(wakes[0].consumer, interested);
    }

    #[test]
    fn narrowed_frontier_can_reschedule_next_offline_move() {
        let bridge = bridge();
        let assessment = assess_offline_result(
            &gap(),
            &selected(),
            &artifact(),
            &bridge,
            &correspondence(&bridge),
            AssessmentGrade::ComparatorOnly,
            2,
        )
        .unwrap();
        let candidate = CandidateMove {
            move_ref: "move:next-persisted-authority".into(),
            strategy: ExecutionStrategy::PersistedAuthorityReceipt,
            source_ref: Some("source:next:rev:1".into()),
            provider_operation_ref: "persisted_authority_receipt".into(),
            cost: ExecutionCostVector {
                local_bytes_read_cost: 1,
                parser_pnf_cost: 1,
                semantic_assessment_cost: 1,
                ..ExecutionCostVector::default()
            },
            value: ProofValueVector {
                expected_proof_reduction: 2,
                authority_fitness: 4,
                discriminative_value: 2,
                ..ProofValueVector::default()
            },
            admissible: true,
            calibration_ref: "fixture:next".into(),
        };
        let iteration = reschedule_after_delta(
            &gap(),
            assessment,
            &[],
            &[candidate],
            SchedulerPolicy::default(),
        )
        .unwrap();
        assert_eq!(
            iteration.next_move.unwrap().selected_move_ref,
            "move:next-persisted-authority"
        );
    }
}
