use sensiblaw_proof_search_scheduler::{
    require_offline_execution, schedule, CandidateMove, ExecutionCostVector, ExecutionStrategy,
    ProofGap, ProofValueVector, SchedulerPolicy,
};

fn main() {
    let gap = ProofGap {
        consumer_ref: "consumer:pabai-duty-route".into(),
        residual_ref: "residual:authority-treatment".into(),
        missing_proposition_ref: "prop:current-treatment-of-authority".into(),
        required_producer_ref: "authority-treatment-producer".into(),
        jurisdiction_ref: Some("AU".into()),
    };

    let persisted = CandidateMove {
        move_ref: "move:persisted-cullen-comparator".into(),
        strategy: ExecutionStrategy::PersistedAuthorityReceipt,
        source_ref: Some("fixture:cullen-authority-receipt".into()),
        provider_operation_ref: "persisted_authority_receipt".into(),
        cost: ExecutionCostVector {
            local_bytes_read_cost: 1,
            parser_pnf_cost: 1,
            semantic_assessment_cost: 1,
            ..ExecutionCostVector::default()
        },
        value: ProofValueVector {
            expected_proof_reduction: 3,
            discriminative_value: 4,
            authority_fitness: 5,
            novelty: 1,
            coverage_gain: 2,
        },
        admissible: true,
        calibration_ref: "fixture:pabai-cullen-local".into(),
    };

    let live = CandidateMove {
        move_ref: "move:live-cited-by-search".into(),
        strategy: ExecutionStrategy::GovernedLiveReferenceSearch,
        source_ref: None,
        provider_operation_ref: "jade-or-austlii-cited-by-candidate".into(),
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
            expected_proof_reduction: 3,
            discriminative_value: 4,
            authority_fitness: 5,
            novelty: 2,
            coverage_gain: 3,
        },
        admissible: true,
        calibration_ref: "candidate-only:governed-live".into(),
    };

    let receipt = schedule(&gap, &[persisted, live], SchedulerPolicy::default())
        .expect("fixture has at least one admissible useful move");
    require_offline_execution(&receipt)
        .expect("offline fixture must select an offline-executable candidate");

    println!("{receipt:?}");
}
