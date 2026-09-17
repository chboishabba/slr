use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_pg_source_store::{DiscoveryIdentityBaseline, LatentWorldEdgeRow, LatentWorldRows};
use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    diagnose_mabo_context_expansion_frontier, select_next_mabo_context_expansion,
};

fn baseline() -> DiscoveryIdentityBaseline {
    DiscoveryIdentityBaseline {
        identity_class_refs: BTreeSet::from([
            "world-object:australia".into(),
            "world-object:queensland".into(),
            "world-object:non-qid".into(),
            "world-object:unrelated-global-qid".into(),
        ]),
        representation_identity_class_refs: BTreeMap::from([
            ("Q408".into(), "world-object:australia".into()),
            ("Q36074".into(), "world-object:queensland".into()),
            ("case:[1992]-HCA-23".into(), "world-object:non-qid".into()),
            ("Q999999".into(), "world-object:unrelated-global-qid".into()),
        ]),
    }
}

fn world() -> LatentWorldRows {
    LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 3,
        requested_max_hops: 3,
        visited_refs: vec!["Q1501525".into(), "Q408".into(), "Q36074".into()],
        deepest_observed_hop: 1,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges: vec![
            LatentWorldEdgeRow {
                from_ref: "Q1501525".into(),
                to_ref: "Q36074".into(),
                relation_ref: "context:wikidata:jurisdiction".into(),
                provenance_refs: vec!["context:wikidata:wikidata:Q1501525:oldid:1".into()],
            },
            LatentWorldEdgeRow {
                from_ref: "Q1501525".into(),
                to_ref: "Q36074".into(),
                relation_ref: "context:wikidata:participant".into(),
                provenance_refs: vec!["context:wikidata:wikidata:Q1501525:oldid:1".into()],
            },
            LatentWorldEdgeRow {
                from_ref: "Q1501525".into(),
                to_ref: "Q408".into(),
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
fn only_current_world_durable_unexpanded_qids_become_context_expansion_residuals() {
    let expanded = BTreeSet::from(["Q408".to_string(), "Q1501525".to_string()]);
    let frontier = diagnose_mabo_context_expansion_frontier(
        &baseline(),
        &world(),
        &expanded,
        "frontier:context-expansion:test",
    );

    let residuals = frontier
        .open_residuals()
        .map(|residual| residual.residual_ref.as_str())
        .collect::<Vec<_>>();
    assert_eq!(residuals, vec!["residual:mabo:context-expansion:Q36074"]);
    assert_eq!(frontier.residuals[0].dependency_refs.len(), 2);
    assert!(frontier
        .residuals
        .iter()
        .all(|residual| !residual.residual_ref.ends_with("Q999999")));
}

#[test]
fn canonical_pareto_selection_uses_current_expansion_frontier_not_identity_queue() {
    let expanded = BTreeSet::from(["Q1501525".to_string()]);
    let frontier = diagnose_mabo_context_expansion_frontier(
        &baseline(),
        &world(),
        &expanded,
        "frontier:context-expansion:test",
    );

    let selected = select_next_mabo_context_expansion(&frontier).unwrap();
    assert_eq!(selected.source_qid, "Q36074");
    assert_eq!(selected.shared_dependency_gain, 2);

    let expanded_again = BTreeSet::from(["Q1501525".to_string(), "Q36074".to_string()]);
    let rebuilt = diagnose_mabo_context_expansion_frontier(
        &baseline(),
        &world(),
        &expanded_again,
        "frontier:context-expansion:test:2",
    );
    let next = select_next_mabo_context_expansion(&rebuilt).unwrap();
    assert_eq!(next.source_qid, "Q408");
}
