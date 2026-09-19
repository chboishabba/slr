use sensiblaw_pg_source_store::{
    load_database_config, load_latent_world_rows_with_budget, materialize_reviewed_context_edges,
    review_mabo_wikidata_candidate, ContextReviewDecision, LatentWorldBudget,
};

const REVISION: &str = "wikidata:Q1501525:oldid:2333409615";
const QID: &str = "Q1501525";
const MABO_PROPOSITION: &str = "mabo:proposition:radical-title-native-title";

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_wikidata_context_materialization_and_walker_receipt() {
    let config = load_database_config(None).expect("DATABASE_URL must identify live PG store");

    let direct_candidates = [
        ("P31", "Q2334719"),
        ("P1001", "Q408"),
        ("P4884", "Q1358798"),
        ("P1594", "Q4773043"),
        ("P1594", "Q3778295"),
        ("P1594", "Q267745"),
        ("P1594", "Q5226153"),
        ("P1594", "Q6261017"),
        ("P1594", "Q15527343"),
        ("P1594", "Q6832720"),
        ("P4006", "Q6851910"),
        ("P17", "Q408"),
        ("P710", "Q975866"),
        ("P710", "Q36074"),
    ];

    let mut reviewed_edges = Vec::new();
    let mut reviewed_ids = Vec::new();
    let mut unreviewed_ids = Vec::new();

    for (prop, target) in direct_candidates {
        let candidate_id = format!("wikidata:{QID}:{prop}:{target}");
        if matches!(prop, "P1001" | "P710" | "P4884" | "P1594" | "P4006") {
            let edge = review_mabo_wikidata_candidate(
                REVISION,
                &candidate_id,
                QID,
                target,
                prop,
                ContextReviewDecision::Reviewed,
            )
            .expect("reviewed candidate should pass review gate");
            reviewed_ids.push(candidate_id);
            reviewed_edges.push(edge);
        } else {
            unreviewed_ids.push(candidate_id);
        }
    }

    assert_eq!(direct_candidates.len(), 14);
    assert_eq!(reviewed_edges.len(), 12);
    assert_eq!(unreviewed_ids.len(), 2);

    let receipt = materialize_reviewed_context_edges(&config, &reviewed_edges)
        .expect("materialize_reviewed_context_edges should succeed");

    assert!(receipt.candidate_only);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.applicability_promoted);
    assert!(!receipt.claim_truth_promoted);

    let budget = LatentWorldBudget {
        max_hops: 100,
        max_nodes: 50_000,
        max_edges: 100_000,
    };

    // 1. Walker seeded from Q1501525 (the Wikidata cluster)
    let qid_world = load_latent_world_rows_with_budget(&config, QID, budget)
        .expect("walker should succeed on Q1501525");
    assert_eq!(qid_world.seed_ref, QID);
    assert_eq!(qid_world.requested_max_hops, 100);
    assert_eq!(qid_world.deepest_observed_hop, 1);
    assert!(qid_world.visited_refs.len() >= 13);
    assert!(qid_world.edges.len() >= 12);
    assert!(qid_world.frontier_exhausted);
    assert!(!qid_world.creates_semantic_authority);
    assert!(!qid_world.applicability_promoted);
    assert!(!qid_world.claim_truth_promoted);

    let qid_context_edges = qid_world
        .edges
        .iter()
        .filter(|e| e.relation_ref.starts_with("context:wikidata:"))
        .count();
    assert_eq!(qid_context_edges, 12);

    // 2. Walker seeded from Mabo Proposition (remains insulated until cross-source anchor in P7b.4-P7b.6)
    let mabo_world = load_latent_world_rows_with_budget(&config, MABO_PROPOSITION, budget)
        .expect("walker should succeed on mabo proposition");
    assert_eq!(mabo_world.seed_ref, MABO_PROPOSITION);
    assert_eq!(mabo_world.visited_refs.len(), 11);
    assert_eq!(mabo_world.edges.len(), 11);
    assert!(mabo_world.frontier_exhausted);
    assert!(!mabo_world.creates_semantic_authority);
    assert!(!mabo_world.applicability_promoted);
    assert!(!mabo_world.claim_truth_promoted);
}
