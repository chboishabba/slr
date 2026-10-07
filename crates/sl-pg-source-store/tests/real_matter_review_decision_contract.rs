use sensiblaw_pg_source_store::{
    DecisionBackedReviewedLegalEvidenceError, DecisionBackedReviewedLegalEvidenceSelection,
    LegalEvidenceReviewDecisionDraft, LegalEvidenceReviewDecisionError,
    RealMatterReviewGateDraft, RealMatterReviewGateReceipt,
    RealMatterReviewedResumeError, RealMatterReviewedResumeReceipt,
    LEGAL_EVIDENCE_REVIEW_DECISION_SCHEMA_SQL,
};

#[test]
fn human_role_and_normative_order_are_durable_decision_coordinates() {
    let draft = LegalEvidenceReviewDecisionDraft {
        review_receipt_ref: "review-command:1".into(),
        observation_ref: "observation:1".into(),
        consumer_ref: "matter:1".into(),
        requirement_ref: "requirement:1".into(),
        evidence_role_ref: "evidence-role:primary-authority".into(),
        normative_order_ref: "normative-order:australian-common-law".into(),
        proposition_ref: "proposition:1".into(),
        source_manifestation_ref: "legal-source-manifestation:1".into(),
        candidate_pnf_batch_ref: "candidate-pnf-batch:1".into(),
        candidate_factor_ref: "candidate:1".into(),
    };
    assert_eq!(draft.evidence_role_ref, "evidence-role:primary-authority");
    assert_eq!(draft.normative_order_ref, "normative-order:australian-common-law");
    assert!(LEGAL_EVIDENCE_REVIEW_DECISION_SCHEMA_SQL.contains("normative_order_ref"));
}

#[test]
fn production_selection_does_not_expose_role_or_normative_order_fields() {
    let selection = DecisionBackedReviewedLegalEvidenceSelection {
        review_decision_ref: "legal-evidence-review-decision:1".into(),
        pnf_build_ref: "pnf-build:1".into(),
        refined_pnf_graph_ref: "pnf-graph:1".into(),
        pnf_factor_ref: "pnf-factor:1".into(),
        pnf_revision_ref: "pnf-revision:1".into(),
        structural_signature_ref: "signature:1".into(),
        predicate_ref: "predicate:1".into(),
        legal_system_refs: vec!["legal-system:au".into()],
        jurisdiction_refs: vec!["AU".into()],
        temporal_refs: vec![],
        author_ref: "author:1".into(),
        institution_ref: None,
    };
    assert_eq!(selection.review_decision_ref, "legal-evidence-review-decision:1");

    let _ = std::any::TypeId::of::<LegalEvidenceReviewDecisionError>();
    let _ = std::any::TypeId::of::<DecisionBackedReviewedLegalEvidenceError>();
}

#[test]
fn real_matter_public_surface_has_prepare_and_resume_receipts() {
    let draft = RealMatterReviewGateDraft {
        observation_ref: "observation:1".into(),
        consumer_ref: "matter:1".into(),
        reason: "genuine provider-backed legal evidence requires human review".into(),
        source_manifestation_ref: "legal-source-manifestation:1".into(),
        candidate_pnf_batch_ref: "candidate-pnf-batch:1".into(),
        candidate_factor_ref: "candidate:1".into(),
    };
    assert_eq!(draft.consumer_ref, "matter:1");
    let _ = std::any::TypeId::of::<RealMatterReviewGateReceipt>();
    let _ = std::any::TypeId::of::<RealMatterReviewedResumeReceipt>();
    let _ = std::any::TypeId::of::<RealMatterReviewedResumeError>();
}
