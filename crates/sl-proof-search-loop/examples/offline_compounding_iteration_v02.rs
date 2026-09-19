use sensiblaw_proof_search_loop::engine::{
    plan_local_frontier_research, DeclaredResearchValue,
};
use sensiblaw_proof_search_loop::frontier::{
    select_frontier_move, ProofFrontier, ProofResidual, ResidualStatus,
};
use sensiblaw_proof_search_loop::hypothesis::families_for_frontier;
use sensiblaw_proof_search_loop::local::{LocalDocument, LocalIndex};
use sensiblaw_proof_search_loop::planner::QueryLexicalContext;
use sensiblaw_proof_search_loop::receipt::{
    build_frontier_iteration_receipt_v02, build_iteration_receipt,
    FRONTIER_ITERATION_SCHEMA_V02,
};
use sensiblaw_proof_search_loop::transition::{
    apply_assessments, ResidualAssessment, ResidualAssessmentKind, ResearchTermination,
};
use sensiblaw_proof_search_loop::world::ResearchWorldSnapshot;
use sensiblaw_proof_search_scheduler::{schedule, ExecutionCostVector, SchedulerPolicy};
use std::collections::BTreeMap;

fn main() {
    let frontier = ProofFrontier {
        consumer_ref: "consumer:pabai-duty-route".into(),
        frontier_ref: "frontier:pabai:v1".into(),
        residuals: vec![
            ProofResidual {
                residual_ref: "residual:positive-operational-act-comparator".into(),
                proposition_ref: "prop:cullen-positive-operational-act".into(),
                producer_class_ref: "comparator-producer".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("primary-case".into()),
                salience: 5,
                dependency_refs: vec!["dep:government-duty-discriminator".into()],
                status: ResidualStatus::Open,
            },
            ProofResidual {
                residual_ref: "residual:current-treatment".into(),
                proposition_ref: "prop:current-treatment-of-duty-authority".into(),
                producer_class_ref: "authority-treatment-producer".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("current-appellate-treatment".into()),
                salience: 4,
                dependency_refs: vec!["dep:government-duty-discriminator".into()],
                status: ResidualStatus::Open,
            },
        ],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    let mut world = ResearchWorldSnapshot {
        snapshot_ref: "world:pabai:v1".into(),
        authority: "experimental_candidate_only",
        ..ResearchWorldSnapshot::default()
    };
    world
        .query_vocabulary
        .insert("positive operational act".into());
    world
        .authority_neighbourhood
        .insert("authority:donoghue".into());

    let mut index = LocalIndex::default();
    index.insert(LocalDocument {
        document_ref: "doc:cullen".into(),
        source_revision_ref: "source:cullen:rev:1".into(),
        jurisdiction_ref: Some("AU".into()),
        court_ref: Some("HCA".into()),
        date_ref: Some("2015".into()),
        canonical_text:
            "The positive operational act was considered in addressing the asserted duty of care."
                .into(),
        citation_refs: vec!["authority:donoghue".into()],
    });

    let hypotheses = families_for_frontier(&frontier);
    let mut lexical_contexts = BTreeMap::new();
    let mut declared_values = BTreeMap::new();
    for hypothesis in &hypotheses {
        lexical_contexts.insert(
            hypothesis.hypothesis_ref.clone(),
            QueryLexicalContext {
                target_phrases: vec!["positive operational act".into()],
                support_terms: vec!["duty".into()],
                defeater_terms: vec!["policy".into()],
                comparator_terms: vec!["operational".into()],
                contradiction_terms: vec!["no duty".into()],
                treatment_terms: vec!["followed".into(), "distinguished".into()],
                citation_seeds: vec!["authority:donoghue".into()],
                maximum_world_terms: 4,
                maximum_world_authorities: 4,
                ..QueryLexicalContext::default()
            },
        );
        declared_values.insert(
            hypothesis.hypothesis_ref.clone(),
            DeclaredResearchValue {
                expected_residual_reduction: 2,
                discriminative_value: 4,
                authority_fitness: 4,
                novelty: 1,
                coverage_gain: 1,
                execution_cost: ExecutionCostVector {
                    local_bytes_read_cost: 1,
                    semantic_assessment_cost: 1,
                    ..ExecutionCostVector::default()
                },
                calibration_ref: "fixture:offline-compounding-v02".into(),
            },
        );
    }

    let planned = plan_local_frontier_research(
        &frontier,
        &world,
        &index,
        &lexical_contexts,
        &declared_values,
    )
    .expect("explicit lexical contexts and values should plan");
    assert!(!planned.is_empty());

    let frontier_moves = planned
        .iter()
        .map(|planned| planned.frontier_move.clone())
        .collect::<Vec<_>>();
    let selected_frontier = select_frontier_move(&frontier, &frontier_moves, 1)
        .expect("at least one local whole-frontier move should survive threshold");

    let selected_move = selected_frontier.move_.clone();
    let first_gap = frontier.to_gaps().into_iter().next().expect("open residual");
    let selected = schedule(
        &first_gap,
        std::slice::from_ref(&selected_move),
        SchedulerPolicy::default(),
    )
    .expect("selected local move should satisfy scheduler threshold");

    let (next_frontier, transition) = apply_assessments(
        &frontier,
        "frontier:pabai:v2",
        &[ResidualAssessment {
            residual_ref: "residual:positive-operational-act-comparator".into(),
            kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_proof_reduction: 2,
            assessment_ref: "assessment:cullen-comparator".into(),
            assessment_authority: "experimental_candidate_only",
        }],
    )
    .expect("candidate-only assessment should update one residual");
    assert_eq!(transition.termination, ResearchTermination::Continue);
    assert_eq!(next_frontier.open_residuals().count(), 1);

    let candidates = planned
        .iter()
        .map(|planned| planned.frontier_move.move_.clone())
        .collect::<Vec<_>>();
    let base = build_iteration_receipt(
        "runtime:offline-compounding-v02",
        &frontier,
        &candidates,
        &selected,
        &selected_move,
        Some("sha256:cullen".into()),
        Some("pnf:cullen".into()),
        Some("assessment:cullen-comparator".into()),
        Some("delta:frontier:pabai:v2".into()),
        vec![],
        Some("next:current-treatment".into()),
        "digest:input:v1".into(),
        "digest:output:v2".into(),
    );
    let receipt = build_frontier_iteration_receipt_v02(
        base,
        &transition,
        &world,
        vec!["reasoning:cullen:positive-operational-act".into()],
    );
    assert_eq!(receipt.schema_version, FRONTIER_ITERATION_SCHEMA_V02);
    assert_eq!(receipt.base_iteration.execution_cost.network_requests, 0);
    assert_eq!(receipt.termination, ResearchTermination::Continue);
    assert_eq!(receipt.authority_boundary, "experimental_candidate_only");

    println!("{}", receipt.to_canonical_json());
}
