use sensiblaw_pg_source_store::{
    derive_current_residual_refs, residual_lane_ref, DisagreementKind,
    LegalControversyResidual, LegalControversyResidualHistoryError,
    ResidualSupersessionReceipt,
};

fn residual(reference: &str, question: &str) -> LegalControversyResidual {
    LegalControversyResidual {
        residual_ref: reference.into(),
        controversy_ref: "controversy:mabo".into(),
        kind: DisagreementKind::Characterisation,
        applicant_fibre_ref: "fibre:applicant".into(),
        respondent_fibre_ref: "fibre:respondent".into(),
        residual_level_ref: "residual:open".into(),
        unresolved_question: question.into(),
        requested_discriminator: Some("discriminator:mabo".into()),
        target_evidence_query: Some("query:mabo".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn supersession(
    successor: &LegalControversyResidual,
    predecessors: &[&str],
) -> ResidualSupersessionReceipt {
    ResidualSupersessionReceipt {
        supersession_ref: "residual-supersession:test".into(),
        controversy_ref: "controversy:mabo".into(),
        lane_ref: residual_lane_ref(successor),
        successor_residual_ref: successor.residual_ref.clone(),
        predecessor_residual_refs: predecessors.iter().map(|value| (*value).into()).collect(),
        review_command_ref: "review-command:mabo-residual-refinement".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn one_unversioned_residual_is_the_current_head() {
    let refs = derive_current_residual_refs(&[residual("residual:a", "old")], &[])
        .expect("single legacy residual is unambiguous");
    assert_eq!(refs, vec!["residual:a"]);
}

#[test]
fn two_same_lane_residuals_without_reviewed_supersession_fail_closed() {
    let error = derive_current_residual_refs(
        &[
            residual("residual:a", "older narrative"),
            residual("residual:b", "reviewed refinement"),
        ],
        &[],
    )
    .unwrap_err();
    assert_eq!(error, LegalControversyResidualHistoryError::AmbiguousResidualHead);
}

#[test]
fn reviewed_supersession_preserves_history_and_selects_only_the_successor() {
    let old = residual("residual:a", "older narrative");
    let new = residual("residual:b", "reviewed refinement");
    let refs = derive_current_residual_refs(
        &[old, new.clone()],
        &[supersession(&new, &["residual:a"])],
    )
    .expect("reviewed refinement makes the current head unique");
    assert_eq!(refs, vec!["residual:b"]);
}

#[test]
fn one_reviewed_successor_can_reconcile_multiple_legacy_heads_without_rewriting_them() {
    let first = residual("residual:a", "historical A");
    let second = residual("residual:b", "historical B");
    let current = residual("residual:c", "reviewed reconciled narrative");
    let refs = derive_current_residual_refs(
        &[first, second, current.clone()],
        &[supersession(&current, &["residual:a", "residual:b"])],
    )
    .expect("all legacy heads are retained as history beneath the reviewed successor");
    assert_eq!(refs, vec!["residual:c"]);
}

#[test]
fn a_supersession_receipt_cannot_cross_residual_lanes() {
    let old = residual("residual:a", "characterisation");
    let mut other_lane = residual("residual:other", "different kind");
    other_lane.kind = DisagreementKind::Evidence;
    let error = derive_current_residual_refs(
        &[old, other_lane.clone()],
        &[supersession(&other_lane, &["residual:a"])],
    )
    .unwrap_err();
    assert_eq!(error, LegalControversyResidualHistoryError::ResidualLaneMismatch);
}
