use sensiblaw_pg_source_store::{
    load_database_config, load_latent_world_rows_with_budget, LatentWorldBudget,
};

const MABO_PROPOSITION: &str = "mabo:proposition:radical-title-native-title";

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_world_reports_scale_frontier_and_non_authority() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let world = load_latent_world_rows_with_budget(
        &config,
        MABO_PROPOSITION,
        LatentWorldBudget {
            max_hops: 100,
            max_nodes: 50_000,
            max_edges: 100_000,
        },
    )
    .expect("100-hop Mabo world must project from the existing PG relation surface");

    assert_eq!(world.seed_ref, MABO_PROPOSITION);
    assert_eq!(world.requested_max_hops, 100);
    assert!(world.visited_refs.iter().any(|node| node == MABO_PROPOSITION));
    assert!(world.deepest_observed_hop > 0);
    assert!(world.visited_refs.len() <= 50_000);
    assert!(world.edges.len() <= 100_000);
    assert!(world.edges.iter().all(|edge| !edge.provenance_refs.is_empty()));
    assert!(!world.creates_semantic_authority);
    assert!(!world.applicability_promoted);
    assert!(!world.claim_truth_promoted);

    if world.frontier_exhausted {
        assert!(world.frontier_refs.is_empty());
        assert!(world.residual_refs.is_empty());
    } else {
        assert!(!world.frontier_refs.is_empty());
        assert!(world
            .residual_refs
            .iter()
            .any(|residual| residual.starts_with("world-residual:")));
    }
}
