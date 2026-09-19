use sensiblaw_pg_source_store::{
    load_database_config, load_latent_world_rows_with_budget, materialize_reviewed_context_edges,
    review_mabo_oalc_exact_source, ContextReviewDecision, LatentWorldBudget, SourceFamily,
};

const QID: &str = "Q1501525";
const MABO_CITATION: &str = "[1992] HCA 23; (1992) 175 CLR 1";
const MABO_CASE_IDENTITY: &str = "case:[1992]-HCA-23";
const MABO_OALC_REVISION: &str =
    "oalc:high_court_of_australia:1992-hca-23:sha256:196079f061489be501989b9455449bde358fbe9b1110a0156f74a2b8362d698d";
const MABO_TEXT_DIGEST: &str =
    "sha256:196079f061489be501989b9455449bde358fbe9b1110a0156f74a2b8362d698d";
const RECEIPT_AUTHORITY: &str = "experimental_candidate_only";
const MABO_PROPOSITION: &str = "mabo:proposition:radical-title-native-title";

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_oalc_context_materialization_and_walker_receipt() {
    let config = load_database_config(None).expect("DATABASE_URL must identify live PG store");

    let oalc_edge = review_mabo_oalc_exact_source(
        QID,
        MABO_CITATION,
        MABO_CASE_IDENTITY,
        MABO_OALC_REVISION,
        MABO_TEXT_DIGEST,
        RECEIPT_AUTHORITY,
        ContextReviewDecision::Reviewed,
    )
    .expect("reviewed exact OALC source should pass review gate");

    assert_eq!(oalc_edge.source_family, SourceFamily::Oalc);
    assert_eq!(oalc_edge.source_revision_ref, MABO_OALC_REVISION);
    assert_eq!(
        oalc_edge.source_content_digest.as_deref(),
        Some(MABO_TEXT_DIGEST)
    );
    assert_eq!(oalc_edge.relation_type_ref, "context:oalc:exact-mnc");
    assert_eq!(oalc_edge.left_ref, QID);
    assert_eq!(oalc_edge.right_ref, MABO_CASE_IDENTITY);
    assert!(oalc_edge.candidate_only);
    assert!(!oalc_edge.creates_semantic_authority);
    assert!(!oalc_edge.applicability_promoted);
    assert!(!oalc_edge.claim_truth_promoted);

    let receipt = materialize_reviewed_context_edges(&config, std::slice::from_ref(&oalc_edge))
        .expect("materialize_reviewed_context_edges should succeed for OALC edge");

    assert!(receipt.candidate_only);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.applicability_promoted);
    assert!(!receipt.claim_truth_promoted);

    let budget = LatentWorldBudget {
        max_hops: 100,
        max_nodes: 50_000,
        max_edges: 100_000,
    };

    // 1. Walker seeded from Q1501525 traverses both Wikidata context and OALC context
    let qid_world = load_latent_world_rows_with_budget(&config, QID, budget)
        .expect("walker should succeed on Q1501525");
    assert_eq!(qid_world.seed_ref, QID);
    assert_eq!(qid_world.requested_max_hops, 100);
    assert!(qid_world.visited_refs.contains(&MABO_CASE_IDENTITY.to_string()));
    assert!(qid_world.frontier_exhausted);
    assert!(!qid_world.creates_semantic_authority);
    assert!(!qid_world.applicability_promoted);
    assert!(!qid_world.claim_truth_promoted);

    let oalc_traversed_edges: Vec<_> = qid_world
        .edges
        .iter()
        .filter(|e| e.relation_ref == "context:oalc:exact-mnc")
        .collect();
    assert_eq!(oalc_traversed_edges.len(), 1);
    assert_eq!(oalc_traversed_edges[0].from_ref, QID);
    assert_eq!(oalc_traversed_edges[0].to_ref, MABO_CASE_IDENTITY);
    assert!(
        oalc_traversed_edges[0]
            .provenance_refs
            .iter()
            .any(|p| p.contains(MABO_OALC_REVISION)),
        "provenance must retain exact OALC revision reference"
    );

    // 2. Walker seeded from MABO_CASE_IDENTITY traverses back to QID and its 1-hop neighbours
    let case_world = load_latent_world_rows_with_budget(&config, MABO_CASE_IDENTITY, budget)
        .expect("walker should succeed on case identity");
    assert_eq!(case_world.seed_ref, MABO_CASE_IDENTITY);
    assert!(case_world.visited_refs.contains(&QID.to_string()));
    assert!(case_world.frontier_exhausted);
    assert!(!case_world.creates_semantic_authority);
    assert!(!case_world.applicability_promoted);
    assert!(!case_world.claim_truth_promoted);

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
