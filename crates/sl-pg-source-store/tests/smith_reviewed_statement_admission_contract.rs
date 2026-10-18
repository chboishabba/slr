use sensiblaw_pg_source_store::{
    smith_reviewed_statement_link_from_candidate, SmithReviewedAdmissionCandidate,
    SmithReviewedStatementAdmissionError, SMITH_APPLICANT_STATEMENT_REF,
    SMITH_RESPONDENT_STATEMENT_REF,
};

fn candidate(statement_ref: &str) -> SmithReviewedAdmissionCandidate {
    SmithReviewedAdmissionCandidate {
        reviewed_evidence_ref: format!("reviewed-evidence:{statement_ref}"),
        statement_ref: statement_ref.into(),
        source_revision_ref: "external-source-revision:smith:hca37".into(),
        candidate_pnf_batch_ref: format!("batch:{statement_ref}"),
        observation_ref: format!("observation:{statement_ref}"),
        review_receipt_ref: format!("review:{statement_ref}"),
        review_semantic_ref: format!("observation:{statement_ref}"),
        review_action_ref: "accept".into(),
        review_effect_ref: "status_changed".into(),
        review_effect_value_ref: Some("accepted".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn exact_applicant_and_respondent_reviewed_coordinates_can_materialise_links() {
    for statement_ref in [SMITH_APPLICANT_STATEMENT_REF, SMITH_RESPONDENT_STATEMENT_REF] {
        let value = candidate(statement_ref);
        let link = smith_reviewed_statement_link_from_candidate(&value).unwrap();
        assert_eq!(link.statement_ref, statement_ref);
        assert_eq!(link.candidate_pnf_ref, value.candidate_pnf_batch_ref);
        assert_eq!(link.observation_ref, value.observation_ref);
        assert_eq!(link.admission_receipt_ref.as_deref(), Some(value.review_receipt_ref.as_str()));
        assert!(link.candidate_only);
        assert!(!link.creates_semantic_authority);
        assert!(!link.applicability_promoted);
        assert!(!link.claim_truth_promoted);
    }
}

#[test]
fn unrelated_statement_is_wrong_type() {
    let value = candidate("statement:sha256:not-smith");
    assert!(matches!(
        smith_reviewed_statement_link_from_candidate(&value),
        Err(SmithReviewedStatementAdmissionError::WrongSmithStatement(_))
    ));
}

#[test]
fn qualified_or_nonaccept_review_cannot_become_semantic_admission() {
    let mut value = candidate(SMITH_APPLICANT_STATEMENT_REF);
    value.review_action_ref = "qualify".into();
    value.review_effect_value_ref = Some("qualified".into());
    assert_eq!(
        smith_reviewed_statement_link_from_candidate(&value),
        Err(SmithReviewedStatementAdmissionError::ReviewNotAccepted)
    );
}

#[test]
fn review_must_be_of_the_exact_reopened_observation() {
    let mut value = candidate(SMITH_RESPONDENT_STATEMENT_REF);
    value.review_semantic_ref = "observation:other".into();
    assert_eq!(
        smith_reviewed_statement_link_from_candidate(&value),
        Err(SmithReviewedStatementAdmissionError::ReviewObservationMismatch)
    );
}
