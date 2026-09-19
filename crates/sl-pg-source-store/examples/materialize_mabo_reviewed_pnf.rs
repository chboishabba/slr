use std::env;

use sensiblaw_pg_source_store::{
    load_database_config, materialize_reviewed_pnf_revision, ReviewedPnfRevision,
};

const DOCUMENT: &str = "document:mabo:1992:hca:23:brennan:wikisource-page-39";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

fn required(name: &'static str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must be supplied by the explicit review/admission step"))
}

fn main() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let reviewed = ReviewedPnfRevision {
        review_receipt_ref: required("MABO_PNF_REVIEW_RECEIPT_REF"),
        document_ref: DOCUMENT.into(),
        exact_span_ref: SPAN.into(),
        graph_ref: required("MABO_PNF_GRAPH_REF"),
        graph_type_ref: env::var("MABO_PNF_GRAPH_TYPE_REF")
            .unwrap_or_else(|_| "generic.factor_graph".into()),
        schema_version_ref: env::var("MABO_PNF_SCHEMA_VERSION_REF")
            .unwrap_or_else(|_| "v0_1".into()),
        graph_closure_state_ref: env::var("MABO_PNF_GRAPH_CLOSURE_STATE_REF")
            .unwrap_or_else(|_| "reviewed_candidate".into()),
        factor_ref: required("MABO_PNF_FACTOR_REF"),
        factor_revision_ref: required("MABO_PNF_REVISION_REF"),
        factor_type_ref: required("MABO_PNF_FACTOR_TYPE_REF"),
        factor_closure_state_ref: env::var("MABO_PNF_FACTOR_CLOSURE_STATE_REF")
            .unwrap_or_else(|_| "reviewed_candidate".into()),
        graph_role_ref: env::var("MABO_PNF_GRAPH_ROLE_REF")
            .unwrap_or_else(|_| "support_candidate".into()),
    };

    let receipt = materialize_reviewed_pnf_revision(&config, &reviewed)
        .expect("explicitly reviewed Mabo PNF revision must persist fail-closed");

    println!("graph_ref={}", receipt.graph_ref);
    println!("factor_ref={}", receipt.factor_ref);
    println!("factor_revision_ref={}", receipt.factor_revision_ref);
    println!("review_receipt_ref={}", receipt.review_receipt_ref);
    println!("proposition_support_paid={}", receipt.proposition_support_paid);
    println!("applicability_paid={}", receipt.applicability_paid);
    println!("claim_truth_paid={}", receipt.claim_truth_paid);
}
