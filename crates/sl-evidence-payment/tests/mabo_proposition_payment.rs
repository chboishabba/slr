use sensiblaw_evidence_payment::{
    evaluate_proposition_chain_payment, PropositionEvidenceObservation, PropositionProofRole,
    PropositionRoleResidual,
};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

fn observation(
    reference: &str,
    role: PropositionProofRole,
    observation_provenance: &[&str],
    graph_source_spans: &[&str],
) -> PropositionEvidenceObservation {
    PropositionEvidenceObservation {
        observation_ref: reference.into(),
        pnf_factor_ref: format!("factor:{reference}"),
        pnf_revision_ref: format!("factor-revision:{reference}"),
        role,
        observation_provenance_refs: observation_provenance.iter().map(|v| (*v).into()).collect(),
        graph_source_span_refs: graph_source_spans.iter().map(|v| (*v).into()).collect(),
        residual_refs: Vec::new(),
    }
}

fn residual(role: PropositionProofRole, suffix: &str) -> PropositionRoleResidual {
    PropositionRoleResidual {
        role,
        residual_ref: format!("mabo:residual:{suffix}"),
    }
}

#[test]
fn exact_support_plus_explicit_role_residuals_pays_bounded_why_without_truth() {
    let observations = vec![observation(
        "support",
        PropositionProofRole::Support,
        &[SPAN],
        &[SPAN],
    )];
    let residuals = vec![
        residual(PropositionProofRole::Qualifier, "qualifier-unpaid"),
        residual(PropositionProofRole::Defeater, "defeater-unpaid"),
        residual(PropositionProofRole::Comparator, "comparator-unpaid"),
    ];

    let payment = evaluate_proposition_chain_payment(
        PROPOSITION,
        SPAN,
        true,
        &observations,
        &residuals,
    );

    assert!(payment.proposition_chain_paid);
    assert!(payment.why_executable);
    assert_eq!(payment.support_observation_refs, vec!["support"]);
    assert!(payment.qualifier_observation_refs.is_empty());
    assert_eq!(payment.role_residual_refs.len(), 3);
    assert!(!payment.applicability_paid);
    assert!(!payment.claim_truth_paid);
}

#[test]
fn pnf_support_without_independent_source_span_provenance_does_not_pay_chain() {
    let observations = vec![observation(
        "support",
        PropositionProofRole::Support,
        &[SPAN],
        &[],
    )];

    let payment = evaluate_proposition_chain_payment(
        PROPOSITION,
        SPAN,
        true,
        &observations,
        &[
            residual(PropositionProofRole::Qualifier, "qualifier-unpaid"),
            residual(PropositionProofRole::Defeater, "defeater-unpaid"),
            residual(PropositionProofRole::Comparator, "comparator-unpaid"),
        ],
    );

    assert!(!payment.proposition_chain_paid);
    assert!(!payment.why_executable);
    assert_eq!(payment.residual_refs, vec!["reader-residual:source-provenance-weld"]);
}

#[test]
fn exact_span_alone_does_not_pay_why() {
    let payment = evaluate_proposition_chain_payment(PROPOSITION, SPAN, true, &[], &[]);

    assert!(!payment.proposition_chain_paid);
    assert!(!payment.why_executable);
    assert!(payment.residual_refs.contains(&"reader-residual:proposition-support".into()));
    assert!(!payment.claim_truth_paid);
}

#[test]
fn paid_qualifier_can_replace_qualifier_residual_but_not_truth_or_applicability() {
    let observations = vec![
        observation(
            "support",
            PropositionProofRole::Support,
            &[SPAN],
            &[SPAN],
        ),
        observation(
            "qualifier",
            PropositionProofRole::Qualifier,
            &[SPAN],
            &[SPAN],
        ),
    ];

    let payment = evaluate_proposition_chain_payment(
        PROPOSITION,
        SPAN,
        true,
        &observations,
        &[
            residual(PropositionProofRole::Defeater, "defeater-unpaid"),
            residual(PropositionProofRole::Comparator, "comparator-unpaid"),
        ],
    );

    assert!(payment.proposition_chain_paid);
    assert_eq!(payment.qualifier_observation_refs, vec!["qualifier"]);
    assert!(!payment.applicability_paid);
    assert!(!payment.claim_truth_paid);
}
