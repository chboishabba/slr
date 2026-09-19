use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_pg_source_store::{DiscoveryIdentityBaseline, LatentWorldRows};
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::{ProducerLane, ResidualClass};
use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    compile_mabo_heterogeneous_frontier, select_next_mabo_heterogeneous_decision,
    DurableAdaptiveNegativeAssessment, DurableAdaptiveNegativeKind,
    MaboAdaptiveDecision, TypedAdaptiveResidualMove,
};
use sensiblaw_world_expansion_runtime::MaboConsumerDiagnosis;

fn empty_baseline() -> DiscoveryIdentityBaseline {
    DiscoveryIdentityBaseline {
        identity_class_refs: BTreeSet::new(),
        representation_identity_class_refs: BTreeMap::new(),
    }
}

fn empty_world() -> LatentWorldRows {
    LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 100,
        requested_max_hops: 100,
        visited_refs: vec!["Q1501525".into()],
        deepest_observed_hop: 0,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges: vec![],
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn empty_diagnosis() -> MaboConsumerDiagnosis {
    MaboConsumerDiagnosis {
        consumer_spec: sensiblaw_consumer_residual::ConsumerSpec {
            consumer_id: "consumer:mabo-context-world-identity".into(),
            surface_id: "surface:test".into(),
            requirements: vec![],
        },
        residuals: vec![],
        rows: vec![],
        reviewed_context_edges_considered: 0,
        known_identity_representations: 0,
        duplicate_target_edges: 0,
        out_of_scope_or_wrong_type_edges: 0,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn typed_move(
    residual_ref: &str,
    class: ResidualClass,
    lane: ProducerLane,
    move_ref: &str,
    reduction: u64,
) -> TypedAdaptiveResidualMove {
    TypedAdaptiveResidualMove {
        residual: ProofResidual {
            residual_ref: residual_ref.into(),
            proposition_ref: format!("prop:{residual_ref}"),
            producer_class_ref: format!("producer:{lane:?}"),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: reduction,
            dependency_refs: vec![],
            status: ResidualStatus::Open,
        },
        residual_class: class,
        producer_lane: lane,
        move_ref: move_ref.into(),
        source_ref: Some(format!("source:{move_ref}")),
        provider_operation_ref: format!("provider:{move_ref}"),
        expected_whole_frontier_reduction: reduction,
        shared_dependency_gain: reduction,
        network_requests: 1,
        operator_review_cost: 1,
        admissible: true,
        diagnosis_reference: "diagnosis:test".into(),
    }
}

#[test]
fn legal_and_provenance_residuals_share_the_same_pareto_frontier() {
    let diagnosis = empty_diagnosis();
    let baseline = empty_baseline();
    let world = empty_world();
    let expanded = BTreeSet::new();
    let additional = vec![
        typed_move(
            "residual:mabo:legal:primary-authority",
            ResidualClass::Legal,
            ProducerLane::GovernedLegal,
            "move:oalc",
            4,
        ),
        typed_move(
            "residual:mabo:provenance:source-history",
            ResidualClass::Provenance,
            ProducerLane::SourceSpecificProvenance,
            "move:provenance",
            2,
        ),
    ];

    let compiled = compile_mabo_heterogeneous_frontier(
        &diagnosis,
        &baseline,
        &world,
        &expanded,
        &additional,
        &[],
        "frontier:mabo:heterogeneous:test",
    );

    assert_eq!(compiled.frontier.residuals.len(), 2);
    let decision = select_next_mabo_heterogeneous_decision(&diagnosis, &compiled).unwrap();
    match decision {
        MaboAdaptiveDecision::TypedProducer(selection) => {
            assert_eq!(selection.residual_class, ResidualClass::Legal);
            assert_eq!(selection.producer_lane, ProducerLane::GovernedLegal);
            assert_eq!(selection.move_ref, "move:oalc");
        }
        other => panic!("expected typed legal producer selection, got {other:?}"),
    }
}

#[test]
fn durable_wrong_type_suppresses_one_move_without_satisfying_the_residual() {
    let diagnosis = empty_diagnosis();
    let baseline = empty_baseline();
    let world = empty_world();
    let expanded = BTreeSet::new();
    let additional = vec![
        typed_move(
            "residual:mabo:legal:authority",
            ResidualClass::Legal,
            ProducerLane::GovernedLegal,
            "move:oalc:wrong-type",
            5,
        ),
        typed_move(
            "residual:mabo:legal:authority",
            ResidualClass::Legal,
            ProducerLane::SourceSpecificProvenance,
            "move:official-alt",
            3,
        ),
    ];
    let negatives = vec![DurableAdaptiveNegativeAssessment {
        residual_ref: "residual:mabo:legal:authority".into(),
        move_ref: "move:oalc:wrong-type".into(),
        kind: DurableAdaptiveNegativeKind::WrongType,
        assessment_ref: "assessment:wrong-type:1".into(),
        source_revision_ref: Some("source:oalc:1".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }];

    let compiled = compile_mabo_heterogeneous_frontier(
        &diagnosis,
        &baseline,
        &world,
        &expanded,
        &additional,
        &negatives,
        "frontier:mabo:heterogeneous:negative",
    );

    let residual = compiled
        .frontier
        .residuals
        .iter()
        .find(|residual| residual.residual_ref == "residual:mabo:legal:authority")
        .unwrap();
    assert_eq!(residual.status, ResidualStatus::Open);

    let wrong = compiled
        .candidates
        .iter()
        .find(|candidate| candidate.move_.move_ref == "move:oalc:wrong-type")
        .unwrap();
    assert!(!wrong.move_.admissible);

    let decision = select_next_mabo_heterogeneous_decision(&diagnosis, &compiled).unwrap();
    match decision {
        MaboAdaptiveDecision::TypedProducer(selection) => {
            assert_eq!(selection.move_ref, "move:official-alt");
        }
        other => panic!("expected alternate typed producer, got {other:?}"),
    }
}
