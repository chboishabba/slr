use sensiblaw_pg_source_store::{
    gwb_ambiguity_state_row, GwbAmbiguityStateError, GwbAmbiguityStateInput,
};

fn input(kind: &str) -> GwbAmbiguityStateInput {
    GwbAmbiguityStateInput {
        campaign_ref: "campaign:gwb-ambiguity-directed-v1".into(),
        residual_ref: "residual:gwb:Q207:type".into(),
        subject_ref: "Q207".into(),
        proposition_ref: "gwb:ambiguity:Q207:type".into(),
        kind_ref: kind.into(),
        root_qid: Some("Q207".into()),
        salience: 10,
        dependency_refs: vec!["dep:gwb:Q207".into()],
        opened_by_hop: None,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn ambiguity_state_row_is_open_candidate_only_and_non_promoting() {
    let row = gwb_ambiguity_state_row(&input("type-class")).unwrap();
    assert_eq!(row.status_ref, "open");
    assert_eq!(row.kind_ref, "type-class");
    assert!(row.candidate_only);
    assert!(!row.creates_semantic_authority);
    assert!(!row.applicability_promoted);
    assert!(!row.claim_truth_promoted);
}

#[test]
fn unknown_ambiguity_kind_fails_closed() {
    assert!(matches!(
        gwb_ambiguity_state_row(&input("invented-truth-class")),
        Err(GwbAmbiguityStateError::InvalidKind(_))
    ));
}

#[test]
fn promoting_state_fails_closed() {
    let mut bad = input("type-class");
    bad.claim_truth_promoted = true;
    assert_eq!(
        gwb_ambiguity_state_row(&bad),
        Err(GwbAmbiguityStateError::StateMayNotPromote)
    );
}
