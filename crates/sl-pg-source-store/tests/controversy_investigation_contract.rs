use sensiblaw_pg_source_store::{
    build_controversy_evidence_stage, controversy_acquisition_pareto_frontier,
    controversy_obligation_from_parts, AcquisitionRouteCandidate, AcquisitionUpdate,
    AccessDisposition, ControversyReviewedEvidenceDischarge, DisagreementKind,
    DuplicateRelation, EpistemicStatus, EvidenceIndependence, LegalControversyMatter,
    LegalControversyResidual, LegalProofObligation, LegalPropositionFibre, LegalRole,
    ObligationKind, PartyRole, RecordAvailability, SelectiveReopeningReceipt,
};

fn fibre(reference: &str, party: PartyRole, reviewed: &str) -> LegalPropositionFibre {
    LegalPropositionFibre {
        fibre_ref: reference.into(),
        claim_ref: format!("claim:{reference}"),
        party,
        legal_role: LegalRole::LegalProposition,
        epistemic_status: EpistemicStatus::Supported,
        evidence_kind_ref: "source_text".into(),
        source_reference: format!("span:{reference}"),
        reviewed_evidence_ref: Some(reviewed.into()),
        normative_order_ref: "order:common-law".into(),
        temporal_reference: "temporal:unknown".into(),
        relation_reference: format!("proposition:{reference}"),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn fixture() -> (
    LegalControversyMatter,
    LegalControversyResidual,
    LegalProofObligation,
    Vec<LegalPropositionFibre>,
) {
    let matter = LegalControversyMatter {
        controversy_ref: "controversy:1".into(),
        matter_ref: "matter:1".into(),
        root_fibre_ref: "fibre:applicant".into(),
        proposition_fibre_refs: vec!["fibre:applicant".into(), "fibre:respondent".into()],
        response_refs: vec!["response:1".into()],
        residual_refs: vec!["residual:1".into()],
        obligation_refs: vec!["proof:1".into()],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let residual = LegalControversyResidual {
        residual_ref: "residual:1".into(),
        controversy_ref: "controversy:1".into(),
        kind: DisagreementKind::Evidence,
        applicant_fibre_ref: "fibre:applicant".into(),
        respondent_fibre_ref: "fibre:respondent".into(),
        residual_level_ref: "residual:open".into(),
        unresolved_question: "Which contemporaneous record discriminates the accounts?".into(),
        requested_discriminator: Some("contemporaneous independent record".into()),
        target_evidence_query: Some("find contemporaneous independent record".into()),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let proof = LegalProofObligation {
        obligation_ref: "proof:1".into(),
        controversy_ref: "controversy:1".into(),
        proposition_fibre_ref: "fibre:applicant".into(),
        kind: ObligationKind::Discriminator,
        required_by: "residual:1".into(),
        discharge_reference: None,
        is_open: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let fibres = vec![
        fibre("fibre:applicant", PartyRole::Applicant, "reviewed:a"),
        fibre("fibre:respondent", PartyRole::Respondent, "reviewed:b"),
    ];
    (matter, residual, proof, fibres)
}

fn route(obligation_ref: &str, id: &str, gain: u32, cost: u32) -> AcquisitionRouteCandidate {
    AcquisitionRouteCandidate {
        route_ref: id.into(),
        obligation_ref: obligation_ref.into(),
        route_description: format!("lawful route {id}"),
        source_locator_ref: format!("locator:{id}"),
        access_disposition: AccessDisposition::Public,
        authority_receipt_ref: None,
        provenance_genealogy_ref: format!("genealogy:{id}"),
        independence: EvidenceIndependence::Unknown,
        independence_receipt_ref: None,
        duplicate_relation: DuplicateRelation::Unknown,
        information_gain: gain,
        dependency_closure_impact: 4,
        residual_coverage: 3,
        provenance_novelty: 2,
        acquisition_cost: cost,
        axis_estimation_receipt_ref: "axis-review:1".into(),
        candidate_only: true,
        creates_acquisition_authority: false,
    }
}

#[test]
fn open_controversy_obligation_becomes_targeted_non_scalar_acquisition_work() {
    let (matter, residual, proof, fibres) = fixture();
    let obligation = controversy_obligation_from_parts(
        &matter,
        &residual,
        &proof,
        &fibres,
        vec!["source:a".into(), "source:b".into()],
        "access:lawful-public-records",
        vec!["proof:1".into(), "residual:1".into(), "controversy:1".into()],
    )
    .expect("valid controversy acquisition obligation");

    assert_eq!(obligation.matter_ref, "matter:1");
    assert_eq!(obligation.legal_obligation_ref, "proof:1");
    assert_eq!(obligation.requested_discriminator, "contemporaneous independent record");
    assert_eq!(obligation.target_evidence_query, "find contemporaneous independent record");
    assert_eq!(obligation.current_availability, RecordAvailability::NotLocated);

    let priority = controversy_acquisition_pareto_frontier(
        &obligation,
        &[
            route(&obligation.obligation_ref, "route:better", 10, 2),
            route(&obligation.obligation_ref, "route:worse", 9, 3),
            route(&obligation.obligation_ref, "route:cheap", 5, 1),
        ],
    )
    .expect("valid routes");
    assert_eq!(priority.frontier_route_refs, vec!["route:better", "route:cheap"]);
    assert!(!priority.scalar_score_used);
    assert!(!priority.creates_screening_or_acquisition_decision);
}

#[test]
fn two_adversarial_fibres_may_share_one_source_revision() {
    let (matter, residual, proof, fibres) = fixture();
    let obligation = controversy_obligation_from_parts(
        &matter,
        &residual,
        &proof,
        &fibres,
        vec!["source:same".into(), "source:same".into()],
        "access:lawful-public-records",
        vec![],
    )
    .expect("competing constructions of one source remain a valid controversy");

    assert_eq!(obligation.source_revision_refs, vec!["source:same"]);
    assert!(obligation
        .dependency_target_refs
        .contains(&"controversy:1".to_string()));
    assert!(!obligation.creates_semantic_authority);
    assert!(!obligation.creates_legal_authority);
    assert!(!obligation.claim_truth_promoted);
}

#[test]
fn acquired_source_requires_human_review_bound_to_the_exact_legal_obligation() {
    let (matter, residual, proof, fibres) = fixture();
    let obligation = controversy_obligation_from_parts(
        &matter,
        &residual,
        &proof,
        &fibres,
        vec!["source:a".into(), "source:b".into()],
        "access:lawful-public-records",
        vec!["proof:1".into(), "residual:1".into(), "controversy:1".into()],
    )
    .unwrap();
    let update = AcquisitionUpdate {
        obligation_ref: obligation.obligation_ref.clone(),
        before: RecordAvailability::NotLocated,
        after: RecordAvailability::Present,
        acquired_source_revision_ref: Some("source:new".into()),
        acquisition_receipt_ref: "acquisition:new".into(),
    };
    let reopening = SelectiveReopeningReceipt {
        changed_source_revision_ref: "source:new".into(),
        directly_affected_refs: vec!["reviewed:new".into()],
        transitively_affected_refs: vec!["proof:1".into(), "residual:1".into(), "controversy:1".into()],
        unrelated_refs_not_reopened: vec!["unrelated:1".into()],
        dependency_graph_ref: "dependencies:matter:1".into(),
        creates_semantic_authority: false,
    };

    let wrong_source = build_controversy_evidence_stage(
        &obligation,
        &update,
        &ControversyReviewedEvidenceDischarge {
            reviewed_evidence_ref: "reviewed:new".into(),
            requirement_ref: "proof:1".into(),
            source_revision_ref: "source:other".into(),
        },
        &reopening,
    )
    .expect_err("review of another source cannot discharge the acquisition branch");
    assert!(wrong_source.to_string().contains("reviewed evidence"));

    let wrong_requirement = build_controversy_evidence_stage(
        &obligation,
        &update,
        &ControversyReviewedEvidenceDischarge {
            reviewed_evidence_ref: "reviewed:new".into(),
            requirement_ref: "proof:other".into(),
            source_revision_ref: "source:new".into(),
        },
        &reopening,
    )
    .expect_err("review must be explicitly scoped to the legal proof obligation");
    assert!(wrong_requirement.to_string().contains("requirement"));
}

#[test]
fn reviewed_acquisition_reopens_the_controversy_without_auto_adjudicating_the_residual() {
    let (matter, residual, proof, fibres) = fixture();
    let obligation = controversy_obligation_from_parts(
        &matter,
        &residual,
        &proof,
        &fibres,
        vec!["source:a".into(), "source:b".into()],
        "access:lawful-public-records",
        vec!["proof:1".into(), "residual:1".into(), "controversy:1".into()],
    )
    .unwrap();
    let update = AcquisitionUpdate {
        obligation_ref: obligation.obligation_ref.clone(),
        before: RecordAvailability::NotLocated,
        after: RecordAvailability::Present,
        acquired_source_revision_ref: Some("source:new".into()),
        acquisition_receipt_ref: "acquisition:new".into(),
    };
    let reopening = SelectiveReopeningReceipt {
        changed_source_revision_ref: "source:new".into(),
        directly_affected_refs: vec!["reviewed:new".into()],
        transitively_affected_refs: vec!["proof:1".into(), "residual:1".into(), "controversy:1".into()],
        unrelated_refs_not_reopened: vec!["unrelated:1".into()],
        dependency_graph_ref: "dependencies:matter:1".into(),
        creates_semantic_authority: false,
    };
    let stage = build_controversy_evidence_stage(
        &obligation,
        &update,
        &ControversyReviewedEvidenceDischarge {
            reviewed_evidence_ref: "reviewed:new".into(),
            requirement_ref: "proof:1".into(),
            source_revision_ref: "source:new".into(),
        },
        &reopening,
    )
    .expect("reviewed evidence reopens exact affected controversy");

    assert!(stage.evidence_supplied);
    assert!(stage.controversy_reopened);
    assert!(stage.requires_residual_reassessment);
    assert!(!stage.residual_resolved);
    assert!(!stage.determines_credibility);
    assert!(!stage.determines_ultimate_fact);
    assert!(!stage.creates_semantic_authority);
    assert!(!stage.creates_legal_authority);
    assert_eq!(stage.unrelated_refs_not_reopened, vec!["unrelated:1"]);
}
