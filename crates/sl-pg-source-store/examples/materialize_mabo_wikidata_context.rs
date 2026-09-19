use sensiblaw_pg_source_store::{
    load_database_config, load_latent_world_rows_with_budget, materialize_reviewed_context_edges,
    review_mabo_wikidata_candidate, ContextReviewDecision, LatentWorldBudget,
};

const REVISION: &str = "wikidata:Q1501525:oldid:2333409615";
const QID: &str = "Q1501525";
const MABO_PROPOSITION: &str = "mabo:proposition:radical-title-native-title";

fn main() {
    let config = load_database_config(None).expect("DATABASE_URL must identify live PG store");

    // All 14 direct property candidates emitted by bbb155a for Q1501525 @ 2333409615
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

    println!("=== MABO WIKIDATA P7b MATERIALISATION RECEIPT ===");
    println!("wikidata_revision: {}", REVISION);
    println!("wikidata_candidates_observed: {}", direct_candidates.len());

    let mut reviewed_edges = Vec::new();
    let mut reviewed_candidate_ids = Vec::new();
    let mut unreviewed_candidate_ids = Vec::new();

    for (prop, target) in direct_candidates {
        let candidate_id = format!("wikidata:{QID}:{prop}:{target}");
        // Bounded direct review gate: only {P1001, P710, P4884, P1594, P4006}
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
            reviewed_candidate_ids.push(candidate_id);
            reviewed_edges.push(edge);
        } else {
            unreviewed_candidate_ids.push(candidate_id);
        }
    }

    println!("wikidata_candidates_reviewed: {}", reviewed_candidate_ids.len());
    println!("wikidata_candidates_unreviewed_or_rejected: {}", unreviewed_candidate_ids.len());
    for id in &reviewed_candidate_ids {
        println!("  reviewed: {id}");
    }
    for id in &unreviewed_candidate_ids {
        println!("  unreviewed: {id}");
    }

    let receipt = materialize_reviewed_context_edges(&config, &reviewed_edges)
        .expect("materialize_reviewed_context_edges should succeed");

    println!("materialized_count: {}", receipt.materialized_count);
    println!("candidate_only: {}", receipt.candidate_only);
    println!("creates_semantic_authority: {}", receipt.creates_semantic_authority);
    println!("applicability_promoted: {}", receipt.applicability_promoted);
    println!("claim_truth_promoted: {}", receipt.claim_truth_promoted);

    println!("\n=== PERSISTED RELATIONS RESULTING FROM REVIEW ===");
    for edge in &reviewed_edges {
        println!(
            "  {} -> {} [{}] (rel_ref={}, sha256={})",
            edge.left_ref,
            edge.right_ref,
            edge.relation_type_ref,
            edge.relation_ref,
            &edge.relation_sha256[..16],
        );
    }

    let budget = LatentWorldBudget {
        max_hops: 100,
        max_nodes: 50_000,
        max_edges: 100_000,
    };

    println!("\n=== 100-HOP WALKER RECEIPT: SEED = MABO PROPOSITION ===");
    let mabo_world = load_latent_world_rows_with_budget(&config, MABO_PROPOSITION, budget)
        .expect("walker should succeed on mabo proposition");
    let mabo_context_edges = mabo_world.edges.iter().filter(|e| e.relation_ref.starts_with("context:")).count();
    let mabo_legal_ir_edges = mabo_world.edges.iter().filter(|e| e.relation_ref.starts_with("legal_ir:")).count();
    let mabo_wikidata_edges = mabo_world.edges.iter().filter(|e| e.relation_ref.starts_with("context:wikidata:")).count();

    println!("seed: {}", mabo_world.seed_ref);
    println!("requested_max_hops: {}", mabo_world.requested_max_hops);
    println!("deepest_observed_hop: {}", mabo_world.deepest_observed_hop);
    println!("visited_nodes: {}", mabo_world.visited_refs.len());
    println!("edges: {}", mabo_world.edges.len());
    println!("context_only_edges: {}", mabo_context_edges);
    println!("legal_ir_proof_edges: {}", mabo_legal_ir_edges);
    println!("wikidata_provenanced_edges: {}", mabo_wikidata_edges);
    println!("frontier_exhausted: {}", mabo_world.frontier_exhausted);
    println!("residual_refs: {:?}", mabo_world.residual_refs);
    println!("creates_semantic_authority: {}", mabo_world.creates_semantic_authority);
    println!("applicability_promoted: {}", mabo_world.applicability_promoted);
    println!("claim_truth_promoted: {}", mabo_world.claim_truth_promoted);

    println!("\n=== 100-HOP WALKER RECEIPT: SEED = Q1501525 (MABO WIKIDATA) ===");
    let qid_world = load_latent_world_rows_with_budget(&config, QID, budget)
        .expect("walker should succeed on Q1501525");
    let qid_context_edges = qid_world.edges.iter().filter(|e| e.relation_ref.starts_with("context:")).count();
    let qid_legal_ir_edges = qid_world.edges.iter().filter(|e| e.relation_ref.starts_with("legal_ir:")).count();
    let qid_wikidata_edges = qid_world.edges.iter().filter(|e| e.relation_ref.starts_with("context:wikidata:")).count();

    println!("seed: {}", qid_world.seed_ref);
    println!("requested_max_hops: {}", qid_world.requested_max_hops);
    println!("deepest_observed_hop: {}", qid_world.deepest_observed_hop);
    println!("visited_nodes: {}", qid_world.visited_refs.len());
    println!("edges: {}", qid_world.edges.len());
    println!("context_only_edges: {}", qid_context_edges);
    println!("legal_ir_proof_edges: {}", qid_legal_ir_edges);
    println!("wikidata_provenanced_edges: {}", qid_wikidata_edges);
    println!("frontier_exhausted: {}", qid_world.frontier_exhausted);
    println!("residual_refs: {:?}", qid_world.residual_refs);
    println!("creates_semantic_authority: {}", qid_world.creates_semantic_authority);
    println!("applicability_promoted: {}", qid_world.applicability_promoted);
    println!("claim_truth_promoted: {}", qid_world.claim_truth_promoted);
    println!("visited_nodes_list: {:?}", qid_world.visited_refs);

    let frontier_broader_from_mabo = mabo_world.visited_refs.len() > 11;
    println!("\npersisted_frontier_became_broader_from_mabo_proposition: {}", frontier_broader_from_mabo);
    if !frontier_broader_from_mabo {
        println!("explanation: Q1501525 context edges are currently persisted as a reviewed Wikidata cluster; no cross-source anchor edge between the legal_ir Mabo proposition and Q1501525 has been materialized yet (scheduled for Wikipedia/OALC federation in P7b.4-P7b.6).");
    }
}
