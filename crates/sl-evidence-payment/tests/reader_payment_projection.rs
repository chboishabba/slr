use sensiblaw_evidence_payment::{
    project_reader_payment, PropositionChainPayment, PropositionProofRole, PropositionRoleResidual,
};
use sensiblaw_reader_model::{ReaderDisposition, ReaderIntent};

const PROP: &str = "mabo:proposition:radical-title-native-title";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";
const REV: &str = "source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29";

fn paid_chain() -> PropositionChainPayment {
    PropositionChainPayment {
        proposition_ref: PROP.into(),
        exact_source_paid: true,
        support_observation_refs: vec!["observation:mabo:support".into()],
        qualifier_observation_refs: vec![],
        defeater_observation_refs: vec![],
        comparator_observation_refs: vec![],
        role_residual_refs: vec![
            "residual:mabo:qualifier".into(),
            "residual:mabo:defeater".into(),
            "residual:mabo:comparator".into(),
        ],
        residual_refs: vec![],
        proposition_chain_paid: true,
        why_executable: true,
        applicability_paid: false,
        claim_truth_paid: false,
    }
}

fn role_residuals() -> Vec<PropositionRoleResidual> {
    vec![
        PropositionRoleResidual {
            role: PropositionProofRole::Qualifier,
            residual_ref: "residual:mabo:qualifier".into(),
        },
        PropositionRoleResidual {
            role: PropositionProofRole::Defeater,
            residual_ref: "residual:mabo:defeater".into(),
        },
        PropositionRoleResidual {
            role: PropositionProofRole::Comparator,
            residual_ref: "residual:mabo:comparator".into(),
        },
    ]
}

#[test]
fn paid_chain_projects_to_bounded_reader_without_truth_or_applicability() {
    let payment = project_reader_payment(&paid_chain(), REV, SPAN, &role_residuals()).unwrap();
    assert!(payment.proposition_chain_paid());
    assert!(!payment.applicability_paid());
    assert!(!payment.claim_truth_paid());
    match payment.resolve(ReaderIntent::WhyClaim) {
        ReaderDisposition::ExecuteBoundedWhy(cone) => {
            assert_eq!(cone.proposition_ref.as_str(), PROP);
            assert!(!cone.applicability_paid());
            assert!(!cone.claim_truth_paid());
        }
        other => panic!("expected bounded Why, got {other:?}"),
    }
}

#[test]
fn exact_source_without_paid_chain_still_projects_source_only() {
    let mut chain = paid_chain();
    chain.proposition_chain_paid = false;
    chain.why_executable = false;
    chain.support_observation_refs.clear();
    chain.residual_refs = vec!["reader-residual:proposition-support".into()];

    let payment = project_reader_payment(&chain, REV, SPAN, &role_residuals()).unwrap();
    assert!(!payment.proposition_chain_paid());
    assert!(matches!(
        payment.resolve(ReaderIntent::OpenSource),
        ReaderDisposition::ExecuteSource { .. }
    ));
    assert!(matches!(
        payment.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::Defer(_)
    ));
}
