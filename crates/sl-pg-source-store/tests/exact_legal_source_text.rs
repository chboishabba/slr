use sensiblaw_pg_source_store::ExactLegalSourceText;

#[test]
fn exact_legal_source_text_is_runtime_evidence_not_semantic_payment() {
    let receipt = ExactLegalSourceText {
        source_revision_ref: "source-revision:mabo:1992:hca:23:wikisource:page-39:rev-16058297:2026-06-29".into(),
        document_ref: "document:mabo:1992:hca:23:brennan:wikisource-page-39".into(),
        canonical_text_sha256: "sha256:abc".into(),
        canonical_text: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    assert!(receipt.candidate_only);
    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.applicability_promoted);
    assert!(!receipt.claim_truth_promoted);
}
