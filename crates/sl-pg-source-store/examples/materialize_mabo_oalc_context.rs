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

fn main() {
    let config = load_database_config(None).expect("DATABASE_URL must identify live PG store");

    println!("=== MABO OALC P7b.5 MATERIALISATION RECEIPT ===");
    println!("oalc_revision: {}", MABO_OALC_REVISION);
    println!("citation: {}", MABO_CITATION);
    println!("source_identity: {}", MABO_CASE_IDENTITY);
    println!("text_digest: {}", MABO_TEXT_DIGEST);
    println!("receipt_authority: {}", RECEIPT_AUTHORITY);

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

    println!("materialized_count: {}", receipt.materialized_count);
    println!("candidate_only: {}", receipt.candidate_only);
    println!("creates_semantic_authority: {}", receipt.creates_semantic_authority);
    println!("applicability_promoted: {}", receipt.applicability_promoted);
    println!("claim_truth_promoted: {}", receipt.claim_truth_promoted);

    println!("\n=== PERSISTED RELATIONS RESULTING FROM OALC REVIEW ===");
    println!(
        "  {} -> {} [{}] (rel_ref={}, sha256={}, digest={:?})",
        oalc_edge.left_ref,
        oalc_edge.right_ref,
        oalc_edge.relation_type_ref,
        oalc_edge.relation_ref,
        &oalc_edge.relation_sha256[..16],
        oalc_edge.source_content_digest,
    );

    let budget = LatentWorldBudget {
        max_hops: 100,
        max_nodes: 50_000,
        max_edges: 100_000,
    };

    println!("\n=== 100-HOP WALKER RECEIPT: SEED = Q1501525 (MABO WIKIDATA + OALC) ===");
    let qid_world = load_latent_world_rows_with_budget(&config, QID, budget)
        .expect("walker should succeed on Q1501525");
    let qid_oalc_edges: Vec<_> = qid_world
        .edges
        .iter()
        .filter(|e| e.relation_ref == "context:oalc:exact-mnc")
        .collect();

    println!("seed: {}", qid_world.seed_ref);
    println!("requested_max_hops: {}", qid_world.requested_max_hops);
    println!("deepest_observed_hop: {}", qid_world.deepest_observed_hop);
    println!("visited_nodes: {}", qid_world.visited_refs.len());
    println!("edges: {}", qid_world.edges.len());
    println!("oalc_exact_mnc_edges: {}", qid_oalc_edges.len());
    for e in &qid_oalc_edges {
        println!("  from: {}, to: {}, provenance: {:?}", e.from_ref, e.to_ref, e.provenance_refs);
    }
    println!("frontier_exhausted: {}", qid_world.frontier_exhausted);
    println!("creates_semantic_authority: {}", qid_world.creates_semantic_authority);
    println!("applicability_promoted: {}", qid_world.applicability_promoted);
    println!("claim_truth_promoted: {}", qid_world.claim_truth_promoted);

    println!("\n=== 100-HOP WALKER RECEIPT: SEED = MABO PROPOSITION ===");
    let mabo_world = load_latent_world_rows_with_budget(&config, MABO_PROPOSITION, budget)
        .expect("walker should succeed on mabo proposition");
    println!("seed: {}", mabo_world.seed_ref);
    println!("visited_nodes: {}", mabo_world.visited_refs.len());
    println!("edges: {}", mabo_world.edges.len());
    println!("creates_semantic_authority: {}", mabo_world.creates_semantic_authority);
    println!("applicability_promoted: {}", mabo_world.applicability_promoted);
    println!("claim_truth_promoted: {}", mabo_world.claim_truth_promoted);
}
