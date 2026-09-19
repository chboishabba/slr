use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::hypothesis::{family_for_residual, SearchHypothesisKind};
use sensiblaw_proof_search_loop::local::{LocalDocument, LocalIndex};
use sensiblaw_proof_search_loop::planner::{
    deterministic_query_key, execute_local_candidates, synthesize_queries, QueryLexicalContext,
};
use sensiblaw_proof_search_loop::receipt::{
    build_frontier_iteration_receipt_v02, build_iteration_receipt,
};
use sensiblaw_proof_search_loop::transition::{
    apply_assessments, ResidualAssessment, ResidualAssessmentKind, ResearchTermination,
};
use sensiblaw_proof_search_loop::world::ResearchWorldSnapshot;
use sensiblaw_proof_search_scheduler::{
    schedule, CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector,
    SchedulerPolicy,
};
use std::fs;

fn main() {
    // This is the surviving frontier from the previous v0.2 compounding receipt.
    let frontier = ProofFrontier {
        consumer_ref: "consumer:pabai-duty-route".into(),
        frontier_ref: "frontier:pabai:v2".into(),
        residuals: vec![ProofResidual {
            residual_ref: "residual:current-treatment".into(),
            proposition_ref: "prop:current-treatment-of-duty-authority".into(),
            producer_class_ref: "authority-treatment-producer".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("current-appellate-treatment".into()),
            salience: 4,
            dependency_refs: vec!["dep:government-duty-discriminator".into()],
            status: ResidualStatus::Open,
        }],
        satisfied_payment_refs: vec!["assessment:cullen-comparator".into()],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    // Reconstruct only knowledge learned by the prior receipt; Donoghue is not
    // introduced as a fresh external seed in this iteration.
    let mut world = ResearchWorldSnapshot {
        snapshot_ref: "world:pabai:v2".into(),
        authority: "experimental_candidate_only",
        ..ResearchWorldSnapshot::default()
    };
    world
        .query_vocabulary
        .insert("positive operational act".into());
    world
        .authority_neighbourhood
        .insert("authority:donoghue".into());
    world
        .iteration_receipt_refs
        .push("receipt:offline-compounding-iteration-v02".into());

    let residual = &frontier.residuals[0];
    let treatment = family_for_residual(residual)
        .into_iter()
        .find(|hypothesis| hypothesis.kind == SearchHypothesisKind::AuthorityTreatment)
        .expect("open treatment residual must generate treatment hypothesis");

    let lexical = QueryLexicalContext {
        target_phrases: vec!["duty of care".into()],
        treatment_terms: vec!["followed".into(), "distinguished".into(), "applied".into()],
        maximum_world_authorities: 4,
        maximum_world_terms: 4,
        ..QueryLexicalContext::default()
    };

    let synthesized = synthesize_queries(&treatment, &lexical, &world);
    assert_eq!(synthesized.len(), 1);
    let rendered = deterministic_query_key(&synthesized[0]);
    assert!(rendered.contains("authority:donoghue"));

    let mut index = LocalIndex::default();
    index.insert(LocalDocument {
        document_ref: "doc:later-duty-treatment".into(),
        source_revision_ref: "source:later-duty-treatment:rev:1".into(),
        jurisdiction_ref: Some("AU".into()),
        court_ref: Some("HCA".into()),
        date_ref: Some("2020".into()),
        canonical_text: "The duty of care analysis applied and distinguished the earlier authority in its present context.".into(),
        citation_refs: vec!["authority:donoghue".into()],
    });

    let executions = execute_local_candidates(&index, &synthesized);
    assert_eq!(executions.len(), 1);
    assert_eq!(executions[0].network_requests, 0);
    assert_eq!(executions[0].passages.len(), 1);

    let selected_move = CandidateMove {
        move_ref: format!("move:local-treatment:{}", rendered),
        strategy: ExecutionStrategy::LocalWorldGraph,
        source_ref: Some("source:later-duty-treatment:rev:1".into()),
        provider_operation_ref: "local-authority-treatment-query".into(),
        cost: ExecutionCostVector {
            local_bytes_read_cost: 1,
            semantic_assessment_cost: 1,
            ..ExecutionCostVector::default()
        },
        value: ProofValueVector {
            expected_proof_reduction: 2,
            discriminative_value: 4,
            authority_fitness: 4,
            novelty: 1,
            coverage_gain: 1,
        },
        admissible: true,
        calibration_ref: "fixture:current-treatment-from-learned-authority".into(),
    };

    let gap = frontier.to_gaps().into_iter().next().expect("open treatment gap");
    let selected = schedule(
        &gap,
        std::slice::from_ref(&selected_move),
        SchedulerPolicy::default(),
    )
    .expect("local treatment move should schedule");

    let (next_frontier, transition) = apply_assessments(
        &frontier,
        "frontier:pabai:v3",
        &[ResidualAssessment {
            residual_ref: "residual:current-treatment".into(),
            kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_proof_reduction: 2,
            assessment_ref: "assessment:current-treatment-candidate".into(),
            assessment_authority: "experimental_candidate_only",
        }],
    )
    .expect("candidate treatment assessment should update frontier");
    assert_eq!(transition.termination, ResearchTermination::ClosedCandidate);
    assert_eq!(next_frontier.open_residuals().count(), 0);

    world.snapshot_ref = "world:pabai:v3".into();
    world
        .query_vocabulary
        .insert("applied and distinguished".into());
    world
        .authority_neighbourhood
        .insert("source:later-duty-treatment:rev:1".into());

    let base = build_iteration_receipt(
        "runtime:offline-current-treatment-v03",
        &frontier,
        std::slice::from_ref(&selected_move),
        &selected,
        &selected_move,
        Some("sha256:later-duty-treatment".into()),
        Some("pnf:later-duty-treatment".into()),
        Some("assessment:current-treatment-candidate".into()),
        Some("delta:frontier:pabai:v3".into()),
        vec![],
        None,
        "digest:input:v2".into(),
        "digest:output:v3".into(),
    );
    let receipt = build_frontier_iteration_receipt_v02(
        base,
        &transition,
        &world,
        vec!["reasoning:later-duty-treatment".into()],
    );
    assert_eq!(receipt.base_iteration.execution_cost.network_requests, 0);
    assert_eq!(receipt.authority_boundary, "experimental_candidate_only");

    let json = receipt.to_canonical_json();
    let path = std::env::var("SLR_CURRENT_TREATMENT_RECEIPT")
        .unwrap_or_else(|_| "/tmp/offline-current-treatment-iteration-v03.json".into());
    fs::write(&path, &json).expect("write current-treatment receipt");

    println!(
        "prior=frontier:pabai:v2 next={} learned_authority=authority:donoghue local_hits={} termination={:?} network={} authority={} receipt={}",
        transition.next_frontier_ref,
        executions[0].passages.len(),
        transition.termination,
        receipt.base_iteration.execution_cost.network_requests,
        receipt.authority_boundary,
        path,
    );
}
