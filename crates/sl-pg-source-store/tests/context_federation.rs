use sensiblaw_pg_source_store::{
    materialize_reviewed_context_edges, reviewed_context_edge, SourceFamily,
};

#[test]
fn reviewed_context_edges_are_revision_attributed_candidate_only_and_non_promoting() {
    let first = reviewed_context_edge(
        SourceFamily::Wikidata,
        "wikidata:Q1501525:oldid:2333409615",
        "Q1501525",
        "Q1358798",
        "context:wikidata:court",
    )
    .expect("valid reviewed Wikidata edge");
    let revised = reviewed_context_edge(
        SourceFamily::Wikidata,
        "wikidata:Q1501525:oldid:2333409616",
        "Q1501525",
        "Q1358798",
        "context:wikidata:court",
    )
    .expect("valid reviewed Wikidata edge");

    assert_ne!(first.relation_ref, revised.relation_ref);
    assert_ne!(first.relation_sha256, revised.relation_sha256);
    assert_eq!(first.source_family, SourceFamily::Wikidata);
    assert_eq!(first.source_revision_ref, "wikidata:Q1501525:oldid:2333409615");
    assert!(first.candidate_only);
    assert!(!first.creates_semantic_authority);
    assert!(!first.applicability_promoted);
    assert!(!first.claim_truth_promoted);
}

#[test]
fn federation_rejects_empty_revision_or_endpoint_coordinates() {
    assert!(reviewed_context_edge(
        SourceFamily::Wikipedia,
        "",
        "wiki:en:Mabo_v_Queensland_(No_2)",
        "Q1501525",
        "context:wikipedia:about",
    )
    .is_err());
    assert!(reviewed_context_edge(
        SourceFamily::Oalc,
        "oalc:snapshot:example",
        "",
        "source:mabo:1992:hca:23",
        "context:oalc:exact-mnc",
    )
    .is_err());
}

#[test]
fn materializer_symbol_is_part_of_the_public_storage_surface() {
    let _ = materialize_reviewed_context_edges;
}
