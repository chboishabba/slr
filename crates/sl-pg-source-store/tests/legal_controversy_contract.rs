use sensiblaw_pg_source_store::{
    DisagreementKind, EpistemicStatus, LegalControversyMatter, LegalProofObligation,
    LegalPropositionFibre, LegalRole, ObligationKind, PartyRole, ProceduralGoal,
    ResponseMode, ReverseLegalProofSearch, TypedResponseEdge,
};

#[test]
fn adversarial_response_modes_are_typed_not_boolean_negation() {
    assert_ne!(ResponseMode::DenyOccurrence, ResponseMode::AdmitOccurrenceDisputeCharacterisation);
    assert_ne!(ResponseMode::DenyOccurrence, ResponseMode::DisputeCausation);
    assert_ne!(
        ResponseMode::AdmitOccurrenceDisputeCharacterisation,
        ResponseMode::DisputeCausation
    );
}

#[test]
fn normative_order_mismatch_is_an_explicit_controversy_residual_kind() {
    assert_eq!(
        DisagreementKind::NormativeOrderMismatch,
        DisagreementKind::NormativeOrderMismatch
    );
}

#[test]
fn admission_and_common_ground_do_not_require_truth_or_applicability_promotion() {
    let fibre = LegalPropositionFibre {
        fibre_ref: "legal-fibre:1".into(),
        claim_ref: "claim:1".into(),
        party: PartyRole::Respondent,
        legal_role: LegalRole::EvidentiaryFact,
        epistemic_status: EpistemicStatus::Admitted,
        evidence_kind_ref: "source_text".into(),
        source_reference: "statement:1".into(),
        reviewed_evidence_ref: Some("reviewed-evidence:1".into()),
        normative_order_ref: "normative-order:australian-municipal".into(),
        temporal_reference: "temporal:unknown".into(),
        relation_reference: "relation:1".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    assert!(fibre.validate().is_ok());
    assert!(!fibre.claim_truth_promoted);
    assert!(!fibre.applicability_promoted);
}

#[test]
fn reverse_search_is_obligation_and_residual_only() {
    let value = ReverseLegalProofSearch {
        reverse_ref: "reverse:1".into(),
        controversy_ref: "controversy:1".into(),
        goal: ProceduralGoal::PrepareForAdjudication,
        open_obligation_refs: vec!["obligation:1".into()],
        candidate_residual_refs: vec!["residual:1".into()],
        requested_discriminator: "discriminator:1".into(),
        target_evidence_query: "find source addressing continuity".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    assert!(value.validate().is_ok());
    let _ = std::any::TypeId::of::<LegalControversyMatter>();
    let _ = std::any::TypeId::of::<TypedResponseEdge>();
    let _ = std::any::TypeId::of::<LegalProofObligation>();
    assert_eq!(ObligationKind::Evidence, ObligationKind::Evidence);
}
