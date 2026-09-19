use sensiblaw_pg_source_store::{
    identity_alias_row, NonNovelIdentityAliasError, NonNovelIdentityAliasInput,
};

fn alias() -> NonNovelIdentityAliasInput {
    NonNovelIdentityAliasInput {
        representation_ref: "Q975866".into(),
        identity_class_ref: "world-object:eddie-mabo".into(),
        review_ref: "review:mabo:eddie-mabo-qid".into(),
        source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn alias_receipt_persists_representation_identity_without_novel_discovery_semantics() {
    let row = identity_alias_row(&alias()).expect("reviewed non-novel alias should be representable");
    assert_eq!(row.representation_ref, "Q975866");
    assert_eq!(row.identity_class_ref, "world-object:eddie-mabo");
    assert_eq!(row.receipt_authority, "reviewed_non_novel_identity_alias_only");
    assert!(!row.counts_as_novel_discovery);
    assert!(!row.creates_discovery_lineage);
    assert!(row.candidate_only);
    assert!(!row.creates_semantic_authority);
    assert!(!row.applicability_promoted);
    assert!(!row.claim_truth_promoted);
}

#[test]
fn alias_receipt_fails_closed_on_promotion_or_empty_coordinates() {
    let mut promoting = alias();
    promoting.claim_truth_promoted = true;
    assert_eq!(
        identity_alias_row(&promoting),
        Err(NonNovelIdentityAliasError::AliasMayNotPromote)
    );

    let mut empty = alias();
    empty.representation_ref.clear();
    assert_eq!(
        identity_alias_row(&empty),
        Err(NonNovelIdentityAliasError::EmptyCoordinate("representation_ref"))
    );
}

#[test]
fn alias_receipt_is_hash_stable_and_review_sensitive() {
    let first = identity_alias_row(&alias()).unwrap();
    let second = identity_alias_row(&alias()).unwrap();
    assert_eq!(first.receipt_sha256, second.receipt_sha256);

    let mut changed = alias();
    changed.review_ref = "review:mabo:eddie-mabo-qid:v2".into();
    assert_ne!(first.receipt_sha256, identity_alias_row(&changed).unwrap().receipt_sha256);
}
