use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_pg_source_store::{DiscoveryIdentityBaseline, LatentWorldEdgeRow, LatentWorldRows};
use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    select_next_mabo_adaptive_decision, MaboAdaptiveDecision,
};
use sensiblaw_world_expansion_runtime::diagnose_mabo_context_world_identity;

fn baseline() -> DiscoveryIdentityBaseline {
    DiscoveryIdentityBaseline {
        identity_class_refs: BTreeSet::from(["world-object:known".into()]),
        representation_identity_class_refs: BTreeMap::from([("Q2".into(), "world-object:known".into())]),
    }
}

fn world() -> LatentWorldRows {
    LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 2,
        requested_max_hops: 2,
        visited_refs: vec!["Q1501525".into(), "Q1".into(), "Q2".into()],
        deepest_observed_hop: 1,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges: vec![
            LatentWorldEdgeRow {
                from_ref: "Q1501525".into(),
                to_ref: "Q1".into(),
                relation_ref: "context:wikidata:participant".into(),
                provenance_refs: vec!["context:wikidata:wikidata:Q1501525:oldid:1".into()],
            },
            LatentWorldEdgeRow {
                from_ref: "Q1501525".into(),
                to_ref: "Q1".into(),
                relation_ref: "context:wikidata:judge".into(),
                provenance_refs: vec!["context:wikidata:wikidata:Q1501525:oldid:1".into()],
            },
            LatentWorldEdgeRow {
                from_ref: "Q1501525".into(),
                to_ref: "Q2".into(),
                relation_ref: "context:wikidata:jurisdiction".into(),
                provenance_refs: vec!["context:wikidata:wikidata:Q1501525:oldid:1".into()],
            },
        ],
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn decision_is_ranked_before_review_availability_is_considered() {
    let baseline = baseline();
    let world = world();
    let diagnosis = diagnose_mabo_context_world_identity(&world, &baseline);
    let expanded = BTreeSet::from(["Q1501525".to_string()]);

    let decision = select_next_mabo_adaptive_decision(
        &diagnosis,
        &baseline,
        &world,
        &expanded,
        "frontier:adaptive:test",
    )
    .expect("one adaptive decision should exist");

    match decision {
        MaboAdaptiveDecision::Identity(selection) => {
            assert_eq!(selection.representation_ref, "Q1");
            assert_eq!(selection.shared_dependency_gain, 2);
        }
        other => panic!("expected identity decision before review lookup, got {other:?}"),
    }
}

#[test]
fn once_identity_gap_is_removed_rebuilt_world_can_choose_context_expansion() {
    let mut baseline = baseline();
    baseline
        .identity_class_refs
        .insert("world-object:q1".into());
    baseline
        .representation_identity_class_refs
        .insert("Q1".into(), "world-object:q1".into());
    let world = world();
    let diagnosis = diagnose_mabo_context_world_identity(&world, &baseline);
    assert!(diagnosis.rows.is_empty());

    let expanded = BTreeSet::from(["Q1501525".to_string(), "Q2".to_string()]);
    let decision = select_next_mabo_adaptive_decision(
        &diagnosis,
        &baseline,
        &world,
        &expanded,
        "frontier:adaptive:test:2",
    )
    .unwrap();

    match decision {
        MaboAdaptiveDecision::ContextExpansion(selection) => {
            assert_eq!(selection.source_qid, "Q1");
        }
        other => panic!("expected context expansion after re-diagnosis, got {other:?}"),
    }
}
