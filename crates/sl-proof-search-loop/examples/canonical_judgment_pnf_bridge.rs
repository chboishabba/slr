use sensiblaw_proof_search_loop::judgment_pnf::{
    compile_canonical_judgment_text_to_pnf_bridge, CanonicalJudgmentTextReceipt,
};

fn main() {
    let text = CanonicalJudgmentTextReceipt {
        document_ref: "document:hca:[2026]-HCA-19:canonical-text".into(),
        canonical_text_sha256: "sha256:cullen-canonical-text-fixture".into(),
        paragraph_count: 321,
        source_revision_ref: "source-revision:sha256:cullen-docx-fixture".into(),
        receipt_authority: "experimental_candidate_only",
    };
    let bridge = compile_canonical_judgment_text_to_pnf_bridge(
        "run:hca-cullen-pnf:fixture",
        &text,
        "graph:cullen:source-grounded",
        vec!["residual:current-treatment".into()],
    )
    .expect("compile canonical HCA judgment text to existing PNF bridge");

    assert_eq!(bridge.document_ref, text.document_ref);
    assert_eq!(bridge.canonical_text_sha256, text.canonical_text_sha256);
    assert!(bridge.world_resolution_deferred);
    assert!(!bridge.cross_document_identity_closed);
    assert!(!bridge.parser_observation_is_semantic_authority);
    assert!(bridge.semantic_correspondence_required);

    println!(
        "canonical_judgment_pnf_bridge=PASS document={} graph={} residuals={} semantic_authority=false correspondence_required=true",
        bridge.document_ref,
        bridge.graph_ref,
        bridge.residual_demand_refs.len()
    );
}
