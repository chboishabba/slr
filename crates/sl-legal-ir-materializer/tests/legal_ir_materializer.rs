use sensiblaw_legal_ir_materializer::{
    decode_record, encode_record, exact_source_weld_sql, legal_ir_schema_sql, GraphRevision,
    LegalIrRecord, Observation, Projection, SemanticBuild, SourceWeldState, SLRI_MAGIC,
    SLRI_VERSION,
};
use std::io::Cursor;

fn sample_records() -> Vec<LegalIrRecord> {
    vec![
        LegalIrRecord::SemanticBuild(SemanticBuild {
            build_ref: "legal-build:mabo:radical-title:v1".into(),
            document_ref: "document:mabo:1992:hca:23:brennan:wikisource-page-39".into(),
            source_revision_ref: "source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29".into(),
            canonical_text_ref: "canonical:mabo:page-39".into(),
            parser_build_ref: "parser:spacy:trained".into(),
            pnf_build_ref: "pnf-build:mabo:page-39".into(),
            refined_pnf_graph_ref: "pnf-graph:mabo:page-39".into(),
            legal_ir_projection_ref: "legal-projection:mabo:radical-title:v1".into(),
            build_state_ref: "candidate".into(),
            provenance_refs: vec!["manifestation:mabo:1992:hca:23:wikisource:page-39".into()],
        }),
        LegalIrRecord::Projection(Projection {
            projection_ref: "legal-projection:mabo:radical-title:v1".into(),
            build_ref: "legal-build:mabo:radical-title:v1".into(),
            pnf_build_ref: "pnf-build:mabo:page-39".into(),
            projection_contract_ref: "contract:sensiblaw:legal-ir:v2".into(),
            omitted_factor_refs: vec![],
            projection_residuals: vec!["residual:applicability".into()],
        }),
        LegalIrRecord::Observation(Observation {
            observation_ref: "legal-observation:mabo:radical-title:v1".into(),
            projection_ref: "legal-projection:mabo:radical-title:v1".into(),
            pnf_factor_ref: "pnf-factor:mabo:radical-title".into(),
            pnf_revision_ref: "pnf-factor-revision:mabo:radical-title:v1".into(),
            structural_signature_ref: "signature:radical-title-native-title".into(),
            predicate_ref: "predicate:does-not-automatically-confer-beneficial-ownership".into(),
            observation_body: b"OBS1\0fixed-binary-role-qualifier-wrapper-body".to_vec(),
            provenance_refs: vec!["span:mabo:brennan:radical-title:no-automatic-beneficial-ownership".into()],
            residual_refs: vec!["residual:applicability".into()],
            projection_state_ref: "candidate".into(),
        }),
        LegalIrRecord::GraphRevision(GraphRevision {
            revision_ref: "legal-graph-revision:mabo:radical-title:v1".into(),
            subject_ref: "mabo:radical-title-native-title".into(),
            prior_revision_refs: vec![],
            source_span_refs: vec!["span:mabo:brennan:radical-title:no-automatic-beneficial-ownership".into()],
            legal_system_refs: vec!["legal-system:australia:commonwealth".into()],
            jurisdiction_refs: vec!["jurisdiction:australia".into()],
            temporal_refs: vec!["date:1992-06-03".into()],
            author_ref: "person:justice-brennan".into(),
            institution_ref: Some("institution:high-court-of-australia".into()),
            build_ref: "legal-build:mabo:radical-title:v1".into(),
            revision_state_ref: "candidate".into(),
        }),
    ]
}

#[test]
fn slri_v1_round_trip_preserves_all_four_materialisation_kinds() {
    for record in sample_records() {
        let mut bytes = Vec::new();
        encode_record(&mut bytes, &record).expect("encode");
        assert_eq!(&bytes[..4], &SLRI_MAGIC);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), SLRI_VERSION);
        let decoded = decode_record(&mut Cursor::new(bytes))
            .expect("decode")
            .expect("record");
        assert_eq!(decoded, record);
    }
}

#[test]
fn observation_keeps_historical_map_state_as_fixed_binary_body_not_json() {
    let observation = match &sample_records()[2] {
        LegalIrRecord::Observation(value) => value.clone(),
        _ => unreachable!(),
    };
    assert!(observation.observation_body.starts_with(b"OBS1"));
    let body = String::from_utf8_lossy(&observation.observation_body).to_ascii_uppercase();
    assert!(!body.contains("JSON"));
}

#[test]
fn legal_ir_v2_schema_is_append_only_and_json_free() {
    let sql = legal_ir_schema_sql();
    for table in [
        "legal_ir.semantic_build_v2",
        "legal_ir.projection_v2",
        "legal_ir.observation_v2",
        "legal_ir.graph_revision_v2",
    ] {
        assert!(sql.contains(table), "missing {table}");
    }
    let upper = sql.to_ascii_uppercase();
    assert!(upper.contains("BYTEA"));
    assert!(!upper.contains("JSON"));
    assert!(!upper.contains("UPDATE "));
    assert!(!upper.contains("DELETE "));
}

#[test]
fn exact_source_weld_requires_same_build_source_revision_and_span() {
    let sql = exact_source_weld_sql();
    assert!(sql.contains("graph_revision_v2"));
    assert!(sql.contains("semantic_build_v2"));
    assert!(sql.contains("g.build_ref = b.build_ref"));
    assert!(sql.contains("b.source_revision_ref = $2"));
    assert!(sql.contains("$3 = ANY(g.source_span_refs)"));
}

#[test]
fn provenance_payment_does_not_pay_truth_or_applicability() {
    let state = SourceWeldState::from_exact_source_match(true);
    assert!(state.exact_source_paid);
    assert!(!state.proposition_truth_paid);
    assert!(!state.applicability_paid);

    let missing = SourceWeldState::from_exact_source_match(false);
    assert!(!missing.exact_source_paid);
    assert!(!missing.proposition_truth_paid);
    assert!(!missing.applicability_paid);
}

#[test]
fn materializer_dependencies_have_no_json_or_regex_contract() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("materializer Cargo.toml");
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}

#[test]
fn mabo_runner_consumes_admitted_slri_without_python_legal_semantics() {
    let script = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/run_slr_mabo_proof_graph.sh"),
    )
    .expect("Mabo runner");
    assert!(script.contains("sensiblaw-legal-ir-materializer"));
    assert!(script.contains("mabo:radical-title-native-title"));
    assert!(script.contains("rev-16058297:2026-06-29"));
    assert!(script.contains("proposition_truth_paid=false"));
    assert!(script.contains("applicability_paid=false"));
    assert!(!script.contains("python3"));
    assert!(!script.contains("serde_json"));
    assert!(!script.contains(".json"));
}
