use sensiblaw_evidential_reopen::{EvidentialBridgeReceipt, ReviewedCorrespondenceReceipt};
use sensiblaw_proof_search_loop::{
    assess_offline_result, reschedule_after_delta, AssessmentGrade, LocalArtifactReceipt,
};
use sensiblaw_proof_search_scheduler::{
    schedule, CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofGap, ProofValueVector,
    SchedulerPolicy,
};
use std::{env, fs};

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

fn disposition_name(disposition: impl std::fmt::Debug) -> String {
    format!("{disposition:?}")
}

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
    let selected = schedule(&gap, std::slice::from_ref(&first), SchedulerPolicy::default())
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
        &first,
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
    let output = env::args()
        .nth(1)
        .unwrap_or_else(|| "target/offline_pabai_loop.json".into());
    let receipt = format!(
        "{{\n  \"schema_version\": \"sensiblaw.offline-proof-search-loop-receipt.v0_1\",\n  \"authority\": \"experimental_candidate_only\",\n  \"network_requests\": 0,\n  \"proof_gap\": {{\n    \"consumer_ref\": \"{}\",\n    \"residual_ref\": \"{}\",\n    \"missing_proposition_ref\": \"{}\"\n  }},\n  \"selected_move\": {{\n    \"move_ref\": \"{}\",\n    \"source_revision_ref\": \"{}\",\n    \"strategy\": \"PersistedAuthorityReceipt\"\n  }},\n  \"local_artifact\": {{\n    \"artifact_ref\": \"{}\",\n    \"document_ref\": \"{}\",\n    \"source_revision_ref\": \"{}\",\n    \"bytes_digest_ref\": \"{}\",\n    \"network_requests\": {}\n  }},\n  \"bridge\": {{\n    \"document_ref\": \"{}\",\n    \"canonical_text_sha256\": \"{}\",\n    \"graph_ref\": \"{}\"\n  }},\n  \"correspondence\": {{\n    \"document_ref\": \"{}\",\n    \"graph_ref\": \"{}\",\n    \"proposition_ref\": \"{}\",\n    \"world_truth_claimed\": {},\n    \"legal_holding_claimed\": {}\n  }},\n  \"assessment\": {{\n    \"grade\": \"ComparatorOnly\",\n    \"correspondence_status\": \"ReviewedSupported\",\n    \"observed_proof_reduction\": {},\n    \"disposition\": \"{}\",\n    \"changed_coordinates\": [\"{}\"]\n  }},\n  \"wake_requests\": [],\n  \"next_move\": {{\n    \"move_ref\": \"{}\",\n    \"strategy\": \"PersistedAuthorityReceipt\"\n  }},\n  \"delta_authority\": \"{}\",\n  \"iteration_authority\": \"{}\"\n}}\n",
        json_escape(&gap.consumer_ref),
        json_escape(&gap.residual_ref),
        json_escape(&gap.missing_proposition_ref),
        json_escape(&selected.selected_move_ref),
        json_escape(&first.source_ref.clone().expect("source ref")),
        json_escape(&artifact.local_artifact_ref),
        json_escape(&artifact.document_ref),
        json_escape(&artifact.source_revision_ref),
        json_escape(&artifact.bytes_digest_ref),
        artifact.network_requests,
        json_escape(&bridge.document_ref),
        json_escape(&bridge.canonical_text_sha256),
        json_escape(&bridge.graph_ref),
        json_escape(&correspondence.bridge_document_ref),
        json_escape(&correspondence.graph_ref),
        json_escape(&correspondence.proposition_ref),
        correspondence.world_truth_claimed,
        correspondence.legal_holding_claimed,
        iteration.assessment.frontier_delta.observed_proof_reduction,
        disposition_name(iteration.assessment.frontier_delta.disposition),
        json_escape(&correspondence.proposition_ref),
        json_escape(&next_move.selected_move_ref),
        iteration.assessment.frontier_delta.delta_authority,
        iteration.iteration_authority,
    );
    if let Some(parent) = std::path::Path::new(&output).parent() {
        fs::create_dir_all(parent).expect("create receipt directory");
    }
    fs::write(&output, receipt).expect("write JSON receipt");
    println!(
        "first={} delta={:?} next={} authority={} network=0 receipt={}",
        selected.selected_move_ref,
        iteration.assessment.frontier_delta.disposition,
        next_move.selected_move_ref,
        iteration.iteration_authority,
        output,
    );
}
