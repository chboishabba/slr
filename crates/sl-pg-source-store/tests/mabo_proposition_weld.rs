use sensiblaw_evidence_payment::{
    evaluate_proposition_chain_payment, PropositionEvidenceObservation, PropositionProofRole,
    PropositionRoleResidual,
};
use sensiblaw_pg_source_store::{
    load_database_config, load_mabo_proposition_rows, PropositionObservationRow,
    PropositionResidualRow,
};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

fn role(value: &str) -> PropositionProofRole {
    match value {
        "support" => PropositionProofRole::Support,
        "qualifier" => PropositionProofRole::Qualifier,
        "defeater" => PropositionProofRole::Defeater,
        "comparator" => PropositionProofRole::Comparator,
        other => panic!("unexpected proposition proof role: {other}"),
    }
}

fn observation(row: PropositionObservationRow) -> PropositionEvidenceObservation {
    PropositionEvidenceObservation {
        observation_ref: row.observation_ref,
        pnf_factor_ref: row.pnf_factor_ref,
        pnf_revision_ref: row.pnf_revision_ref,
        role: role(&row.role_ref),
        observation_provenance_refs: row.observation_provenance_refs,
        graph_source_span_refs: row.graph_source_span_refs,
        residual_refs: row.residual_refs,
    }
}

fn residual(row: PropositionResidualRow) -> PropositionRoleResidual {
    PropositionRoleResidual {
        role: role(&row.role_ref),
        residual_ref: row.residual_ref,
    }
}

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_radical_title_rows_pay_bounded_why_without_truth_or_applicability() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let rows = load_mabo_proposition_rows(&config, PROPOSITION, SPAN)
        .expect("the materialised Mabo proposition coordinates must be readable");

    assert!(rows.exact_source_paid);
    assert_eq!(rows.proposition_ref, PROPOSITION);
    assert_eq!(rows.required_span_ref, SPAN);

    let observations = rows
        .observations
        .into_iter()
        .map(observation)
        .collect::<Vec<_>>();
    let residuals = rows
        .residuals
        .into_iter()
        .map(residual)
        .collect::<Vec<_>>();

    let payment = evaluate_proposition_chain_payment(
        &rows.proposition_ref,
        &rows.required_span_ref,
        rows.exact_source_paid,
        &observations,
        &residuals,
    );

    assert!(payment.proposition_chain_paid);
    assert!(payment.why_executable);
    assert!(!payment.applicability_paid);
    assert!(!payment.claim_truth_paid);
}
