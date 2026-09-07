use sensiblaw_evidential_reopen::{
    EvidentialBridgeReceipt, ReviewedCorrespondenceReceipt,
};
use sensiblaw_proof_search_loop::{
    assess_offline_result, reschedule_after_delta, AssessmentGrade, LocalArtifactReceipt,
};
use sensiblaw_proof_search_scheduler::{
    schedule, CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofGap, ProofValueVector,
    SchedulerPolicy,
};

fn persisted_move(move_ref: &str, source_ref: &str, reduction: u64) -> CandidateMove {
    CandidateMove {
        move_ref: move_ref.into(),
        strategy: ExecutionStrategy::PersistedAuthorityReceipt,
        source_ref: Some(source_ref.into()),
        provider_operation_ref: "persisted_authority_receipt".into(),
        cost: ExecutionCostVector {
            local_bytes_read_cost: 1,
            parser_pnf_cost: 1,
            semantic_assessment_cost: 1,
            ..ExecutionCostVector::default()
        },
        value: ProofValueVector {
            expected_proof_reduction: reduction,
            discriminative_value: 4,
            authority_fitness: 5,
            novelty: 1,
            coverage_gain: 2,
        },
        admissible: true,
        calibration_ref: "fixture:pabai-offline-loop".into(),
    }
}

fn main() {
    let gap = ProofGap {
        consumer_ref: "consumer:pabai-duty-route".into(),
        residual_ref: "residual:positive-operational-act-comparator".into(),
        missing_proposition_ref: "prop:cullen-positive-operational-act".into(),
        required_producer_ref: "comparator-producer".into(),
        jurisdiction_ref: Some("AU".into()),
    };

    let first = persisted_move(
        "move:persisted-cullen-comparator",
        "source:cullen:rev:1",
        3,
    );
    let selected = schedule(&gap, &[first], SchedulerPolicy::default())
        .expect("persisted comparator should be selected");

    let artifact = LocalArtifactReceipt {
        source_revision_ref: "source:cullen:rev:1".into(),
        document_ref: "doc:cullen".into(),
        local_artifact_ref: "fixture:cullen".into(),
        bytes_digest_ref: "sha256:cullen".into(),
        locally_ingested: true,
        network_requests: 0,
    };
    let bridge = EvidentialBridgeReceipt::new(
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
    .expect("valid local evidential bridge");
    let correspondence = ReviewedCorrespondenceReceipt::supported(
        &bridge,
        "span:cullen:1".into(),
        "prop:cullen-positive-operational-act".into(),
        "reviewer:fixture".into(),
        vec!["source:cullen:rev:1".into()],
    );

    let assessment = assess_offline_result(
        &gap,
        &selected,
        &artifact,
        &bridge,
        &correspondence,
        AssessmentGrade::ComparatorOnly,
        2,
    )
    .expect("local comparator should produce a candidate frontier delta");

    let next = persisted_move(
        "move:persisted-next-authority",
        "source:next:rev:1",
        2,
    );
    let iteration = reschedule_after_delta(
        &gap,
        assessment,
        &[],
        &[next],
        SchedulerPolicy::default(),
    )
    .expect("narrowed frontier should schedule the next offline move");

    let next_move = iteration.next_move.expect("non-terminal fixture should continue");
    println!(
        "first={} delta={:?} next={} authority={} network=0",
        selected.selected_move_ref,
        iteration.assessment.frontier_delta.disposition,
        next_move.selected_move_ref,
        iteration.iteration_authority,
    );
}
