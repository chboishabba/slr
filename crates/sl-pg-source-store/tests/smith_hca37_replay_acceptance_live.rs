use sensiblaw_pg_source_store::{
    load_database_config, materialize_and_persist_real_matter_replay_acceptance,
    DisagreementKind, ProceduralGoal, RealMatterControversyDraft, ResponseMode,
};

fn env_ref(name: &str) -> String {
    std::env::var(name)
        .unwrap_or_else(|_| panic!("{name} must identify an already-persisted reviewed Smith coordinate"))
}

#[test]
#[ignore = "requires source-pinned reviewed Smith [2026] HCA 37 rows"]
fn live_smith_hca37_freezes_two_pass_replay_without_authority_promotion() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let draft = RealMatterControversyDraft {
        matter_ref: env_ref("ITIR_SMITH_HCA37_MATTER_REF"),
        applicant_claim_ref: env_ref("ITIR_SMITH_HCA37_NARROW_CLAIM_REF"),
        applicant_reviewed_evidence_ref: env_ref("ITIR_SMITH_HCA37_NARROW_REVIEWED_EVIDENCE_REF"),
        applicant_normative_order_ref: env_ref("ITIR_SMITH_HCA37_APPLICANT_NORMATIVE_ORDER_REF"),
        respondent_claim_ref: env_ref("ITIR_SMITH_HCA37_WIDE_CLAIM_REF"),
        respondent_reviewed_evidence_ref: env_ref("ITIR_SMITH_HCA37_WIDE_REVIEWED_EVIDENCE_REF"),
        respondent_normative_order_ref: env_ref("ITIR_SMITH_HCA37_RESPONDENT_NORMATIVE_ORDER_REF"),
        response_mode: ResponseMode::AdmitOccurrenceDisputeCharacterisation,
        disagreement_kind: DisagreementKind::LegalConsequence,
        unresolved_question: "Whether, on the proper construction of Corrective Services Act 2006 (Qld) s 22, egg extraction and freezing is participation in assisted reproductive technology".into(),
        requested_discriminator: "Distinguish a purpose-connected construction from a construction requiring technology that directly assists conception/reproduction, while retaining purpose, statutory context, rights context and opinion-level provenance".into(),
        target_evidence_query: "Acquire/reopen reviewed primary legal materials that discriminate the competing constructions without treating acquisition, citation, or precedent search as automatic residual payment".into(),
        procedural_goal: ProceduralGoal::DecideEvidenceNeeded,
    };

    let receipt = materialize_and_persist_real_matter_replay_acceptance(
        &config,
        "fixture:smith-hca37:interpretation",
        &draft,
        2,
    )
    .expect("the reviewed Smith controversy must replay identically twice and freeze");

    assert_eq!(receipt.matter_ref, draft.matter_ref);
    assert_eq!(receipt.replay_count, 2);
    assert!(receipt.replay_stable);
    assert!(receipt.candidate_only);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.creates_legal_authority);
    assert!(!receipt.applicability_promoted);
    assert!(!receipt.claim_truth_promoted);
    println!("smith_replay_acceptance_ref={}", receipt.acceptance_ref);
    println!("smith_controversy_ref={}", receipt.controversy_ref);
    println!("smith_residual_ref={}", receipt.residual_ref);
    println!("smith_obligation_ref={}", receipt.obligation_ref);
    println!("smith_reverse_ref={}", receipt.reverse_ref);
}
