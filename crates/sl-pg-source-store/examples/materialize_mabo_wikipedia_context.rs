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

fn main() {
    let config = load_database_config(None).expect("DATABASE_URL must identify live PG store");

    println!("=== MABO WIKIPEDIA P7b.4 MATERIALISATION RECEIPT ===");
    println!("wikipedia_revision: {}", WIKIPEDIA_REVISION);
    println!("canonical_url: {}", WIKIPEDIA_CANONICAL_URL);
    println!("source_identity: {}", WIKIPEDIA_IDENTITY);
    println!("text_digest: {}", WIKIPEDIA_DIGEST);
    println!("receipt_authority: {}", RECEIPT_AUTHORITY);

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

    println!("materialized_count: {}", receipt.materialized_count);
    println!("candidate_only: {}", receipt.candidate_only);
    println!("creates_semantic_authority: {}", receipt.creates_semantic_authority);
    println!("applicability_promoted: {}", receipt.applicability_promoted);
    println!("claim_truth_promoted: {}", receipt.claim_truth_promoted);

    println!("\n=== PERSISTED RELATIONS RESULTING FROM WIKIPEDIA REVIEW ===");
    println!(
        "  {} -> {} [{}] (rel_ref={}, sha256={}, digest={:?})",
        wiki_edge.left_ref,
        wiki_edge.right_ref,
        wiki_edge.relation_type_ref,
        wiki_edge.relation_ref,
        &wiki_edge.relation_sha256[..16],
        wiki_edge.source_content_digest,
    );

    let budget = LatentWorldBudget {
        max_hops: 100,
        max_nodes: 50_000,
        max_edges: 100_000,
    };

    println!("\n=== 100-HOP WALKER RECEIPT: SEED = Q1501525 (MABO WIKIDATA + OALC + WIKIPEDIA) ===");
    let qid_world = load_latent_world_rows_with_budget(&config, QID, budget)
        .expect("walker should succeed on Q1501525");
    let qid_wiki_edges: Vec<_> = qid_world
        .edges
        .iter()
        .filter(|e| e.relation_ref == "context:wikipedia:article")
        .collect();

    println!("seed: {}", qid_world.seed_ref);
    println!("requested_max_hops: {}", qid_world.requested_max_hops);
    println!("deepest_observed_hop: {}", qid_world.deepest_observed_hop);
    println!("visited_nodes: {}", qid_world.visited_refs.len());
    println!("edges: {}", qid_world.edges.len());
    println!("wikipedia_article_edges: {}", qid_wiki_edges.len());
    for e in &qid_wiki_edges {
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
