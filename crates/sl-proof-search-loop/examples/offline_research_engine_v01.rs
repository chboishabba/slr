use sensiblaw_proof_search_loop::frontier::{
    select_frontier_move, FrontierCandidateMove, ProofFrontier, ProofResidual, ResidualStatus,
};
use sensiblaw_proof_search_loop::hypothesis::families_for_frontier;
use sensiblaw_proof_search_loop::local::{LocalDocument, LocalIndex};
use sensiblaw_proof_search_loop::query::{compile_austlii, compile_local, QueryExpr};
use sensiblaw_proof_search_loop::reasoning::{
    CitationUse, ConditionCoordinate, ConditionKind, PropositionReasoningEdge, ReasoningGraphDelta,
    ReasoningRole,
};
use sensiblaw_proof_search_loop::receipt::{build_iteration_receipt, ITERATION_SCHEMA};
use sensiblaw_proof_search_loop::world::{ResearchWorldSnapshot, SourceRevisionRecord};
use sensiblaw_proof_search_scheduler::{
    schedule, CandidateMove, ExecutionCostVector, ExecutionStrategy, ProofValueVector, SchedulerPolicy,
};

fn main() {
    let frontier = ProofFrontier {
        consumer_ref: "consumer:pabai-duty-route".into(),
        frontier_ref: "frontier:pabai:v0".into(),
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

    let hypotheses = families_for_frontier(&frontier);
    assert!(hypotheses.len() >= 10);

    let query = QueryExpr::All(vec![
        QueryExpr::Phrase("positive operational act".into()),
        QueryExpr::Near {
            left: Box::new(QueryExpr::Term("duty".into())),
            right: Box::new(QueryExpr::Term("policy".into())),
            words: 20,
        },
    ]);
    let provider_query = compile_austlii(&query);
    assert_eq!(provider_query.network_requests, 0);
    let _local_plan = compile_local(&query);

    let mut index = LocalIndex::default();
    index.insert(LocalDocument {
        document_ref: "doc:cullen".into(),
        source_revision_ref: "source:cullen:rev:1".into(),
        jurisdiction_ref: Some("AU".into()),
        court_ref: Some("HCA".into()),
        date_ref: Some("2015-01-01".into()),
        canonical_text: "A positive operational act may bear on duty where policy concerns otherwise remain.".into(),
        citation_refs: vec!["authority:donoghue".into()],
    });
    let passages = index.search(&query);
    assert_eq!(passages.len(), 1);

    let persisted = CandidateMove {
        move_ref: "move:persisted-cullen".into(),
        strategy: ExecutionStrategy::PersistedAuthorityReceipt,
        source_ref: Some("source:cullen:rev:1".into()),
        provider_operation_ref: "local-query".into(),
        cost: ExecutionCostVector {
            local_bytes_read_cost: 1,
            parser_pnf_cost: 1,
            semantic_assessment_cost: 1,
            ..ExecutionCostVector::default()
        },
        value: ProofValueVector {
            expected_proof_reduction: 3,
            discriminative_value: 5,
            authority_fitness: 5,
            novelty: 1,
            coverage_gain: 2,
        },
        admissible: true,
        calibration_ref: "fixture:offline-research-engine-v01".into(),
    };
    let frontier_move = FrontierCandidateMove {
        target_residual_refs: frontier.open_residuals().map(|r| r.residual_ref.clone()).collect(),
        move_: persisted.clone(),
        expected_whole_frontier_reduction: 2,
        shared_dependency_gain: 1,
    };
    let selected_frontier = select_frontier_move(&frontier, &[frontier_move], 1)
        .expect("persisted local move should be selectable");

    let first_gap = frontier.to_gaps().into_iter().next().expect("open gap");
    let selected = schedule(&first_gap, &[persisted.clone()], SchedulerPolicy::default())
        .expect("persisted move should schedule");

    let edge = PropositionReasoningEdge {
        citing_document_ref: "doc:cullen".into(),
        citing_proposition_ref: "prop:cullen-positive-operational-act".into(),
        cited_document_ref: "authority:donoghue".into(),
        cited_proposition_ref: "prop:neighbour-principle".into(),
        citation_use: CitationUse::Distinguished,
        reasoning_role: ReasoningRole::Distinction,
        condition_coordinates: vec![ConditionCoordinate {
            kind: ConditionKind::Factual,
            condition_ref: "cond:positive-operational-act".into(),
        }],
        pinpoint_ref: Some("[42]".into()),
        judge_or_speaker_ref: Some("judge:fixture".into()),
        court_ref: Some("HCA".into()),
        jurisdiction_ref: Some("AU".into()),
        temporal_ref: Some("2015".into()),
        outcome_ref: Some("claim-dismissed".into()),
        remedy_ref: None,
        burden_refs: vec![],
        exception_refs: vec!["exception:core-policy".into()],
        lexical_realisation: "positive operational act".into(),
        reviewed: true,
        candidate_only: true,
    };
    let delta = ReasoningGraphDelta::from_edges(vec![edge]);

    let mut world = ResearchWorldSnapshot {
        snapshot_ref: "world:pabai:v0".into(),
        authority: "experimental_candidate_only",
        ..ResearchWorldSnapshot::default()
    };
    world.append_source(SourceRevisionRecord {
        source_revision_ref: "source:cullen:rev:1".into(),
        canonical_bytes_digest: "sha256:cullen".into(),
        canonical_text_digest: "sha256:cullen".into(),
        provider_receipt_ref: "provider:persisted".into(),
        jurisdiction_ref: Some("AU".into()),
        source_role_ref: "primary-case".into(),
        authority_candidate_ref: Some("authority:cullen".into()),
        parsed_pnf_ref: "pnf:cullen".into(),
        citation_topology_ref: "citations:cullen".into(),
        assessment_receipt_refs: vec!["assessment:cullen-comparator".into()],
    }).expect("append-only source insert");
    world.apply_reasoning_delta(delta);
    assert!(world.query_vocabulary.contains("positive operational act"));
    assert!(world.authority_neighbourhood.contains("authority:donoghue"));

    let receipt = build_iteration_receipt(
        "runtime:offline-research-engine-v01",
        &frontier,
        &[persisted.clone()],
        &selected,
        &persisted,
        Some("sha256:cullen".into()),
        Some("pnf:cullen".into()),
        Some("assessment:cullen-comparator".into()),
        Some("delta:frontier-narrowed".into()),
        vec![],
        Some("move:persisted-next-authority".into()),
        "digest:input:v0".into(),
        "digest:output:v0".into(),
    );
    assert_eq!(receipt.schema_version, ITERATION_SCHEMA);
    assert_eq!(receipt.execution_cost.network_requests, 0);
    assert_eq!(receipt.authority_boundary, "experimental_candidate_only");

    println!(
        "selected={} gaps={} hypotheses={} local_hits={} learned_terms={} learned_authorities={} network={} authority={} json={}",
        selected_frontier.move_.move_ref,
        frontier.open_residuals().count(),
        hypotheses.len(),
        passages.len(),
        world.query_vocabulary.len(),
        world.authority_neighbourhood.len(),
        receipt.execution_cost.network_requests,
        receipt.authority_boundary,
        receipt.to_canonical_json(),
    );
}
