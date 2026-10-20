use sensiblaw_pg_source_store::{
    assess_declared_manifestation_coverage, AlignedComparison, SourceDomain, SourceInterval,
};
fn domain(side: &str) -> SourceDomain {
    SourceDomain {
        extraction_ref: format!("{side}-extraction"),
        output_sha256_hex: "a".repeat(64),
        parser_revision_ref: "parser:v1".into(),
        document_len: 10,
        spans: vec![
            SourceInterval { span_ref: format!("{side}-0"), start: 0, end: 5 },
            SourceInterval { span_ref: format!("{side}-1"), start: 5, end: 10 },
        ],
    }
}
fn comparison(n: usize) -> AlignedComparison {
    AlignedComparison {
        pdf_span_ref: format!("pdf-{n}"),
        docx_span_ref: format!("docx-{n}"),
        pdf_batch_ref: format!("pdf-batch-{n}"),
        docx_batch_ref: format!("docx-batch-{n}"),
        exhaustive_overlap_ref: format!("v3-{n}"),
        semantic_multiset_exact: true,
        other_mismatch: false,
        other_debris_review_ref: None,
    }
}
fn run(pdf: &SourceDomain, docx: &SourceDomain, comparisons: &[AlignedComparison])
    -> sensiblaw_pg_source_store::CoverageReceipt {
    assess_declared_manifestation_coverage(pdf, docx, comparisons, &[], &[])
}
#[test]
fn exact_declared_domain_replays() {
    let p = domain("pdf");
    let d = domain("docx");
    let comparisons = vec![comparison(0), comparison(1)];
    let a = run(&p, &d, &comparisons);
    assert!(a.complete_declared_domain);
    assert!(a.transformation_residual_closed);
    assert!(a.residuals.is_empty());
    assert_eq!(a, run(&p, &d, &comparisons));
}
#[test]
fn missing_aligned_span_fails_closed() {
    let result = run(&domain("pdf"), &domain("docx"), &[comparison(0)]);
    assert!(!result.complete_declared_domain);
    assert!(result.residuals.iter().any(|s| s.contains("unaccounted_span")));
}
#[test]
fn omitted_source_partition_fails_closed() {
    let mut p = domain("pdf");
    p.spans.remove(1);
    let result = run(&p, &domain("docx"), &[comparison(0), comparison(1)]);
    assert!(!result.complete_declared_domain);
    assert!(result.residuals.iter().any(|s| s.contains("incomplete_partition")));
}
#[test]
fn changed_extraction_digest_changes_receipt_identity() {
    let p = domain("pdf");
    let d = domain("docx");
    let comparisons = vec![comparison(0), comparison(1)];
    let first = run(&p, &d, &comparisons);
    let mut altered = p.clone();
    altered.output_sha256_hex = "b".repeat(64);
    assert_ne!(first.receipt_ref, run(&altered, &d, &comparisons).receipt_ref);
}
#[test]
fn unmatched_semantic_factor_forces_review() {
    let mut comparisons = vec![comparison(0), comparison(1)];
    comparisons[1].semantic_multiset_exact = false;
    let result = run(&domain("pdf"), &domain("docx"), &comparisons);
    assert!(!result.transformation_residual_closed);
    assert!(result.residuals.iter().any(|s| s.contains("unmatched_semantic")));
}
#[test]
fn other_mismatch_without_debris_review_forces_review() {
    let mut comparisons = vec![comparison(0), comparison(1)];
    comparisons[1].other_mismatch = true;
    let result = run(&domain("pdf"), &domain("docx"), &comparisons);
    assert!(!result.transformation_residual_closed);
    assert!(result.residuals.iter().any(|s| s.contains("unreviewed_other")));
}
#[test]
fn duplicate_comparison_fails_closed() {
    let result = run(&domain("pdf"), &domain("docx"), &[comparison(0),comparison(0),comparison(1)]);
    assert!(!result.complete_declared_domain);
}
