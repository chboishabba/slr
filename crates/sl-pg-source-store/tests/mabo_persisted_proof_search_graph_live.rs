//! Live, ignored, fail-closed Mabo graph acceptance gate.
//! Requires DATABASE_URL pointing at the user's populated canonical SLR PG.

use postgres::{Client, NoTls};
use sensiblaw_pg_source_store::{
    derive_mabo_proof_search_graph, load_bound_investigation_graph_projection,
    load_database_config, persist_mabo_proof_search_graph,
    MABO_CONTROVERSY_REF,
};

#[test]
#[ignore = "requires real populated Mabo PostgreSQL; no fixture may pay this gate"]
fn mabo_persisted_proof_search_graph_live() {
    let config = load_database_config(None).expect("existing real DATABASE_URL");
    let mut client = Client::connect(config.database_url(), NoTls).expect("real SLR PostgreSQL");
    let rows = client.query(
        "SELECT obligation_ref FROM legal_controversy.proof_obligation
         WHERE controversy_ref=$1 AND is_open AND candidate_only
           AND NOT creates_semantic_authority AND NOT creates_legal_authority
           AND NOT applicability_promoted AND NOT claim_truth_promoted
         ORDER BY obligation_ref",
        &[&MABO_CONTROVERSY_REF],
    ).expect("reopen persisted open Mabo obligations");
    assert!(!rows.is_empty(), "no persisted open Mabo proof obligation");
    for row in rows {
        let obligation_ref: String = row.get(0);
        let graph = derive_mabo_proof_search_graph(
            &config, MABO_CONTROVERSY_REF, &obligation_ref,
        ).expect("derive only from persisted legal/source/review objects");
        let first = persist_mabo_proof_search_graph(&config, &graph)
            .expect("persist exact legal-follow projection and binding");
        let second = persist_mabo_proof_search_graph(&config, &graph)
            .expect("immutable replay");
        assert_eq!(first, second);
        let reopened = load_bound_investigation_graph_projection(
            &config, &obligation_ref, &graph.matter_ref,
        ).expect("reopen exact binding").expect("no no-synthesis fallback after paid persistence");
        assert_eq!(reopened.binding.binding_ref, first.binding_ref);
        assert_eq!(reopened.projection.legal_follow_graph.projection_ref, first.projection_ref);
        assert_eq!(reopened.projection.legal_follow_graph.nodes.len() as i64, first.node_count);
        assert_eq!(reopened.projection.legal_follow_graph.edges.len() as i64, first.edge_count);
        assert!(first.derived_only && first.challengeable);
        assert!(!first.creates_semantic_authority);
        assert!(!first.creates_legal_authority);
        assert!(!first.applicability_promoted);
        assert!(!first.claim_truth_promoted);
        assert!(!first.residual_resolved);
        assert!(!first.search_executed);
        println!(
            "MABO_GRAPH controversy_ref={} matter_ref={} obligation_ref={} residual_ref={} projection_ref={} binding_ref={} nodes={} edges={} derived_only=true challengeable=true semantic_authority=false legal_authority=false applicability=false claim_truth=false residual_resolution=false search_execution=false",
            first.controversy_ref, first.matter_ref, first.legal_obligation_ref,
            first.residual_ref, first.projection_ref, first.binding_ref,
            first.node_count, first.edge_count,
        );
    }
}
