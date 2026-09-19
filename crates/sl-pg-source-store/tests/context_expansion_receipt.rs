use sensiblaw_pg_source_store::{
    reviewed_source_expansion_row, ReviewedSourceExpansionError, ReviewedSourceExpansionInput,
};

fn expansion() -> ReviewedSourceExpansionInput {
    ReviewedSourceExpansionInput {
        source_ref: "Q36074".into(),
        source_revision_ref: "wikidata:Q36074:oldid:246813579".into(),
        review_ref: "review:context:Q36074:246813579".into(),
        bounded_candidate_count: 3,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn exact_reviewed_source_expansion_is_not_identity_novelty_or_claim_payment() {
    let row = reviewed_source_expansion_row(&expansion()).unwrap();
    assert_eq!(row.source_ref, "Q36074");
    assert_eq!(row.source_revision_ref, "wikidata:Q36074:oldid:246813579");
    assert_eq!(row.bounded_candidate_count, 3);
    assert!(!row.counts_as_novel_identity);
    assert!(!row.pays_claim_residual);
    assert_eq!(row.receipt_authority, "reviewed_bounded_context_expansion_only");
}

#[test]
fn zero_bounded_candidates_can_still_close_exact_source_expansion() {
    let mut input = expansion();
    input.bounded_candidate_count = 0;
    let row = reviewed_source_expansion_row(&input).unwrap();
    assert_eq!(row.bounded_candidate_count, 0);
}

#[test]
fn source_revision_must_pin_the_same_qid_and_must_not_promote() {
    let mut mismatch = expansion();
    mismatch.source_revision_ref = "wikidata:Q408:oldid:1".into();
    assert!(matches!(
        reviewed_source_expansion_row(&mismatch),
        Err(ReviewedSourceExpansionError::SourceRevisionMismatch { .. })
    ));

    let mut promoting = expansion();
    promoting.creates_semantic_authority = true;
    assert_eq!(
        reviewed_source_expansion_row(&promoting),
        Err(ReviewedSourceExpansionError::ExpansionMayNotPromote)
    );
}
