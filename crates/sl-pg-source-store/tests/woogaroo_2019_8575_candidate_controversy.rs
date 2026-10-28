//! EPBC 2019/8575 native SLR production case boundary tests.
//! Case model is owned by sl-pg-source-store::woogaroo_case, not Python.
//! Source-written fixtures are NOT paid case evidence, litigation or judgment.
use sensiblaw_pg_source_store::{
    project_matter_reverse_proof_search, validate_matter_controversy_draft,
    validate_woogaroo_candidate_matter, validate_woogaroo_case_cut,
    woogaroo_candidate_matter, woogaroo_same_action_merits_source,
    MatterEpistemicStatus, MatterPartyRole, MatterProceduralGoal,
    MatterResponseMode, WoogarooCaseCutError, WoogarooGroundPriority,
    WOOGAROO_ACTION_REF, WOOGAROO_CASE_CONTRACT, WOOGAROO_GROUNDS,
    WOOGAROO_MATTER_REF, WOOGAROO_SOURCE_LOCATORS, SCENIC_ACTION_REF,
};

#[test]
fn source_cut_is_exactly_one_project_and_all_published_locators_unpaid() {
    validate_woogaroo_case_cut(&WOOGAROO_SOURCE_LOCATORS, &WOOGAROO_GROUNDS)
        .expect("case source cut is a valid candidate-only manifest");
    assert!(WOOGAROO_CASE_CONTRACT.starts_with("slr:"));
    assert_eq!(WOOGAROO_ACTION_REF, "2019/8575");
    assert_eq!(SCENIC_ACTION_REF, "2020/8651");
    assert_eq!(WOOGAROO_SOURCE_LOCATORS.len(), 9);
    assert_eq!(WOOGAROO_GROUNDS.len(), 7);
    assert!(WOOGAROO_SOURCE_LOCATORS.iter().all(|s| {
        !s.source_or_case_evidence_paid()
            && s.retained_source_revision_ref.is_none()
            && s.canonical_content_sha256.is_none()
            && s.exact_source_span_ref.is_none()
            && s.reviewed_case_evidence_ref.is_none()
    }));
    assert!(WOOGAROO_GROUNDS.iter().all(|g| {
        g.evidence_gate == 0
            && g.positive_witness_ref.is_none()
            && g.negative_witness_ref.is_none()
            && g.court_filing_ref.is_none()
    }));
}

#[test]
fn scenic_activity_is_not_springview_merits_and_an_unseen_source_stays_unknown() {
    assert_eq!(woogaroo_same_action_merits_source("swfg-scenic-letter"), Some(false));
    assert_eq!(woogaroo_same_action_merits_source("qut-springview-letter"), Some(true));
    assert_eq!(woogaroo_same_action_merits_source("stockland-springview"), Some(true));
    assert_eq!(woogaroo_same_action_merits_source("not-acquired"), None);
    let scenic = WOOGAROO_SOURCE_LOCATORS.iter()
        .find(|s| s.epbc_action == SCENIC_ACTION_REF).expect("distinct Scenic");
    assert_eq!(scenic.key, "swfg-scenic-letter");
    assert!(scenic.standing_activity_candidate);
}

#[test]
fn promotion_into_a_paid_case_source_or_legal_ground_is_rejected() {
    let mut sources = WOOGAROO_SOURCE_LOCATORS;
    sources[1].reviewed_case_evidence_ref = Some("synthetic-receipt:unreviewed");
    assert_eq!(
        validate_woogaroo_case_cut(&sources, &WOOGAROO_GROUNDS),
        Err(WoogarooCaseCutError::UnpaidSourcePresentedAsPaid),
    );
    let mut sources = WOOGAROO_SOURCE_LOCATORS;
    sources[0].exact_source_span_ref = Some("invented:span");
    assert_eq!(
        validate_woogaroo_case_cut(&sources, &WOOGAROO_GROUNDS),
        Err(WoogarooCaseCutError::UnpaidSourcePresentedAsPaid),
    );
    let mut sources = WOOGAROO_SOURCE_LOCATORS;
    sources[6].epbc_action = WOOGAROO_ACTION_REF;
    assert_eq!(
        validate_woogaroo_case_cut(&sources, &WOOGAROO_GROUNDS),
        Err(WoogarooCaseCutError::WrongAction),
    );
    let mut sources = WOOGAROO_SOURCE_LOCATORS;
    sources[2].key = sources[1].key;
    assert_eq!(
        validate_woogaroo_case_cut(&sources, &WOOGAROO_GROUNDS),
        Err(WoogarooCaseCutError::DuplicateSource),
    );

    let mut grounds = WOOGAROO_GROUNDS;
    grounds[0].evidence_gate = 1;
    grounds[0].positive_witness_ref = Some("synthetic:fabricated-fit");
    assert_eq!(
        validate_woogaroo_case_cut(&WOOGAROO_SOURCE_LOCATORS, &grounds),
        Err(WoogarooCaseCutError::GroundPurportsToBePaid),
    );
    let mut grounds = WOOGAROO_GROUNDS;
    grounds[1].court_filing_ref = Some("not-a-real-case");
    assert_eq!(
        validate_woogaroo_case_cut(&WOOGAROO_SOURCE_LOCATORS, &grounds),
        Err(WoogarooCaseCutError::GroundPurportsToBePaid),
    );
}

#[test]
fn case_frontier_prioritises_real_decision_evidence_over_network_speculation() {
    let procedural: Vec<_> = WOOGAROO_GROUNDS.iter()
        .filter(|g| g.priority == WoogarooGroundPriority::UrgentProcedural)
        .map(|g| g.key).collect();
    assert_eq!(procedural, [
        "advice-consideration", "comment-transmission",
        "approval-scope", "local-prestart",
    ]);
    assert_eq!(WOOGAROO_GROUNDS[6].key, "improper-influence");
    assert_eq!(WOOGAROO_GROUNDS[6].priority, WoogarooGroundPriority::SourceOnlySpeculation);
}

#[test]
fn native_matter_is_unfiled_and_hypothetical_responses_are_not_admissions() {
    validate_woogaroo_candidate_matter().expect("valid SLR draft");
    let draft = woogaroo_candidate_matter();
    assert_eq!(draft.matter_ref, WOOGAROO_MATTER_REF);
    assert_eq!(draft.propositions.len(), 5);
    assert_eq!(draft.residuals.len(), 2);
    assert_eq!(draft.obligations.len(), 2);
    assert!(draft.stage_ref.contains("unreviewed"));
    assert!(draft.propositions.iter().all(|p| {
        p.reviewed_evidence_ref.is_none()
            && p.epistemic_status != MatterEpistemicStatus::Proved
            && p.matter_ref == WOOGAROO_MATTER_REF
    }));
    assert!(draft.responses.iter()
        .all(|r| r.mode == MatterResponseMode::ChallengeEvidenceReliability));
    assert!(draft.propositions.iter()
        .filter(|p| p.party == MatterPartyRole::Respondent)
        .all(|p| p.display_text.contains("HYPOTHETICAL DEFENCE")));
    assert!(draft.obligations.iter().all(|o| o.discharge_ref.is_none()));
}

#[test]
fn reverse_search_only_exposes_original_evidence_queries() {
    let draft = woogaroo_candidate_matter();
    let projection = project_matter_reverse_proof_search(
        &draft, MatterProceduralGoal::DecideEvidenceNeeded,
    ).expect("reuse canonical SLR reverse search");
    assert_eq!(projection.target_proposition_ref, draft.root_proposition_ref);
    assert_eq!(projection.candidate_residual_refs.len(), 2);
    assert_eq!(projection.open_obligation_refs.len(), 2);
    assert!(projection.target_evidence_queries.iter()
        .any(|q| q.contains("Secretary's 95C report")));
    assert!(projection.target_evidence_queries.iter()
        .any(|q| q.contains("corridor polygons")));
    assert!(!projection.creates_actual_reopening);
    assert!(!projection.creates_access_authority);
    assert!(!projection.creates_semantic_authority);
    assert!(!projection.claim_truth_promoted);
}

#[test]
fn same_matter_and_review_existence_fail_closed() {
    let mut draft = woogaroo_candidate_matter();
    draft.propositions[3].matter_ref = "matter:au:epbc:2020-8651:scenic".into();
    assert!(validate_matter_controversy_draft(&draft).is_err());
    let mut draft = woogaroo_candidate_matter();
    draft.obligations[0].required_by_ref = "unpaid:unknown-residual".into();
    assert!(validate_matter_controversy_draft(&draft).is_err());
    let mut draft = woogaroo_candidate_matter();
    draft.propositions[2].display_text.clear();
    assert!(validate_matter_controversy_draft(&draft).is_err());
}
