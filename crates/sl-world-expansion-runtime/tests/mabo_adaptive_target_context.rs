use sensiblaw_pg_source_store::ContextReviewDecision;
use sensiblaw_proof_search_loop::world_expansion_runner::RecurrentRunBlockerKind;
use sensiblaw_wikimedia_candidate_provider::entity_revision_receipt_from_rdf;
use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    parse_bounded_target_context, prepare_reviewed_target_context,
};

fn acquired() -> sensiblaw_wikimedia_candidate_provider::AcquiredEntityRdf {
    let rdf = br#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:wdt="http://www.wikidata.org/prop/direct/">
  <rdf:Description rdf:about="http://www.wikidata.org/entity/Q36074">
    <wdt:P1001 rdf:resource="http://www.wikidata.org/entity/Q408"/>
    <wdt:P31 rdf:resource="http://www.wikidata.org/entity/Q6256"/>
  </rdf:Description>
</rdf:RDF>"#;
    entity_revision_receipt_from_rdf("Q36074", 246813579, rdf.to_vec()).unwrap()
}

#[test]
fn parser_retains_only_finite_bounded_outgoing_context() {
    let candidates = parse_bounded_target_context(&acquired()).unwrap();
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].candidate_id, "wikidata:Q36074:P1001:Q408");
    assert_eq!(candidates[0].source_qid, "Q36074");
    assert_eq!(candidates[0].target_qid, "Q408");
    assert_eq!(candidates[0].property_ref, "P1001");
    assert_eq!(
        candidates[0].source_revision_ref,
        "wikidata:Q36074:oldid:246813579"
    );
}

#[test]
fn identity_review_does_not_substitute_for_outgoing_context_review() {
    let candidates = parse_bounded_target_context(&acquired()).unwrap();
    let blocker = prepare_reviewed_target_context(&candidates, ContextReviewDecision::NotReviewed)
        .expect_err("outgoing context needs its own explicit review");
    assert_eq!(blocker.kind, RecurrentRunBlockerKind::ContextReviewRequired);
}

#[test]
fn explicit_context_review_builds_candidate_only_edges() {
    let candidates = parse_bounded_target_context(&acquired()).unwrap();
    let edges = prepare_reviewed_target_context(&candidates, ContextReviewDecision::Reviewed)
        .expect("explicit context review should prepare bounded context");
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].left_ref, "Q36074");
    assert_eq!(edges[0].right_ref, "Q408");
    assert_eq!(edges[0].relation_type_ref, "context:wikidata:jurisdiction");
    assert!(edges[0].candidate_only);
    assert!(!edges[0].creates_semantic_authority);
    assert!(!edges[0].applicability_promoted);
    assert!(!edges[0].claim_truth_promoted);
}
