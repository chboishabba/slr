use sensiblaw_evidence_payment::{
    evaluate_persisted_proposition_payment,
    PersistedPropositionCoordinates,
    PersistedRoleCoordinate,
    PropositionPaymentState,
    PropositionRole,
};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SOURCE_REVISION: &str = "source-revision:mabo:1992:hca:23:wikisource:page-39:16058297";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

fn paid_support() -> PersistedRoleCoordinate {
    PersistedRoleCoordinate {
        observation_ref: "observation:mabo:radical-title:support-1".into(),
        role: PropositionRole::Support,
        pnf_supports_proposition: true,
        observation_source_span_ref: Some(SPAN.into()),
        graph_source_span_refs: vec![SPAN.into()],
        residual_ref: None,
    }
}

fn residual(role: PropositionRole, suffix: &str) -> PersistedRoleCoordinate {
    PersistedRoleCoordinate {
        observation_ref: format!("coordinate:mabo:{suffix}"),
        role,
        pnf_supports_proposition: false,
        observation_source_span_ref: None,
        graph_source_span_refs: vec![],
        residual_ref: Some(format!("residual:mabo:{suffix}")),
    }
}

#[test]
fn pg_backed_coordinates_pay_bounded_why_without_paying_applicability_or_truth() {
    let coordinates = PersistedPropositionCoordinates {
        proposition_ref: PROPOSITION.into(),
        source_revision_ref: SOURCE_REVISION.into(),
        required_span_ref: SPAN.into(),
        exact_source_paid: true,
        roles: vec![
            paid_support(),
            residual(PropositionRole::Qualifier, "qualifier"),
            residual(PropositionRole::Defeater, "defeater"),
            residual(PropositionRole::Comparator, "comparator"),
        ],
    };

    let payment = evaluate_persisted_proposition_payment(&coordinates);

    assert_eq!(payment.state, PropositionPaymentState::Paid);
    assert!(payment.support_paid);
    assert!(payment.qualifier_covered);
    assert!(payment.defeater_covered);
    assert!(payment.comparator_covered);
    assert!(!payment.applicability_paid);
    assert!(!payment.claim_truth_paid);
}

#[test]
fn persisted_pnf_support_without_independent_graph_span_weld_remains_unpaid() {
    let mut support = paid_support();
    support.graph_source_span_refs.clear();

    let coordinates = PersistedPropositionCoordinates {
        proposition_ref: PROPOSITION.into(),
        source_revision_ref: SOURCE_REVISION.into(),
        required_span_ref: SPAN.into(),
        exact_source_paid: true,
        roles: vec![
            support,
            residual(PropositionRole::Qualifier, "qualifier"),
            residual(PropositionRole::Defeater, "defeater"),
            residual(PropositionRole::Comparator, "comparator"),
        ],
    };

    let payment = evaluate_persisted_proposition_payment(&coordinates);

    assert_eq!(payment.state, PropositionPaymentState::Residual);
    assert!(!payment.support_paid);
    assert!(!payment.applicability_paid);
    assert!(!payment.claim_truth_paid);
}
