use sensiblaw_evidence_payment::{
    evaluate_proposition_chain_payment, project_reader_payment, PropositionEvidenceObservation,
    PropositionProofRole, PropositionRoleResidual,
};
use sensiblaw_pg_source_store::{
    load_database_config, load_mabo_proposition_rows, PropositionObservationRow,
};
use sensiblaw_reader_model::{ReaderDisposition, ReaderIntent};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";
const SOURCE_REVISION: &str =
    "source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29";

fn observation(row: PropositionObservationRow) -> PropositionEvidenceObservation {
    PropositionEvidenceObservation {
        observation_ref: row.observation_ref,
        pnf_factor_ref: row.pnf_factor_ref,
        pnf_revision_ref: row.pnf_revision_ref,
        role: PropositionProofRole::Support,
        observation_provenance_refs: row.observation_provenance_refs,
        graph_source_span_refs: row.graph_source_span_refs,
        residual_refs: row.residual_refs,
    }
}

fn residual(role: PropositionProofRole, suffix: &str) -> PropositionRoleResidual {
    PropositionRoleResidual {
        role,
        residual_ref: format!("mabo:residual:{suffix}"),
    }
}

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_radical_title_rows_reach_reader_abi_without_truth_or_applicability() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let rows = load_mabo_proposition_rows(&config, PROPOSITION, SPAN)
        .expect("the materialised Mabo proposition coordinates must be readable");

    assert!(rows.exact_source_paid);
    assert_eq!(rows.proposition_ref, PROPOSITION);
    assert_eq!(rows.required_span_ref, SPAN);
    assert!(
        !rows.observations.is_empty(),
        "the live proposition needs at least one PNF observation welded to the exact span"
    );

    let observations = rows
        .observations
        .into_iter()
        .map(observation)
        .collect::<Vec<_>>();

    let residuals = vec![
        residual(PropositionProofRole::Qualifier, "qualifier-unpaid"),
        residual(PropositionProofRole::Defeater, "defeater-unpaid"),
        residual(PropositionProofRole::Comparator, "comparator-unpaid"),
    ];

    let chain = evaluate_proposition_chain_payment(
        &rows.proposition_ref,
        &rows.required_span_ref,
        rows.exact_source_paid,
        &observations,
        &residuals,
    );

    assert!(chain.proposition_chain_paid);
    assert!(chain.why_executable);
    assert!(!chain.applicability_paid);
    assert!(!chain.claim_truth_paid);

    let reader = project_reader_payment(&chain, SOURCE_REVISION, SPAN, &residuals)
        .expect("paid live proposition chain must project into the portable Reader ABI");
    assert!(!reader.applicability_paid());
    assert!(!reader.claim_truth_paid());
    assert!(matches!(
        reader.resolve(ReaderIntent::OpenSource),
        ReaderDisposition::ExecuteSource { .. }
    ));
    assert!(matches!(
        reader.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::ExecuteBoundedWhy(_)
    ));
}
