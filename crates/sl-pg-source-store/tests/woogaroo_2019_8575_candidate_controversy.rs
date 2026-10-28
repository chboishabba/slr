//! EPBC 2019/8575 native SLR production case boundary tests.
//! Case model is owned by sl-pg-source-store::woogaroo_case, not Python.
//! Source-written fixtures are NOT paid case evidence, litigation or judgment.
use sensiblaw_pg_source_store::{
    project_matter_reverse_proof_search, validate_matter_controversy_draft,
    validate_woogaroo_candidate_matter, validate_woogaroo_case_cut,
    woogaroo_candidate_matter, woogaroo_locator_matches_springview,
    woogaroo_priority_frontier,
    MatterEpistemicStatus, MatterPartyRole, MatterProceduralGoal,
    MatterResponseMode, WoogarooCaseCutError, WoogarooGroundPriority,
    WOOGAROO_ACTION_REF, WOOGAROO_CASE_CONTRACT, WOOGAROO_GROUNDS,
    WOOGAROO_MATTER_REF, WOOGAROO_SOURCE_LOCATORS, WOOGAROO_PRECONDITIONS,
    SCENIC_ACTION_REF,
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
fn scenic_activity_is_other_action_and_locator_identity_does_not_pay_merits() {
    assert_eq!(woogaroo_locator_matches_springview("swfg-scenic-letter"), Some(false));
    assert_eq!(woogaroo_locator_matches_springview("qut-springview-letter"), Some(true));
    assert_eq!(woogaroo_locator_matches_springview("stockland-springview"), Some(true));
    assert_eq!(woogaroo_locator_matches_springview("not-acquired"), None);
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
    sources[2].key = "epbc-decision-status";
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
fn procedural_standing_clock_works_and_signed_conditions_remain_unpaid() {
    assert_eq!(WOOGAROO_PRECONDITIONS.len(), 4);
    assert!(WOOGAROO_PRECONDITIONS.iter().all(|precondition|
        !precondition.document_paid && !precondition.competent_decision_paid
        && !precondition.key.is_empty()
        && !precondition.required_original.is_empty()
    ));
    assert!(WOOGAROO_PRECONDITIONS.iter().any(|p|
        p.key == "applicant-specific-adjr-clock"));
    assert!(WOOGAROO_PRECONDITIONS.iter().any(|p|
        p.key == "actual-stage-works-and-urgency"));
    assert!(WOOGAROO_PRECONDITIONS.iter().any(|p|
        p.key == "signed-federal-decision-package"));
}

#[test]
fn pareto_frontier_is_a_deterministic_evidence_queue_not_a_success_score() {
    let frontier = woogaroo_priority_frontier();
    assert_eq!(frontier.len(), 7);
    let urgent: Vec<_> = frontier.iter()
        .take_while(|g| g.priority == WoogarooGroundPriority::UrgentProcedural)
        .map(|g| g.key).collect();
    assert_eq!(urgent.len(), 4);
    assert_eq!(frontier[0].key, "advice-consideration");
    assert_eq!(frontier.last().expect("not empty").key, "improper-influence");
    assert!(frontier.iter().all(|g| g.evidence_gate == 0));
    assert_eq!(frontier, woogaroo_priority_frontier());
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

use sensiblaw_pg_source_store::{
    validate_woogaroo_closure_contract, woogaroo_closure_frontier,
    WoogarooClosureClass, WoogarooClosureError, WOOGAROO_ACQUISITION_PACKETS,
    WOOGAROO_CLOSURE_OBLIGATIONS,
};

#[test]
fn woogaroo_full_closure_has_80_distinct_unpaid_targets() {
    validate_woogaroo_closure_contract(
        &WOOGAROO_CLOSURE_OBLIGATIONS, &WOOGAROO_ACQUISITION_PACKETS,
    ).expect("closure ledger is source-written and explicitly unpaid");
    assert_eq!(WOOGAROO_CLOSURE_OBLIGATIONS.len(), 80);
    let counts = [
        (WoogarooClosureClass::Identity, 8),
        (WoogarooClosureClass::Provenance, 8),
        (WoogarooClosureClass::Procedure, 8),
        (WoogarooClosureClass::DecisionFile, 8),
        (WoogarooClosureClass::Ground, 28),
        (WoogarooClosureClass::Ecology, 8),
        (WoogarooClosureClass::Verification, 7),
        (WoogarooClosureClass::Outcome, 5),
    ];
    for (class, expected) in counts {
        assert_eq!(
            WOOGAROO_CLOSURE_OBLIGATIONS.iter().filter(|o| o.class == class).count(),
            expected, "incorrect obligation count for {class:?}",
        );
    }
    assert!(WOOGAROO_CLOSURE_OBLIGATIONS.iter().all(|o| !o.discharged));
    assert_eq!(WOOGAROO_CLOSURE_OBLIGATIONS.first().unwrap().id, "I01");
    assert_eq!(WOOGAROO_CLOSURE_OBLIGATIONS.last().unwrap().id, "O05");
}

#[test]
fn closure_ground_crosswalk_is_exact_and_keeps_all_seven_claims_open() {
    for ground in WOOGAROO_GROUNDS {
        let rows: Vec<_> = WOOGAROO_CLOSURE_OBLIGATIONS.iter()
            .filter(|o| o.ground_key == Some(ground.key)).collect();
        assert_eq!(rows.len(), 4, "each existing ground has four discriminators");
        assert!(rows.iter().all(|o| {
            o.class == WoogarooClosureClass::Ground && !o.discharged
        }));
        assert_eq!(ground.evidence_gate, 0);
        assert!(ground.positive_witness_ref.is_none());
        assert!(ground.negative_witness_ref.is_none());
        assert!(ground.court_filing_ref.is_none());
    }
    let mut rows = WOOGAROO_CLOSURE_OBLIGATIONS;
    rows[32].ground_key = Some("not-an-existing-legal-ground");
    assert_eq!(
        validate_woogaroo_closure_contract(&rows, &WOOGAROO_ACQUISITION_PACKETS),
        Err(WoogarooClosureError::UnknownGround),
    );
}

#[test]
fn synthetic_source_payment_and_mutated_obligation_are_never_accepted() {
    let mut rows = WOOGAROO_CLOSURE_OBLIGATIONS;
    rows[0].discharged = true;
    assert_eq!(
        validate_woogaroo_closure_contract(&rows, &WOOGAROO_ACQUISITION_PACKETS),
        Err(WoogarooClosureError::ImproperDischarge),
    );
    let mut rows = WOOGAROO_CLOSURE_OBLIGATIONS;
    rows[1].id = "I01";
    assert_eq!(
        validate_woogaroo_closure_contract(&rows, &WOOGAROO_ACQUISITION_PACKETS),
        Err(WoogarooClosureError::DuplicateObligation),
    );
    let mut rows = WOOGAROO_CLOSURE_OBLIGATIONS;
    rows[0].primary_packet = "synthetic-paid-original";
    assert_eq!(
        validate_woogaroo_closure_contract(&rows, &WOOGAROO_ACQUISITION_PACKETS),
        Err(WoogarooClosureError::UncoveredObligation),
    );
    let mut rows = WOOGAROO_CLOSURE_OBLIGATIONS;
    rows[0].title = "wrong-source";
    assert_eq!(
        validate_woogaroo_closure_contract(&rows, &WOOGAROO_ACQUISITION_PACKETS),
        Err(WoogarooClosureError::WrongCanonicalObligation),
    );
}

#[test]
fn max_cut_packets_derive_all_open_queries_without_success_probabilities() {
    let frontier = woogaroo_closure_frontier();
    assert_eq!(frontier.len(), 12);
    assert_eq!(frontier[0].packet_key, "counsel");
    assert_eq!(frontier[1].packet_key, "signed");
    assert_eq!(frontier[2].packet_key, "brief");
    assert_eq!(frontier[3].packet_key, "comments");
    assert_eq!(frontier[4].packet_key, "works");
    assert_eq!(frontier.last().unwrap().packet_key, "conflict");
    assert_eq!(frontier.iter().map(|p| p.unpaid_obligations.len()).sum::<usize>(), 80);
    assert!(frontier.iter().all(|p| !p.unpaid_obligations.is_empty()));
    assert!(frontier.iter().flat_map(|p| p.unpaid_obligations.iter()).all(|id|
        WOOGAROO_CLOSURE_OBLIGATIONS.iter().any(|o| o.id == *id && !o.discharged)
    ));
    assert_eq!(frontier, woogaroo_closure_frontier());
    let mut packets = WOOGAROO_ACQUISITION_PACKETS;
    packets[1].ordinal_priority = 0;
    assert_eq!(
        validate_woogaroo_closure_contract(&WOOGAROO_CLOSURE_OBLIGATIONS, &packets),
        Err(WoogarooClosureError::InvalidPriority),
    );
}


#[test]
fn source_snowball_is_a_potential_reopening_cone_only() {
    use sensiblaw_pg_source_store::woogaroo_potential_reopening_cone;
    for packet in woogaroo_closure_frontier() {
        let cone = woogaroo_potential_reopening_cone(packet.packet_key)
            .expect("all canonical acquisition packets have a cone");
        assert_eq!(cone.seed_ref, format!("packet:woogaroo:{}", packet.packet_key));
        assert_eq!(cone.direct_refs.len(), packet.unpaid_obligations.len());
        assert!(packet.unpaid_obligations.iter().all(|id|
            cone.direct_refs.contains(&format!("obligation:woogaroo:{id}"))
        ));
        assert!(!cone.creates_actual_reopening);
        assert!(!cone.creates_semantic_authority);
    }
    let advice_cone = woogaroo_potential_reopening_cone("brief").unwrap();
    assert!(advice_cone.transitive_refs.contains(
        &"ground:woogaroo:advice-consideration".to_owned()
    ));
    assert!(advice_cone.transitive_refs.contains(
        &"matter:woogaroo:candidate-seven-ground-review".to_owned()
    ));
    assert!(woogaroo_potential_reopening_cone("unknown-or-synthetic-source").is_err());
}


#[test]
fn all_seven_grounds_have_exhaustive_hypothetical_for_against_tests() {
    use sensiblaw_pg_source_store::{
        woogaroo_hypothetical_adversarial_test, WoogarooHypotheticalWorld,
    };
    let mut ground_worlds = 0usize;
    for ground in WOOGAROO_GROUNDS {
        // Six independent Boolean premises: exactly 64 worlds per ground.
        for mask in 0u8..64u8 {
            let w = WoogarooHypotheticalWorld {
                legally_applicable: mask & 1 != 0,
                defect_established: mask & 2 != 0,
                materiality_established: mask & 4 != 0,
                government_compliance_established: mask & 8 != 0,
                evidence_reviewed: mask & 16 != 0,
                ecological_harm_assumed: mask & 32 != 0,
            };
            let result = woogaroo_hypothetical_adversarial_test(ground.key, w)
                .expect("all canonical ground IDs are accepted");
            ground_worlds += 1;
            assert_eq!(result.ground_key, ground.key);
            assert_eq!(result.applicant_conditional,
                !w.government_compliance_established && w.evidence_reviewed
                && w.legally_applicable && w.defect_established
                && w.materiality_established);
            assert_eq!(result.respondent_conditional,
                w.government_compliance_established && w.evidence_reviewed
                && w.legally_applicable);
            assert!(!(result.applicant_conditional && result.respondent_conditional));
            assert!(!result.proves_actual_legal_merits);
            assert!(!result.creates_legal_evidence_review);
            assert!(!result.creates_court_filing);
            if !w.evidence_reviewed {
                assert!(!result.applicant_conditional);
                assert!(!result.respondent_conditional);
            }
        }
    }
    assert_eq!(ground_worlds, 7 * 64);
    assert!(woogaroo_hypothetical_adversarial_test(
        "fictional-springview-court-ground",
        WoogarooHypotheticalWorld {
            legally_applicable: true,
            defect_established: true,
            materiality_established: true,
            government_compliance_established: false,
            evidence_reviewed: true,
            ecological_harm_assumed: true,
        },
    ).is_none());
}

#[test]
fn ecological_harm_does_not_entail_unlawfulness_and_compliance_does_not_deny_harm() {
    use sensiblaw_pg_source_store::{
        woogaroo_hypothetical_adversarial_test, WoogarooHypotheticalWorld,
    };
    let w = WoogarooHypotheticalWorld {
        legally_applicable: true,
        defect_established: false,
        materiality_established: false,
        government_compliance_established: true,
        evidence_reviewed: true,
        ecological_harm_assumed: true,
    };
    let result = woogaroo_hypothetical_adversarial_test("advice-consideration", w)
        .expect("existing 2022 advice ground");
    assert!(w.ecological_harm_assumed);
    assert!(!result.applicant_conditional);
    assert!(result.respondent_conditional);
    assert!(!result.proves_actual_legal_merits);
}
