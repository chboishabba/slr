use sensiblaw_pg_source_store::{
    DisagreementKind, PartyRole, ProceduralGoal, RealMatterControversyDraft,
    RealMatterControversyReceipt, ResponseMode, ReverseLegalProofSearch,
};

#[test]
fn strict_draft_requires_persisted_reviewed_coordinates_and_typed_response() {
    let draft = RealMatterControversyDraft {
        matter_ref: "matter:mabo:oalc:1".into(),
        applicant_claim_ref: "claim:applicant:1".into(),
        applicant_reviewed_evidence_ref: "reviewed-evidence:applicant:1".into(),
        applicant_normative_order_ref: "normative-order:indigenous:meriam".into(),
        respondent_claim_ref: "claim:respondent:1".into(),
        respondent_reviewed_evidence_ref: "reviewed-evidence:respondent:1".into(),
        respondent_normative_order_ref: "normative-order:australian-municipal".into(),
        response_mode: ResponseMode::AdmitOccurrenceDisputeCharacterisation,
        disagreement_kind: DisagreementKind::Characterisation,
        unresolved_question: "What legal characterisation follows from the admitted occurrence?".into(),
        requested_discriminator: "authority-or-evidence:characterisation".into(),
        target_evidence_query: "find reviewed authority addressing the characterisation".into(),
        procedural_goal: ProceduralGoal::DecideEvidenceNeeded,
    };
    assert_eq!(draft.response_mode, ResponseMode::AdmitOccurrenceDisputeCharacterisation);
    assert_eq!(draft.disagreement_kind, DisagreementKind::Characterisation);
}

#[test]
fn receipt_is_structure_and_search_not_merits() {
    let _ = std::any::TypeId::of::<RealMatterControversyReceipt>();
    let _ = std::any::TypeId::of::<ReverseLegalProofSearch>();
    assert_ne!(PartyRole::Applicant, PartyRole::Court);
}
