use sensiblaw_pg_source_store::{
    adaptive_negative_assessment_row, AdaptiveNegativeAssessmentError,
    AdaptiveNegativeAssessmentInput,
};

fn input(kind: &str) -> AdaptiveNegativeAssessmentInput {
    AdaptiveNegativeAssessmentInput {
        residual_ref: "residual:mabo:legal:authority".into(),
        move_ref: "move:oalc:wrong-type".into(),
        kind_ref: kind.into(),
        assessment_ref: "assessment:mabo:wrong-type:1".into(),
        source_revision_ref: Some("oalc:exact:1".into()),
        candidate_only: true,
        makes_move_inadmissible: true,
        satisfies_residual: false,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn negative_assessment_is_hash_stable_and_never_satisfies_residual() {
    let row = adaptive_negative_assessment_row(&input("wrong-type")).unwrap();
    assert_eq!(row.kind_ref, "wrong-type");
    assert!(row.makes_move_inadmissible);
    assert!(!row.satisfies_residual);
    assert!(row.candidate_only);
    assert!(!row.creates_semantic_authority);
    assert!(!row.applicability_promoted);
    assert!(!row.claim_truth_promoted);
    assert_eq!(row.receipt_sha256.len(), 64);
    assert_eq!(
        row.receipt_sha256,
        adaptive_negative_assessment_row(&input("wrong-type"))
            .unwrap()
            .receipt_sha256
    );
}

#[test]
fn only_typed_negative_kinds_are_persistable() {
    assert!(adaptive_negative_assessment_row(&input("wrong-type")).is_ok());
    assert!(adaptive_negative_assessment_row(&input("duplicate")).is_ok());
    assert!(adaptive_negative_assessment_row(&input("irrelevant-to-residual")).is_ok());
    assert!(adaptive_negative_assessment_row(&input("failed-factors-through")).is_ok());
    assert!(adaptive_negative_assessment_row(&input("inadmissible")).is_ok());

    assert!(matches!(
        adaptive_negative_assessment_row(&input("truth")),
        Err(AdaptiveNegativeAssessmentError::InvalidKind(_))
    ));
}

#[test]
fn promotion_or_residual_satisfaction_fails_closed() {
    let mut bad = input("wrong-type");
    bad.satisfies_residual = true;
    assert_eq!(
        adaptive_negative_assessment_row(&bad),
        Err(AdaptiveNegativeAssessmentError::NegativeMayNotSatisfyResidual)
    );

    bad.satisfies_residual = false;
    bad.creates_semantic_authority = true;
    assert_eq!(
        adaptive_negative_assessment_row(&bad),
        Err(AdaptiveNegativeAssessmentError::NegativeMayNotPromote)
    );
}
