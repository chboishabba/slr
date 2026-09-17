use std::collections::BTreeMap;

use sensiblaw_pg_source_store::{DiscoveryIdentityBaseline, LatentWorldEdgeRow, LatentWorldRows};
use sensiblaw_world_expansion_runtime::{
    diagnose_mabo_context_world_identity, parse_mabo_identity_review_tsv,
    plan_mabo_identity_reviews, MaboReviewManifestError,
};

fn reviewed_edge(target: &str) -> LatentWorldEdgeRow {
    LatentWorldEdgeRow {
        from_ref: "Q1501525".into(),
        to_ref: target.into(),
        relation_ref: "context:wikidata:participant".into(),
        provenance_refs: vec![
            "context:wikidata:wikidata:Q1501525:oldid:2333409615".into(),
        ],
    }
}

fn diagnosis() -> sensiblaw_world_expansion_runtime::MaboConsumerDiagnosis {
    let world = LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 100,
        requested_max_hops: 100,
        visited_refs: vec!["Q1501525".into(), "Q975866".into(), "Q123".into()],
        deepest_observed_hop: 1,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges: vec![reviewed_edge("Q975866"), reviewed_edge("Q123")],
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    diagnose_mabo_context_world_identity(&world, &DiscoveryIdentityBaseline::default())
}

#[test]
fn manifest_ignores_comments_and_blank_lines_and_deduplicates_exact_assignments() {
    let parsed = parse_mabo_identity_review_tsv(
        "\n# representation<TAB>identity-class<TAB>review-ref\n\
         Q975866\tworld-object:eddie-mabo\treview:mabo:eddie\n\
         Q975866\tworld-object:eddie-mabo\treview:mabo:eddie\n\
         Q123\tworld-object:q123\treview:mabo:q123\n",
    )
    .expect("valid explicit review manifest");

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].representation_ref, "Q123");
    assert_eq!(parsed[1].representation_ref, "Q975866");
}

#[test]
fn manifest_requires_exactly_three_nonempty_tab_separated_fields() {
    assert_eq!(
        parse_mabo_identity_review_tsv("Q975866\tworld-object:eddie-mabo"),
        Err(MaboReviewManifestError::InvalidFieldCount {
            line_number: 1,
            field_count: 2,
        })
    );
    assert_eq!(
        parse_mabo_identity_review_tsv("Q975866\t\treview:mabo:eddie"),
        Err(MaboReviewManifestError::EmptyField {
            line_number: 1,
            field_name: "identity_class_ref",
        })
    );
}

#[test]
fn conflicting_duplicate_representation_fails_closed() {
    let error = parse_mabo_identity_review_tsv(
        "Q975866\tworld-object:eddie-mabo\treview:mabo:eddie\n\
         Q975866\tworld-object:someone-else\treview:mabo:other\n",
    )
    .expect_err("one representation cannot receive two reviewed identity assignments");

    assert_eq!(
        error,
        MaboReviewManifestError::ConflictingAssignment {
            representation_ref: "Q975866".into(),
            first_identity_class_ref: "world-object:eddie-mabo".into(),
            second_identity_class_ref: "world-object:someone-else".into(),
        }
    );
}

#[test]
fn plan_matches_only_explicit_reviews_and_keeps_pending_and_unmatched_visible() {
    let diagnosis = diagnosis();
    let reviews = parse_mabo_identity_review_tsv(
        "Q975866\tworld-object:eddie-mabo\treview:mabo:eddie\n\
         Q999999\tworld-object:outside-diagnosis\treview:mabo:outside\n",
    )
    .unwrap();

    let plan = plan_mabo_identity_reviews(&diagnosis, &reviews).unwrap();

    assert_eq!(plan.matched.len(), 1);
    assert_eq!(plan.matched[0].row.representation_ref, "Q975866");
    assert_eq!(
        plan.matched[0].assignment.identity_class_ref,
        "world-object:eddie-mabo"
    );
    assert_eq!(plan.pending_rows.len(), 1);
    assert_eq!(plan.pending_rows[0].representation_ref, "Q123");
    assert_eq!(plan.unmatched_assignments.len(), 1);
    assert_eq!(
        plan.unmatched_assignments[0].representation_ref,
        "Q999999"
    );
}

#[test]
fn no_reviews_leaves_every_diagnosed_row_pending() {
    let diagnosis = diagnosis();
    let plan = plan_mabo_identity_reviews(&diagnosis, &[]).unwrap();

    assert!(plan.matched.is_empty());
    assert_eq!(plan.pending_rows.len(), 2);
    assert!(plan.unmatched_assignments.is_empty());
}

#[test]
fn review_for_already_durable_representation_is_unmatched_not_recounted() {
    let world = LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 100,
        requested_max_hops: 100,
        visited_refs: vec!["Q1501525".into(), "Q975866".into()],
        deepest_observed_hop: 1,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges: vec![reviewed_edge("Q975866")],
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let mut baseline = DiscoveryIdentityBaseline::default();
    baseline.representation_identity_class_refs = BTreeMap::from([(
        "Q975866".into(),
        "world-object:eddie-mabo".into(),
    )]);
    baseline.identity_class_refs.insert("world-object:eddie-mabo".into());
    let diagnosis = diagnose_mabo_context_world_identity(&world, &baseline);
    let reviews = parse_mabo_identity_review_tsv(
        "Q975866\tworld-object:eddie-mabo\treview:mabo:eddie\n",
    )
    .unwrap();

    let plan = plan_mabo_identity_reviews(&diagnosis, &reviews).unwrap();

    assert!(plan.matched.is_empty());
    assert!(plan.pending_rows.is_empty());
    assert_eq!(plan.unmatched_assignments.len(), 1);
}
