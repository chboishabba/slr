use sensiblaw_pg_source_store::{
    review_bounded_wikidata_candidate, ContextFederationError, ContextReviewDecision,
};

#[test]
fn arbitrary_exact_qid_revision_can_be_reviewed_through_bounded_property_mapping() {
    let edge = review_bounded_wikidata_candidate(
        "wikidata:Q36074:oldid:123456",
        "wikidata:Q36074:P1001:Q408",
        "Q36074",
        "Q408",
        "P1001",
        ContextReviewDecision::Reviewed,
    )
    .expect("bounded exact QID revision should be reviewable");

    assert_eq!(edge.source_revision_ref, "wikidata:Q36074:oldid:123456");
    assert_eq!(edge.left_ref, "Q36074");
    assert_eq!(edge.right_ref, "Q408");
    assert_eq!(edge.relation_type_ref, "context:wikidata:jurisdiction");
    assert!(edge.candidate_only);
    assert!(!edge.creates_semantic_authority);
    assert!(!edge.applicability_promoted);
    assert!(!edge.claim_truth_promoted);
}

#[test]
fn source_revision_qid_must_match_candidate_source() {
    let error = review_bounded_wikidata_candidate(
        "wikidata:Q1501525:oldid:2333409615",
        "wikidata:Q36074:P1001:Q408",
        "Q36074",
        "Q408",
        "P1001",
        ContextReviewDecision::Reviewed,
    )
    .expect_err("revision/source mismatch must fail closed");

    assert!(matches!(
        error,
        ContextFederationError::WikidataCandidateMismatch(_)
    ));
}

#[test]
fn unsupported_property_remains_outside_bounded_context_review() {
    let error = review_bounded_wikidata_candidate(
        "wikidata:Q36074:oldid:123456",
        "wikidata:Q36074:P31:Q6256",
        "Q36074",
        "Q6256",
        "P31",
        ContextReviewDecision::Reviewed,
    )
    .expect_err("P31 is not part of the reviewed Mabo bounded context surface");

    assert!(matches!(
        error,
        ContextFederationError::UnsupportedMaboWikidataProperty(property) if property == "P31"
    ));
}

#[test]
fn non_reviewed_candidate_still_fails_closed() {
    let error = review_bounded_wikidata_candidate(
        "wikidata:Q36074:oldid:123456",
        "wikidata:Q36074:P1001:Q408",
        "Q36074",
        "Q408",
        "P1001",
        ContextReviewDecision::NotReviewed,
    )
    .expect_err("context relation review remains distinct and mandatory");

    assert!(matches!(error, ContextFederationError::CandidateNotReviewed(_)));
}
