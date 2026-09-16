use sensiblaw_pg_source_store::{
    validate_reviewed_pnf_revision, ReviewedPnfRevision, ReviewedPnfValidationError,
};

fn reviewed() -> ReviewedPnfRevision {
    ReviewedPnfRevision {
        review_receipt_ref: "review:mabo:radical-title:v1".into(),
        document_ref: "document:mabo:1992:hca:23:brennan:wikisource-page-39".into(),
        exact_span_ref: "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership".into(),
        graph_ref: "pnf-graph:mabo:radical-title:v1".into(),
        graph_type_ref: "generic.factor_graph".into(),
        schema_version_ref: "v0_1".into(),
        graph_closure_state_ref: "reviewed_candidate".into(),
        factor_ref: "pnf-factor:mabo:radical-title:predicate:v1".into(),
        factor_revision_ref: "pnf-factor-revision:mabo:radical-title:predicate:v1".into(),
        factor_type_ref: "predicate".into(),
        factor_closure_state_ref: "reviewed_candidate".into(),
        graph_role_ref: "support_candidate".into(),
    }
}

#[test]
fn reviewed_pnf_requires_explicit_review_receipt_and_exact_span() {
    let mut missing_review = reviewed();
    missing_review.review_receipt_ref.clear();
    assert_eq!(
        validate_reviewed_pnf_revision(&missing_review),
        Err(ReviewedPnfValidationError::EmptyCoordinate("review_receipt_ref"))
    );

    let mut missing_span = reviewed();
    missing_span.exact_span_ref.clear();
    assert_eq!(
        validate_reviewed_pnf_revision(&missing_span),
        Err(ReviewedPnfValidationError::EmptyCoordinate("exact_span_ref"))
    );
}

#[test]
fn reviewed_pnf_receipt_is_still_not_proposition_payment() {
    let reviewed = reviewed();
    validate_reviewed_pnf_revision(&reviewed).unwrap();
    assert!(!reviewed.proposition_support_paid());
    assert!(!reviewed.applicability_paid());
    assert!(!reviewed.claim_truth_paid());
}
