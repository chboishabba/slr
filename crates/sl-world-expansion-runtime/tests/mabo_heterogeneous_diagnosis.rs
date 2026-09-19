use sensiblaw_pg_source_store::{PropositionObservationRow, PropositionRows};
use sensiblaw_proof_search_loop::world_expansion::{ProducerLane, ResidualClass};
use sensiblaw_world_expansion_runtime::mabo_heterogeneous_diagnosis::{
    diagnose_mabo_proposition_research, MABO_RADICAL_TITLE_PROPOSITION,
    MABO_RADICAL_TITLE_SPAN,
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
