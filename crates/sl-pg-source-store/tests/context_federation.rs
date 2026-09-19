use sensiblaw_pg_source_store::{
    materialize_reviewed_context_edges, review_mabo_oalc_exact_source,
    review_mabo_wikidata_candidate, review_mabo_wikipedia_exact_source, reviewed_context_edge,
    ContextReviewDecision, SourceFamily,
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
fn mabo_wikidata_candidate_requires_explicit_review_before_edge_construction() {
    let rejected = review_mabo_wikidata_candidate(
        "wikidata:Q1501525:oldid:2333409615",
        "wikidata:Q1501525:P4006:Q123",
        "Q1501525",
        "Q123",
        "P4006",
        ContextReviewDecision::NotReviewed,
    );
    assert!(rejected.is_err());

    let reviewed = review_mabo_wikidata_candidate(
        "wikidata:Q1501525:oldid:2333409615",
        "wikidata:Q1501525:P4006:Q123",
        "Q1501525",
        "Q123",
        "P4006",
        ContextReviewDecision::Reviewed,
    )
    .expect("explicitly reviewed P4006 candidate should become context only");

    assert_eq!(reviewed.relation_type_ref, "context:wikidata:overrules");
    assert_eq!(reviewed.left_ref, "Q1501525");
    assert_eq!(reviewed.right_ref, "Q123");
    assert!(reviewed.candidate_only);
    assert!(!reviewed.creates_semantic_authority);
    assert!(!reviewed.applicability_promoted);
    assert!(!reviewed.claim_truth_promoted);
}

#[test]
fn mabo_wikidata_review_gate_accepts_only_the_bounded_direct_property_surface() {
    for (property, relation) in [
        ("P1001", "context:wikidata:jurisdiction"),
        ("P710", "context:wikidata:participant"),
        ("P4884", "context:wikidata:court"),
        ("P1594", "context:wikidata:judge"),
        ("P4006", "context:wikidata:overrules"),
    ] {
        let edge = review_mabo_wikidata_candidate(
            "wikidata:Q1501525:oldid:2333409615",
            format!("wikidata:Q1501525:{property}:Q42"),
            "Q1501525",
            "Q42",
            property,
            ContextReviewDecision::Reviewed,
        )
        .expect("bounded reviewed property should be admitted");
        assert_eq!(edge.relation_type_ref, relation);
    }

    assert!(review_mabo_wikidata_candidate(
        "wikidata:Q1501525:oldid:2333409615",
        "wikidata:Q1501525:P31:Q2334719",
        "Q1501525",
        "Q2334719",
        "P31",
        ContextReviewDecision::Reviewed,
    )
    .is_err());

    assert!(review_mabo_wikidata_candidate(
        "wikidata:Q1501525:oldid:2333409615",
        "wikidata:Q999:P710:Q42",
        "Q999",
        "Q42",
        "P710",
        ContextReviewDecision::Reviewed,
    )
    .is_err());
}

#[test]
fn materializer_symbol_is_part_of_the_public_storage_surface() {
    let _ = materialize_reviewed_context_edges;
}


#[test]
fn oalc_exact_source_requires_explicit_review_and_retains_revision_digest_without_promotion() {
    let rejected = review_mabo_oalc_exact_source(
        "Q1501525",
        "[1992] HCA 23",
        "case:[1992]-HCA-23",
        "oalc:[1992]-HCA-23:sha256:abc",
        "sha256:abc",
        "experimental_candidate_only",
        ContextReviewDecision::NotReviewed,
    );
    assert!(rejected.is_err());

    let reviewed = review_mabo_oalc_exact_source(
        "Q1501525",
        "[1992] HCA 23",
        "case:[1992]-HCA-23",
        "oalc:[1992]-HCA-23:sha256:abc",
        "sha256:abc",
        "experimental_candidate_only",
        ContextReviewDecision::Reviewed,
    )
    .expect("reviewed exact OALC source should become candidate context");

    assert_eq!(reviewed.source_family, SourceFamily::Oalc);
    assert_eq!(reviewed.source_revision_ref, "oalc:[1992]-HCA-23:sha256:abc");
    assert_eq!(reviewed.source_content_digest.as_deref(), Some("sha256:abc"));
    assert_eq!(reviewed.relation_type_ref, "context:oalc:exact-mnc");
    assert_eq!(reviewed.left_ref, "Q1501525");
    assert_eq!(reviewed.right_ref, "case:[1992]-HCA-23");
    assert!(reviewed.candidate_only);
    assert!(!reviewed.creates_semantic_authority);
    assert!(!reviewed.applicability_promoted);
    assert!(!reviewed.claim_truth_promoted);
}

#[test]
fn oalc_exact_source_rejects_mutable_revision_alias_and_non_candidate_authority() {
    for revision in ["latest", "current", "head"] {
        assert!(review_mabo_oalc_exact_source(
            "Q1501525",
            "[1992] HCA 23",
            "case:[1992]-HCA-23",
            revision,
            "sha256:abc",
            "experimental_candidate_only",
            ContextReviewDecision::Reviewed,
        )
        .is_err());
    }

    assert!(review_mabo_oalc_exact_source(
        "Q1501525",
        "[1992] HCA 23",
        "case:[1992]-HCA-23",
        "oalc:[1992]-HCA-23:sha256:abc",
        "sha256:abc",
        "legal_authority",
        ContextReviewDecision::Reviewed,
    )
    .is_err());
}

#[test]
fn wikipedia_exact_source_requires_explicit_review_and_retains_revision_digest_without_promotion() {
    let rejected = review_mabo_wikipedia_exact_source(
        "Q1501525",
        "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)",
        "wiki:en:Mabo_v_Queensland_(No_2)",
        "wikipedia:en:oldid:1240464670",
        "sha256:fedcba9876543210",
        "experimental_candidate_only",
        ContextReviewDecision::NotReviewed,
    );
    assert!(rejected.is_err());

    let reviewed = review_mabo_wikipedia_exact_source(
        "Q1501525",
        "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)",
        "wiki:en:Mabo_v_Queensland_(No_2)",
        "wikipedia:en:oldid:1240464670",
        "sha256:fedcba9876543210",
        "experimental_candidate_only",
        ContextReviewDecision::Reviewed,
    )
    .expect("reviewed exact Wikipedia source should become candidate context");

    assert_eq!(reviewed.source_family, SourceFamily::Wikipedia);
    assert_eq!(
        reviewed.source_revision_ref,
        "wikipedia:en:oldid:1240464670"
    );
    assert_eq!(
        reviewed.source_content_digest.as_deref(),
        Some("sha256:fedcba9876543210")
    );
    assert_eq!(reviewed.relation_type_ref, "context:wikipedia:article");
    assert_eq!(reviewed.left_ref, "Q1501525");
    assert_eq!(reviewed.right_ref, "wiki:en:Mabo_v_Queensland_(No_2)");
    assert!(reviewed.candidate_only);
    assert!(!reviewed.creates_semantic_authority);
    assert!(!reviewed.applicability_promoted);
    assert!(!reviewed.claim_truth_promoted);
}

#[test]
fn wikipedia_exact_source_rejects_mutable_revision_alias_and_non_candidate_authority() {
    for revision in ["latest", "current", "head", "main", "master"] {
        assert!(review_mabo_wikipedia_exact_source(
            "Q1501525",
            "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)",
            "wiki:en:Mabo_v_Queensland_(No_2)",
            revision,
            "sha256:fedcba9876543210",
            "experimental_candidate_only",
            ContextReviewDecision::Reviewed,
        )
        .is_err());
    }

    assert!(review_mabo_wikipedia_exact_source(
        "Q1501525",
        "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)",
        "wiki:en:Mabo_v_Queensland_(No_2)",
        "wikipedia:en:oldid:1240464670",
        "sha256:fedcba9876543210",
        "background_context_authority",
        ContextReviewDecision::Reviewed,
    )
    .is_err());
}

