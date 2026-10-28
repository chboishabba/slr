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
                !w.defect_established && w.government_compliance_established
                && w.evidence_reviewed && w.legally_applicable);
            assert_eq!(result.contradictory_assumptions_retained,
                w.defect_established && w.government_compliance_established);
            if result.contradictory_assumptions_retained {
                assert!(!result.applicant_conditional);
                assert!(!result.respondent_conditional);
            }
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
fn conflicting_defect_and_compliance_claims_are_retained_not_exploded() {
    use sensiblaw_pg_source_store::{
        woogaroo_hypothetical_adversarial_test, WoogarooHypotheticalWorld,
    };
    for ground in WOOGAROO_GROUNDS {
        let result = woogaroo_hypothetical_adversarial_test(
            ground.key,
            WoogarooHypotheticalWorld {
                legally_applicable: true,
                defect_established: true,
                materiality_established: true,
                government_compliance_established: true,
                evidence_reviewed: true,
                ecological_harm_assumed: true,
            },
        ).unwrap();
        assert!(result.contradictory_assumptions_retained);
        assert!(!result.applicant_conditional);
        assert!(!result.respondent_conditional);
        assert!(!result.proves_actual_legal_merits);
    }
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


#[test]
fn dialectic_trialectic_output_preserves_three_lenses_and_unpaid_face() {
    use sensiblaw_pg_source_store::{
        woogaroo_synthesise_dialectic_trialectic, woogaroo_unpaid_trialectic_synthesis,
        WoogarooDialecticEdge, WoogarooHypotheticalWorld, WoogarooSynthesisKind,
        WoogarooTrialecticFace,
    };
    let current = woogaroo_unpaid_trialectic_synthesis();
    assert_eq!(current.len(), 7);
    for (packet, ground) in current.iter().zip(WOOGAROO_GROUNDS) {
        assert_eq!(packet.ground_key, ground.key);
        assert_eq!(packet.applicant_issue, ground.statutory_hook);
        assert_eq!(packet.decisive_original, ground.next_original);
        assert!(!packet.respondent_countercase.is_empty());
        assert_eq!(packet.open_ground_obligation_ids.len(), 4);
        assert!(!packet.creates_actual_reopening);
        let ground_rows: Vec<_> = WOOGAROO_CLOSURE_OBLIGATIONS.iter()
            .filter(|o| o.ground_key == Some(ground.key)).collect();
        assert_eq!(ground_rows.len(), 4);
        assert!(ground_rows.iter().all(|o| {
            packet.open_ground_obligation_ids.contains(&o.id)
                && packet.original_acquisition_packet_key == o.primary_packet
                && !o.discharged
        }));
        let cone = sensiblaw_pg_source_store::woogaroo_potential_reopening_cone(
            packet.original_acquisition_packet_key).unwrap();
        assert!(!cone.creates_actual_reopening);
        assert!(!cone.creates_semantic_authority);
        assert!(packet.open_ground_obligation_ids.iter().all(|id|
            cone.direct_refs.contains(&format!("obligation:woogaroo:{id}"))
        ));
        assert_eq!(packet.kind, WoogarooSynthesisKind::SeekOriginals);
        assert_eq!(packet.applicant_respondent_edge, WoogarooDialecticEdge::Unresolved);
        assert_eq!(packet.respondent_independent_edge, WoogarooDialecticEdge::Unresolved);
        assert_eq!(packet.independent_applicant_edge, WoogarooDialecticEdge::Unresolved);
        assert_eq!(packet.triadic_face, WoogarooTrialecticFace::Underdetermined);
        assert_eq!(packet.next_procedural_goal, MatterProceduralGoal::DecideEvidenceNeeded);
        assert!(packet.candidate_only);
        assert!(!packet.creates_actual_review);
        assert!(!packet.creates_court_filing);
        assert!(!packet.proves_legal_error_or_compliance);
        assert!(!packet.orders_physical_protection);
    }

    let hypothetical = WoogarooHypotheticalWorld {
        legally_applicable: true,
        defect_established: true,
        materiality_established: true,
        government_compliance_established: false,
        evidence_reviewed: true,
        ecological_harm_assumed: true,
    };
    let candidate = woogaroo_synthesise_dialectic_trialectic(
        "advice-consideration", hypothetical).expect("canonical ground");
    assert_eq!(candidate.kind, WoogarooSynthesisKind::ApplicantCandidateForDefeaterSearch);
    assert_eq!(candidate.applicant_respondent_edge, WoogarooDialecticEdge::ProvisionalApplicant);
    assert_eq!(candidate.next_procedural_goal, MatterProceduralGoal::IsolateResidualControversy);
    assert_eq!(candidate.triadic_face, WoogarooTrialecticFace::Underdetermined);
    // Hypothetical 'evidence_reviewed=true' is NOT a real source-store receipt.
    assert!(!candidate.creates_actual_review);
    assert!(!candidate.proves_legal_error_or_compliance);
}

#[test]
fn trialectic_routes_all_448_worlds_without_filling_missing_face() {
    use sensiblaw_pg_source_store::{
        woogaroo_synthesise_dialectic_trialectic,
        WoogarooDialecticEdge, WoogarooHypotheticalWorld, WoogarooSynthesisKind,
        WoogarooTrialecticFace,
    };
    let mut n = 0;
    for ground in WOOGAROO_GROUNDS {
        for mask in 0u8..64u8 {
            let w = WoogarooHypotheticalWorld {
                legally_applicable: mask & 1 != 0,
                defect_established: mask & 2 != 0,
                materiality_established: mask & 4 != 0,
                government_compliance_established: mask & 8 != 0,
                evidence_reviewed: mask & 16 != 0,
                ecological_harm_assumed: mask & 32 != 0,
            };
            let packet = woogaroo_synthesise_dialectic_trialectic(ground.key, w).unwrap();
            let contradictory = w.defect_established && w.government_compliance_established;
            let applicant = !w.government_compliance_established && w.evidence_reviewed
                && w.legally_applicable && w.defect_established && w.materiality_established;
            let respondent = w.government_compliance_established && !w.defect_established
                && w.evidence_reviewed && w.legally_applicable;
            let expected = if contradictory {
                WoogarooSynthesisKind::ReconcileOpposingSourceAssertions
            } else if applicant {
                WoogarooSynthesisKind::ApplicantCandidateForDefeaterSearch
            } else if respondent {
                WoogarooSynthesisKind::RespondentCandidateForCounterDefeaterSearch
            } else {
                WoogarooSynthesisKind::SeekOriginals
            };
            assert_eq!(packet.kind, expected);
            assert_eq!(packet.triadic_face, WoogarooTrialecticFace::Underdetermined);
            assert_eq!(packet.open_ground_obligation_ids.len(), 4);
            assert!(!packet.creates_actual_reopening);
            assert_eq!(packet.respondent_independent_edge, WoogarooDialecticEdge::Unresolved);
            assert_eq!(packet.independent_applicant_edge, WoogarooDialecticEdge::Unresolved);
            assert!(packet.candidate_only);
            assert!(!packet.creates_actual_review);
            assert!(!packet.proves_legal_error_or_compliance);
            assert!(!packet.creates_court_filing);
            assert!(!packet.orders_physical_protection);
            if contradictory {
                assert_eq!(packet.applicant_respondent_edge, WoogarooDialecticEdge::Unresolved);
            }
            n += 1;
        }
    }
    assert_eq!(n, 448);
}


#[test]
fn geometric_sophontic_reference_pairs_are_load_bearing_or_nuisance_by_construction() {
    use sensiblaw_pg_source_store::{
        woogaroo_geometric_pair, WoogarooGeometricIntervention,
        WoogarooHypotheticalWorld, WoogarooSynthesisKind,
    };
    for ground in WOOGAROO_GROUNDS {
        let before = WoogarooHypotheticalWorld {
            legally_applicable: true,
            defect_established: true,
            materiality_established: true,
            government_compliance_established: false,
            evidence_reviewed: false,
            ecological_harm_assumed: true,
        };
        let after = WoogarooHypotheticalWorld { evidence_reviewed: true, ..before };
        let flip = woogaroo_geometric_pair(
            ground.key, before, after, WoogarooGeometricIntervention::ReviewGate
        ).expect("single selected review gate perturbation");
        assert!(flip.changed_369_position);
        assert_eq!(flip.ternary_grid_steps, 2);
        assert!(flip.changed_logical_label);
        assert_eq!(flip.initial_synthesis, WoogarooSynthesisKind::SeekOriginals);
        assert_eq!(flip.perturbed_synthesis,
            WoogarooSynthesisKind::ApplicantCandidateForDefeaterSearch);
        assert!(!flip.model_pair_accuracy_measured);
        assert!(!flip.actual_legal_merits_determined);
        assert_eq!(flip.canonical.full_cube_cardinality, 27);
        assert_eq!(flip.canonical.full_fabric_cardinality, 19683);
        assert!(!flip.canonical.same_object_monster_action_established);
        assert!(!flip.canonical.learned_geometric_action_established);

        // A factual ecology change must not flip the specific statutory
        // review-gate classifier: a geometry move != a legal outcome.
        let with_harm = WoogarooHypotheticalWorld {
            legally_applicable: true,
            defect_established: false,
            materiality_established: false,
            government_compliance_established: true,
            evidence_reviewed: true,
            ecological_harm_assumed: true,
        };
        let without_harm = WoogarooHypotheticalWorld {
            ecological_harm_assumed: false, ..with_harm
        };
        let nuisance = woogaroo_geometric_pair(
            ground.key, with_harm, without_harm,
            WoogarooGeometricIntervention::EcologicalHarm,
        ).unwrap();
        assert!(nuisance.changed_369_position);
        assert_eq!(nuisance.ternary_grid_steps, 1);
        assert!(!nuisance.changed_logical_label);
        assert_eq!(nuisance.initial_synthesis,
            WoogarooSynthesisKind::RespondentCandidateForCounterDefeaterSearch);
        assert_eq!(nuisance.perturbed_synthesis, nuisance.initial_synthesis);
        assert_eq!(nuisance.canonical.applicant_cube, nuisance.perturbed.applicant_cube);
        assert_ne!(nuisance.canonical.respondent_cube, nuisance.perturbed.respondent_cube);
        assert_eq!(nuisance.canonical.independent_review_cube,
            nuisance.perturbed.independent_review_cube);

        // Any two-axis perturbation is not a valid one-fact pair.
        assert!(woogaroo_geometric_pair(
            ground.key, before, with_harm, WoogarooGeometricIntervention::ReviewGate
        ).is_none());
    }
}

#[test]
fn all_64_boolean_worlds_embed_in_ternary_369_without_loss_of_legal_inputs() {
    use std::collections::BTreeSet;
    use sensiblaw_pg_source_store::{woogaroo_369_chart, WoogarooHypotheticalWorld};
    let mut codes = BTreeSet::new();
    for mask in 0..64u8 {
        let world = WoogarooHypotheticalWorld {
            legally_applicable: mask & 1 != 0,
            defect_established: mask & 2 != 0,
            materiality_established: mask & 4 != 0,
            government_compliance_established: mask & 8 != 0,
            evidence_reviewed: mask & 16 != 0,
            ecological_harm_assumed: mask & 32 != 0,
        };
        let chart = woogaroo_369_chart(world);
        let coordinates = [
            chart.applicant_cube[0], chart.applicant_cube[1], chart.applicant_cube[2],
            chart.respondent_cube[0], chart.respondent_cube[1], chart.respondent_cube[2],
            chart.independent_review_cube[0], chart.independent_review_cube[1],
            chart.independent_review_cube[2],
        ];
        assert!(coordinates.iter().all(|v| *v == 0 || *v == 1));
        assert_eq!(chart.full_cube_cardinality, 27);
        assert_eq!(chart.full_fabric_cardinality, 27 * 27 * 27);
        assert!(!chart.actual_evidence_payment_established);
        assert!(!chart.same_object_monster_action_established);
        codes.insert(coordinates);
    }
    // The first three applicant coordinates, respondent compliance/harm,
    // and independent review coordinate retain all original six bits.
    // 64 unique Boolean inputs occupy only 64 of the ambient 19683 states.
    assert_eq!(codes.len(), 64);
}


#[test]
fn all_single_axis_369_perturbations_respect_grounded_pair_semantics() {
    use sensiblaw_pg_source_store::{
        woogaroo_geometric_pair, WoogarooGeometricIntervention, WoogarooHypotheticalWorld,
    };
    let mut tested = 0usize;
    for ground in WOOGAROO_GROUNDS {
        let mut review_label_flips = 0usize;
        let mut harm_false_flips = 0usize;
        for mask in 0u8..32 {
            let base = WoogarooHypotheticalWorld {
                legally_applicable: mask & 1 != 0,
                defect_established: mask & 2 != 0,
                materiality_established: mask & 4 != 0,
                government_compliance_established: mask & 8 != 0,
                evidence_reviewed: false,
                ecological_harm_assumed: mask & 16 != 0,
            };
            let changed_review = WoogarooHypotheticalWorld {
                evidence_reviewed: true, ..base
            };
            let review = woogaroo_geometric_pair(
                ground.key, base, changed_review, WoogarooGeometricIntervention::ReviewGate
            ).unwrap();
            assert!(review.changed_369_position);
            assert_eq!(review.ternary_grid_steps,
                if review.changed_logical_label { 2 } else { 1 });
            assert!(!review.model_pair_accuracy_measured);
            assert!(!review.actual_legal_merits_determined);
            if review.changed_logical_label { review_label_flips += 1; }

            let no_harm = WoogarooHypotheticalWorld {
                ecological_harm_assumed: false, ..base
            };
            let with_harm = WoogarooHypotheticalWorld {
                ecological_harm_assumed: true, ..no_harm
            };
            let harm = woogaroo_geometric_pair(
                ground.key, no_harm, with_harm,
                WoogarooGeometricIntervention::EcologicalHarm
            ).unwrap();
            assert!(harm.changed_369_position);
            assert_eq!(harm.ternary_grid_steps, 1);
            if harm.changed_logical_label { harm_false_flips += 1; }
            assert_eq!(harm.canonical.independent_review_cube,
                harm.perturbed.independent_review_cube);
            tested += 2;
        }
        // Review alone flips only under six sufficient antecedent
        // configurations per ground, not all 32 source-free scenarios.
        assert_eq!(review_label_flips, 6);
        assert_eq!(harm_false_flips, 0);
    }
    assert_eq!(tested, 7 * 32 * 2);
}


#[test]
fn signed_proposition_negative_is_minus_and_both_is_not_neither() {
    use sensiblaw_pg_source_store::{
        WoogarooEvidenceStatus, WoogarooSignedEvidence, WoogarooScopedSignedEvidence
    };
    let blank = WoogarooSignedEvidence::unknown();
    let positive = WoogarooSignedEvidence {
        positive_supported:true, negative_supported:false, original_reference_labels:vec![],
    };
    let negative = WoogarooSignedEvidence {
        positive_supported:false, negative_supported:true, original_reference_labels:vec![],
    };
    let make = |evidence: WoogarooSignedEvidence| WoogarooScopedSignedEvidence {
        ground_key: "advice-consideration".into(),
        predicate_key: "alleged-defect".into(),
        project_and_scope_ref: "epbc:2019/8575:advice-consideration".into(),
        evidence,
    };
    let left = make(positive.clone());
    let right = make(negative.clone());
    let both = left.merge_aligned(&right).unwrap().evidence;
    let mut different = make(negative.clone());
    different.predicate_key = "materiality".into();
    assert!(left.merge_aligned(&different).is_none());
    different.predicate_key = "alleged-defect".into();
    different.project_and_scope_ref = "epbc:2020/8651:advice-consideration".into();
    assert!(left.merge_aligned(&different).is_none());
    different.project_and_scope_ref = "epbc:2019/8575:advice-consideration".into();
    different.ground_key = "recovery-plan".into();
    assert!(left.merge_aligned(&different).is_none());
    assert_eq!(positive.coarse_signed_trit(), 1);
    assert_eq!(negative.coarse_signed_trit(), -1);
    assert_eq!(blank.coarse_signed_trit(), 0);
    assert_eq!(both.coarse_signed_trit(), 0);
    assert_eq!(blank.status(), WoogarooEvidenceStatus::Neither);
    assert_eq!(both.status(), WoogarooEvidenceStatus::Both);
    assert_ne!(blank.status(), both.status());
    assert_ne!(negative.status(), blank.status());
}

#[test]
fn signed_fibre_drives_paired_candidate_synthesis_without_truth_promotion() {
    use sensiblaw_pg_source_store::{
        woogaroo_synthesise_signed_fibre, woogaroo_signed_padic_prefix,
        WoogarooSignedClaimFibre, WoogarooSignedEvidence, WoogarooSignedSearchFocus,
    };
    for ground in WOOGAROO_GROUNDS {
        let mut f = WoogarooSignedClaimFibre::unpaid(ground.key);
        let unresolved = woogaroo_synthesise_signed_fibre(f.clone(), false).unwrap();
        assert_eq!(unresolved.signed_369.applicant_cube, [0,0,0]);
        assert!(unresolved.signed_369.conflict_predicate_keys.is_empty());
        assert_eq!(unresolved.focus, WoogarooSignedSearchFocus::AcquireOriginals);
        assert_eq!(unresolved.nested_predicate_tree.len(), 3);
        assert_eq!(unresolved.nested_predicate_tree.iter()
            .map(|x| x.children.len()).collect::<Vec<_>>(), vec![2,2,2]);
        assert_eq!(unresolved.four_unpaid_obligation_ids.len(), 4);
        assert!(unresolved.triadic_face_underdetermined);
        assert!(!unresolved.actual_source_reviewed);
        assert!(!unresolved.actual_legal_merits_decided);

        // −P is the negative version of EXACT SAME alleged-defect predicate.
        // These are separately stipulated P-claims on the same legal
        // ground. Without all common gates the negative sign alone
        // cannot activate a respondent synthesis.
        f.evidence[1] = WoogarooSignedEvidence {
            positive_supported:false,negative_supported:true,original_reference_labels:vec![],
        };
        let unbridged = woogaroo_synthesise_signed_fibre(f.clone(), true).unwrap();
        assert_eq!(unbridged.focus, WoogarooSignedSearchFocus::AcquireOriginals);
        for slot in [0usize, 3, 6, 7] {
            f.evidence[slot] = WoogarooSignedEvidence {
                positive_supported:true,negative_supported:false,
                original_reference_labels:vec![],
            };
        }
        let negative = woogaroo_synthesise_signed_fibre(f.clone(), true).unwrap();
        assert_eq!(negative.signed_369.applicant_cube[1], -1);
        assert_eq!(negative.focus, WoogarooSignedSearchFocus::TestRespondentCounterDefeaters);
        assert_eq!(negative.one_branch_padic_prefix.low_to_high_digits, vec![0,-1,0]);
        assert!(!negative.one_branch_padic_prefix.full_inverse_limit_established);

        let prefix = woogaroo_signed_padic_prefix(&[
            &f.evidence[0],&f.evidence[1],&f.evidence[2]
        ]);
        let mut extended = f.evidence.clone();
        extended.push(f.evidence[1].clone());
        let finer = woogaroo_signed_padic_prefix(&[
            &extended[0],&extended[1],&extended[2],&extended[9]
        ]);
        assert_eq!(prefix.low_to_high_digits, finer.low_to_high_digits[..3]);

        f.evidence[1].positive_supported = true;
        f.evidence[1].negative_supported = false;
        f.evidence[2] = WoogarooSignedEvidence {
            positive_supported:true,negative_supported:false,original_reference_labels:vec![],
        };
        // A positive compliance assertion blocks automatic applicant
        // synthesis; it does NOT certify legal compliance.
        let competing = woogaroo_synthesise_signed_fibre(f.clone(), true).unwrap();
        assert_eq!(competing.focus, WoogarooSignedSearchFocus::AcquireOriginals);
        f.evidence[3].positive_supported = false;
        f.evidence[3].negative_supported = true;
        let positive = woogaroo_synthesise_signed_fibre(f.clone(), true).unwrap();
        assert_eq!(positive.signed_369.applicant_cube[1], 1);
        assert_eq!(positive.focus, WoogarooSignedSearchFocus::TestApplicantDefeaters);
        assert_eq!(positive.signed_fibre.evidence[1].coarse_signed_trit(),1);

        f.evidence[1].negative_supported = true;
        let both = woogaroo_synthesise_signed_fibre(f.clone(), true).unwrap();
        assert_eq!(both.signed_369.applicant_cube[1], 0);
        assert_eq!(both.signed_369.conflict_predicate_keys,vec!["alleged-defect"]);
        assert_eq!(both.focus, WoogarooSignedSearchFocus::ReconcileContradiction);
        assert_eq!(both.nested_predicate_tree[0].children[0].evidence.status(),
            sensiblaw_pg_source_store::WoogarooEvidenceStatus::Both);
        // The projection is NOT lossless for both/neither. Fine fibre is.
        assert_eq!(unresolved.signed_369.applicant_cube, [0,0,0]);
        assert_ne!(both.signed_fibre.evidence[1].status(),
            unresolved.signed_fibre.evidence[1].status());
        assert!(!both.actual_source_reviewed);
        assert!(!both.actual_legal_merits_decided);
        assert_eq!(both.base_acquisition_packet, negative.base_acquisition_packet);

        f.proposition_scope = "different-project".into();
        assert!(woogaroo_synthesise_signed_fibre(f.clone(), false).is_none());
        f.proposition_scope=format!("epbc:2019/8575:{}",ground.key);
        f.evidence.pop();
        assert!(woogaroo_synthesise_signed_fibre(f, false).is_none());
    }
}

#[test]
fn all_unpaid_signed_outputs_keep_seven_distinct_fibres_and_obligation_roots() {
    use sensiblaw_pg_source_store::{
        woogaroo_unpaid_signed_fibre_synthesis, WoogarooSignedSearchFocus,
        WOOGAROO_SIGNED_PREDICATES,
    };
    let packets = woogaroo_unpaid_signed_fibre_synthesis();
    assert_eq!(packets.len(),7);
    assert_eq!(WOOGAROO_SIGNED_PREDICATES.len(),9);
    for (packet,ground) in packets.iter().zip(WOOGAROO_GROUNDS) {
        assert_eq!(packet.ground_key,ground.key);
        assert_eq!(packet.signed_fibre.ground_key,ground.key);
        assert_eq!(packet.focus,WoogarooSignedSearchFocus::AcquireOriginals);
        assert_eq!(packet.signed_fibre.evidence.len(),9);
        assert!(packet.signed_fibre.evidence.iter().all(|e|
            !e.positive_supported && !e.negative_supported &&
            e.original_reference_labels.is_empty()));
        assert!(packet.signed_369.conflict_predicate_keys.is_empty());
        assert!(packet.signed_369.image_is_only_a_coarse_observation);
        assert!(!packet.signed_369.actual_source_payment);
        assert!(!packet.signed_369.actual_mediation_face_glued);
        assert_eq!(packet.four_unpaid_obligation_ids.len(),4);
        assert!(!packet.actual_legal_merits_decided);
        assert!(!packet.actual_source_reviewed);
    }
}


#[test]
fn one_converged_synthesis_per_ground_preserves_signed_and_dialectical_owners() {
    use sensiblaw_pg_source_store::{
        woogaroo_unpaid_converged_synthesis, WoogarooSignedSearchFocus,
        WoogarooTrialecticFace,
    };
    let outputs = woogaroo_unpaid_converged_synthesis();
    assert_eq!(outputs.len(),7);
    for (output,g) in outputs.iter().zip(WOOGAROO_GROUNDS) {
        assert_eq!(output.canonical_ground,g.key);
        assert_eq!(output.candidate_dialectic.ground_key,g.key);
        assert_eq!(output.signed_predicate_hyperfabric.ground_key,g.key);
        assert_eq!(output.candidate_dialectic.original_acquisition_packet_key,
            output.signed_predicate_hyperfabric.base_acquisition_packet);
        assert_eq!(output.candidate_dialectic.open_ground_obligation_ids,
            output.signed_predicate_hyperfabric.four_unpaid_obligation_ids);
        assert_eq!(output.candidate_dialectic.open_ground_obligation_ids.len(),4);
        assert_eq!(output.signed_predicate_hyperfabric.focus,
            WoogarooSignedSearchFocus::AcquireOriginals);
        assert_eq!(output.candidate_dialectic.triadic_face,
            WoogarooTrialecticFace::Underdetermined);
        assert!(output.matched_source_obligations);
        assert!(output.p_adic_branch_is_only_a_prefix);
        assert!(!output.may_force_irreducible_face);
        assert!(!output.real_legal_conclusion);
        assert!(!output.signed_predicate_hyperfabric.actual_source_reviewed);
    }
}


#[test]
fn dialectic_xy_has_nine_encounters_each_with_three_independent_z_positions() {
    use std::collections::BTreeSet;
    use sensiblaw_pg_source_store::{
        woogaroo_xy_interaction_cell, woogaroo_xy_interaction_column,
        woogaroo_xy_transport_selected_z, woogaroo_xy_next_z_step,
        woogaroo_unpaid_xy_interaction_carrier, WoogarooDialecticPosition as P,
    };
    let mut total = 0;
    for g in WOOGAROO_GROUNDS {
        let carrier = woogaroo_unpaid_xy_interaction_carrier(g.key).unwrap();
        assert_eq!(carrier.xy_encounter_count, 9);
        assert_eq!(carrier.xyz_possible_cells, 27);
        assert_eq!(carrier.actual_x_position_selected, None);
        assert_eq!(carrier.actual_y_position_selected, None);
        assert_eq!(carrier.actual_z_position_selected, None);
        assert!(!carrier.selected_interaction_is_a_legal_conclusion);
        let mut all_cells=BTreeSet::new();
        let mut all_xy=BTreeSet::new();
        for x in P::ALL {
            for y in P::ALL {
                all_xy.insert(x.index()*3+y.index());
                let column = woogaroo_xy_interaction_column(g.key,x,y).unwrap();
                assert_eq!(column.len(),3);
                assert!(woogaroo_xy_transport_selected_z(g.key,x,y,None).is_none());
                assert!(woogaroo_xy_next_z_step(&column[2]).is_none());
                assert_eq!(woogaroo_xy_next_z_step(&column[0]),Some(column[1].clone()));
                assert_eq!(woogaroo_xy_next_z_step(&column[1]),Some(column[2].clone()));
                for z in P::ALL {
                    let cell=woogaroo_xy_interaction_cell(g.key,x,y,z).unwrap();
                    assert_eq!(cell.xy_plane_index,x.index()*3+y.index());
                    assert_eq!(cell.cell_index,x.index()*9+y.index()*3+z.index());
                    assert_eq!(cell.role_position_trits,[
                        x.embedded_trit(),y.embedded_trit(),z.embedded_trit()
                    ]);
                    assert_eq!(cell, column[z.index()]);
                    assert_eq!(Some(cell.clone()),
                        woogaroo_xy_transport_selected_z(g.key,x,y,Some(z)));
                    assert!(!cell.actual_z_mediation_witness_paid);
                    assert!(!cell.actual_source_review_paid);
                    assert!(!cell.actual_legal_merits_established);
                    all_cells.insert(cell.cell_index);
                    total+=1;
                }
                assert_eq!(column[0].x_position,column[2].x_position);
                assert_eq!(column[0].y_position,column[2].y_position);
                assert_ne!(column[0].z_position,column[2].z_position);
            }
        }
        assert_eq!(all_xy.len(),9);
        assert_eq!(all_cells.len(),27);
        assert_eq!(all_cells.into_iter().collect::<Vec<_>>(),
            (0usize..27usize).collect::<Vec<_>>());
    }
    assert_eq!(total,7*27);
    assert!(woogaroo_xy_interaction_column("wrong-referral",P::Thesis,P::Thesis)
        .is_none());
}

#[test]
fn converged_ground_preserves_xy_z_as_roles_and_polar_evidence_as_fibre() {
    use sensiblaw_pg_source_store::{
        woogaroo_unpaid_converged_synthesis, woogaroo_xy_interaction_cell,
        WoogarooDialecticPosition as P,
    };
    for packet in woogaroo_unpaid_converged_synthesis() {
        assert_eq!(packet.xy_interaction_27.ground_key,packet.canonical_ground);
        assert_eq!(packet.xy_interaction_27.xy_encounter_count,9);
        assert_eq!(packet.xy_interaction_27.xyz_possible_cells,27);
        assert_eq!(packet.xy_interaction_27.actual_z_position_selected,None);
        // Position code -1 is "thesis" HERE, NOT support for ¬P.
        let cell=woogaroo_xy_interaction_cell(
            packet.canonical_ground,P::Thesis,P::Antithesis,P::Synthesis).unwrap();
        assert_eq!(cell.role_position_trits,[-1,0,1]);
        assert_eq!(cell.xy_plane_index,1);
        assert_eq!(cell.cell_index,5);
        // All actual signed evidence remains unknown/unpaid independent of
        // the chosen hypothetical X/Y/Z stage positions.
        assert_eq!(packet.signed_predicate_hyperfabric.signed_369.applicant_cube,
            [0,0,0]);
        assert_eq!(packet.signed_predicate_hyperfabric.signed_369.respondent_cube,
            [0,0,0]);
        assert_eq!(packet.signed_predicate_hyperfabric.signed_369.independent_cube,
            [0,0,0]);
        assert!(packet.signed_predicate_hyperfabric.triadic_face_underdetermined);
        assert!(!packet.real_legal_conclusion);
    }
}
