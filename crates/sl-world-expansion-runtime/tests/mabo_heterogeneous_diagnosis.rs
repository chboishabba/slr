use sensiblaw_pg_source_store::{PropositionObservationRow, PropositionRows};
use sensiblaw_proof_search_loop::judgment_candidates::{
    CitationOccurrenceCandidate, LexicalTreatmentHint,
};
use sensiblaw_proof_search_loop::world_expansion::{ProducerLane, ResidualClass};
use sensiblaw_world_expansion_runtime::mabo_heterogeneous_diagnosis::{
    diagnose_mabo_proposition_research, expand_mabo_legal_follow_candidates,
    MABO_RADICAL_TITLE_PROPOSITION, MABO_RADICAL_TITLE_SPAN,
};

fn rows(exact_source_paid: bool, with_support: bool) -> PropositionRows {
    PropositionRows {
        proposition_ref: MABO_RADICAL_TITLE_PROPOSITION.into(),
        required_span_ref: MABO_RADICAL_TITLE_SPAN.into(),
        exact_source_paid,
        observations: if with_support {
            vec![PropositionObservationRow {
                observation_ref: "obs:support".into(),
                pnf_factor_ref: "factor:support".into(),
                pnf_revision_ref: "pnf-rev:1".into(),
                observation_provenance_refs: vec![MABO_RADICAL_TITLE_SPAN.into()],
                graph_source_span_refs: vec![MABO_RADICAL_TITLE_SPAN.into()],
                residual_refs: vec!["residual:pnf:temporal-scope".into()],
            }]
        } else {
            vec![]
        },
    }
}

#[test]
fn canonical_paid_support_retains_researchable_role_and_pnf_residuals() {
    let diagnosed = diagnose_mabo_proposition_research(&rows(true, true)).unwrap();

    let refs = diagnosed
        .moves
        .iter()
        .map(|move_| move_.residual.residual_ref.as_str())
        .collect::<Vec<_>>();

    assert!(refs.contains(&"mabo:residual:qualifier-unpaid"));
    assert!(refs.contains(&"mabo:residual:defeater-unpaid"));
    assert!(refs.contains(&"mabo:residual:comparator-unpaid"));
    assert!(refs.contains(&"residual:pnf:temporal-scope"));
    assert!(!refs.contains(&"reader-residual:exact-authority-span"));
    assert!(!refs.contains(&"reader-residual:proposition-support"));

    for move_ in diagnosed
        .moves
        .iter()
        .filter(|move_| move_.residual.residual_ref.starts_with("mabo:residual:"))
    {
        assert_eq!(move_.residual_class, ResidualClass::Legal);
        assert_eq!(move_.producer_lane, ProducerLane::GovernedLegal);
        assert!(move_.admissible);
    }
}

#[test]
fn missing_exact_source_generates_legal_source_residual_without_claiming_payment() {
    let diagnosed = diagnose_mabo_proposition_research(&rows(false, false)).unwrap();
    let source = diagnosed
        .moves
        .iter()
        .find(|move_| move_.residual.residual_ref == "reader-residual:exact-authority-span")
        .expect("missing exact source must become a legal residual");

    assert_eq!(source.residual_class, ResidualClass::Legal);
    assert_eq!(source.producer_lane, ProducerLane::GovernedLegal);
    assert_eq!(source.provider_operation_ref, "oalc:exact-mnc:[1992]-HCA-23");
    assert!(diagnosed.candidate_only);
    assert!(!diagnosed.creates_semantic_authority);
    assert!(!diagnosed.applicability_promoted);
    assert!(!diagnosed.claim_truth_promoted);
}

#[test]
fn wrong_proposition_or_span_fails_closed() {
    let mut bad = rows(true, true);
    bad.proposition_ref = "mabo:proposition:other".into();
    assert!(diagnose_mabo_proposition_research(&bad).is_err());

    let mut bad = rows(true, true);
    bad.required_span_ref = "span:other".into();
    assert!(diagnose_mabo_proposition_research(&bad).is_err());
}


#[test]
fn retained_role_research_uses_legal_follow_not_trigger_source_reacquisition() {
    let diagnosed = diagnose_mabo_proposition_research(&rows(true, true)).unwrap();
    for residual_ref in [
        "mabo:residual:qualifier-unpaid",
        "mabo:residual:defeater-unpaid",
        "mabo:residual:comparator-unpaid",
    ] {
        let move_ = diagnosed
            .moves
            .iter()
            .find(|move_| move_.residual.residual_ref == residual_ref)
            .unwrap();
        assert_eq!(move_.producer_lane, ProducerLane::GovernedLegal);
        assert!(move_
            .provider_operation_ref
            .starts_with("legal-follow:proposition-role:"));
        assert_eq!(
            move_.source_ref.as_deref(),
            Some("case:[1992]-HCA-23")
        );
        assert_ne!(
            move_.provider_operation_ref,
            "oalc:exact-mnc:[1992]-HCA-23"
        );
    }
}


fn citation(text: &str, locator: &str) -> CitationOccurrenceCandidate {
    CitationOccurrenceCandidate {
        document_ref: "document:mabo".into(),
        source_revision_ref: "source-revision:mabo".into(),
        canonical_text_sha256: "sha256:text".into(),
        paragraph_ordinal: 42,
        paragraph_locator_ref: locator.into(),
        reported_paragraph_label: Some("[42]".into()),
        citation_text: text.into(),
        paragraph_text: format!("considered {text}"),
        anchor_paragraph_locator_refs: vec![locator.into()],
        anchor_paragraph_texts: vec![format!("considered {text}")],
        lexical_treatment_hints: vec![LexicalTreatmentHint::ReliedOnCandidate],
        reviewed: false,
        candidate_only: true,
    }
}

#[test]
fn retained_legal_debt_expands_to_source_located_citation_moves_without_self_follow() {
    let diagnosis = diagnose_mabo_proposition_research(&rows(true, true)).unwrap();
    let expanded = expand_mabo_legal_follow_candidates(
        &diagnosis,
        &[
            citation("[1992] HCA 23", "document:mabo#paragraph-1"),
            citation("[1988] HCA 69", "document:mabo#paragraph-42"),
        ],
    );

    assert!(!expanded.iter().any(|move_| {
        move_.source_ref.as_deref() == Some("[1992] HCA 23")
    }));
    let legal = expanded
        .iter()
        .filter(|move_| {
            move_.source_ref.as_deref() == Some("[1988] HCA 69")
                && move_.producer_lane == ProducerLane::GovernedLegal
        })
        .collect::<Vec<_>>();
    assert_eq!(legal.len(), 3);
    assert!(legal
        .iter()
        .all(|move_| move_.provider_operation_ref == "legal-follow:exact-citation"));
    assert!(legal
        .iter()
        .all(|move_| move_.diagnosis_reference.contains("document:mabo#paragraph-42")));
}
