use sensiblaw_pg_source_store::{
    load_database_config, load_latent_world_rows_with_budget, materialize_reviewed_context_edges,
    review_mabo_wikipedia_exact_source, ContextReviewDecision, LatentWorldBudget, SourceFamily,
};

const QID: &str = "Q1501525";
const WIKIPEDIA_CANONICAL_URL: &str = "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)";
const WIKIPEDIA_IDENTITY: &str = "wiki:en:Mabo_v_Queensland_(No_2)";
const WIKIPEDIA_REVISION: &str = "wikipedia:en:oldid:1240464670";
const WIKIPEDIA_DIGEST: &str =
    "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069";
const RECEIPT_AUTHORITY: &str = "experimental_candidate_only";
const MABO_PROPOSITION: &str = "mabo:proposition:radical-title-native-title";

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_wikipedia_context_materialization_and_walker_receipt() {
    let config = load_database_config(None).expect("DATABASE_URL must identify live PG store");

    let wiki_edge = review_mabo_wikipedia_exact_source(
        QID,
        WIKIPEDIA_CANONICAL_URL,
        WIKIPEDIA_IDENTITY,
        WIKIPEDIA_REVISION,
        WIKIPEDIA_DIGEST,
        RECEIPT_AUTHORITY,
        ContextReviewDecision::Reviewed,
    )
    .expect("reviewed exact Wikipedia source should pass review gate");

    assert_eq!(wiki_edge.source_family, SourceFamily::Wikipedia);
    assert_eq!(wiki_edge.source_revision_ref, WIKIPEDIA_REVISION);
    assert_eq!(
        wiki_edge.source_content_digest.as_deref(),
        Some(WIKIPEDIA_DIGEST)
    );
    assert_eq!(wiki_edge.relation_type_ref, "context:wikipedia:article");
    assert_eq!(wiki_edge.left_ref, QID);
    assert_eq!(wiki_edge.right_ref, WIKIPEDIA_IDENTITY);
    assert!(wiki_edge.candidate_only);
    assert!(!wiki_edge.creates_semantic_authority);
    assert!(!wiki_edge.applicability_promoted);
    assert!(!wiki_edge.claim_truth_promoted);

    let receipt = materialize_reviewed_context_edges(&config, std::slice::from_ref(&wiki_edge))
        .expect("materialize_reviewed_context_edges should succeed for Wikipedia edge");

    assert!(receipt.candidate_only);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.applicability_promoted);
    assert!(!receipt.claim_truth_promoted);

    let budget = LatentWorldBudget {
        max_hops: 100,
        max_nodes: 50_000,
        max_edges: 100_000,
    };

    // 1. Walker seeded from Q1501525 traverses Wikidata, OALC, and Wikipedia context
    let qid_world = load_latent_world_rows_with_budget(&config, QID, budget)
        .expect("walker should succeed on Q1501525");
    assert_eq!(qid_world.seed_ref, QID);
    assert_eq!(qid_world.requested_max_hops, 100);
    assert!(qid_world.visited_refs.contains(&WIKIPEDIA_IDENTITY.to_string()));
    assert!(qid_world.frontier_exhausted);
    assert!(!qid_world.creates_semantic_authority);
    assert!(!qid_world.applicability_promoted);
    assert!(!qid_world.claim_truth_promoted);

    let wiki_traversed_edges: Vec<_> = qid_world
        .edges
        .iter()
        .filter(|e| e.relation_ref == "context:wikipedia:article")
        .collect();
    assert_eq!(wiki_traversed_edges.len(), 1);
    assert_eq!(wiki_traversed_edges[0].from_ref, QID);
    assert_eq!(wiki_traversed_edges[0].to_ref, WIKIPEDIA_IDENTITY);
    assert!(
        wiki_traversed_edges[0]
            .provenance_refs
            .iter()
            .any(|p| p.contains(WIKIPEDIA_REVISION)),
        "provenance must retain exact Wikipedia revision reference"
    );

    // 2. Walker seeded from WIKIPEDIA_IDENTITY traverses back to QID and its neighbours
    let wiki_world = load_latent_world_rows_with_budget(&config, WIKIPEDIA_IDENTITY, budget)
        .expect("walker should succeed on Wikipedia identity");
    assert_eq!(wiki_world.seed_ref, WIKIPEDIA_IDENTITY);
    assert!(wiki_world.visited_refs.contains(&QID.to_string()));
    assert!(wiki_world.frontier_exhausted);
    assert!(!wiki_world.creates_semantic_authority);
    assert!(!wiki_world.applicability_promoted);
    assert!(!wiki_world.claim_truth_promoted);

    // 3. Walker seeded from Mabo Proposition remains insulated until cross-source anchor
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
