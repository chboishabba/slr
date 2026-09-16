use std::env;

use sensiblaw_pg_source_store::{
    load_database_config, load_mabo_proposition_rows, materialize_reviewed_proposition_support,
    ReviewedPropositionSupport,
};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SOURCE_REVISION: &str =
    "source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29";
const DOCUMENT: &str = "document:mabo:1992:hca:23:brennan:wikisource-page-39";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

fn required(name: &'static str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} must identify the reviewed PNF coordinate"))
}

fn refs(name: &'static str) -> Vec<String> {
    required(name)
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn optional_refs(name: &'static str) -> Vec<String> {
    env::var(name)
        .ok()
        .into_iter()
        .flat_map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn main() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let provenance_refs = refs("MABO_PNF_PROVENANCE_REFS");
    if !provenance_refs.iter().any(|value| value == SPAN) {
        panic!("MABO_PNF_PROVENANCE_REFS must explicitly retain the exact Mabo span");
    }

    let support = ReviewedPropositionSupport {
        proposition_ref: PROPOSITION.to_owned(),
        source_revision_ref: SOURCE_REVISION.to_owned(),
        document_ref: DOCUMENT.to_owned(),
        exact_span_ref: SPAN.to_owned(),
        parser_build_ref: required("MABO_PARSER_BUILD_REF"),
        pnf_build_ref: required("MABO_PNF_BUILD_REF"),
        refined_pnf_graph_ref: required("MABO_PNF_GRAPH_REF"),
        pnf_factor_ref: required("MABO_PNF_FACTOR_REF"),
        pnf_revision_ref: required("MABO_PNF_REVISION_REF"),
        structural_signature_ref: required("MABO_STRUCTURAL_SIGNATURE_REF"),
        predicate_ref: required("MABO_PREDICATE_REF"),
        observation_provenance_refs: provenance_refs,
        observation_residual_refs: optional_refs("MABO_PNF_RESIDUAL_REFS"),
        legal_system_refs: optional_refs("MABO_LEGAL_SYSTEM_REFS"),
        jurisdiction_refs: optional_refs("MABO_JURISDICTION_REFS"),
        temporal_refs: optional_refs("MABO_TEMPORAL_REFS"),
        author_ref: required("MABO_REVIEW_AUTHOR_REF"),
        institution_ref: env::var("MABO_REVIEW_INSTITUTION_REF")
            .ok()
            .filter(|value| !value.trim().is_empty()),
    };

    let materialized = materialize_reviewed_proposition_support(&config, &support)
        .expect("reviewed Mabo proposition support must materialize fail-closed");
    let rows = load_mabo_proposition_rows(&config, PROPOSITION, SPAN)
        .expect("materialized Mabo proposition support must be readable");

    println!("semantic_build_ref={}", materialized.semantic_build_ref);
    println!("projection_ref={}", materialized.projection_ref);
    println!("observation_ref={}", materialized.observation_ref);
    println!("graph_revision_ref={}", materialized.graph_revision_ref);
    println!("exact_source_paid={}", rows.exact_source_paid);
    println!("observation_count={}", rows.observations.len());
}
