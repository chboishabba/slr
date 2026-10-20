//! Source-bound *declared-domain* coverage gate. This is not a whole-judgment
//! certificate until the declared partition is independently verified complete.
//! A selected-fibre v3 receipt is never sufficient by itself.
use std::collections::BTreeSet;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDomain {
    pub extraction_ref: String,
    pub output_sha256_hex: String,
    pub parser_revision_ref: String,
    pub document_len: usize,
    pub spans: Vec<SourceInterval>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInterval {
    pub span_ref: String,
    pub start: usize,
    pub end: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignedComparison {
    pub pdf_span_ref: String,
    pub docx_span_ref: String,
    pub pdf_batch_ref: String,
    pub docx_batch_ref: String,
    pub exhaustive_overlap_ref: String,
    pub semantic_multiset_exact: bool,
    pub other_mismatch: bool,
    pub other_debris_review_ref: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedExclusion {
    pub span_ref: String,
    pub review_ref: String,
    pub policy_ref: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageReceipt {
    pub receipt_ref: String,
    pub complete_declared_domain: bool,
    pub transformation_residual_closed: bool,
    pub residuals: Vec<String>,
    pub creates_legal_controversy: bool,
    pub creates_semantic_authority: bool,
    pub independent_source_count_increment: i32,
}
fn add(h: &mut Sha256, value: &str) {
    h.update((value.len() as u64).to_be_bytes());
    h.update(value.as_bytes());
}
fn validate_domain(domain: &SourceDomain, side: &str, residuals: &mut Vec<String>) {
    if domain.extraction_ref.is_empty() || domain.parser_revision_ref.is_empty()
        || domain.output_sha256_hex.len() != 64
        || !domain.output_sha256_hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        residuals.push(format!("{side}:unverified_source_coordinates"));
    }
    let mut cursor = 0;
    let mut seen = BTreeSet::new();
    let mut ordered = domain.spans.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|span| (span.start, span.end));
    for span in ordered {
        if span.span_ref.is_empty() || !seen.insert(&span.span_ref) {
            residuals.push(format!("{side}:duplicate_span"));
        }
        if span.start != cursor || span.end <= span.start || span.end > domain.document_len {
            residuals.push(format!("{side}:partition_gap_or_overlap:{}", span.span_ref));
        }
        cursor = span.end;
    }
    if cursor != domain.document_len || domain.spans.is_empty() {
        residuals.push(format!("{side}:incomplete_partition"));
    }
}
/// Pure deterministic receipt: all declared source intervals must be accounted
/// for, and any unmatched/non-debris Other is an explicit review residual.
/// The caller must separately verify that the domain length/digest/partition
/// came from the official persisted extraction, not caller assertion.
pub fn assess_declared_manifestation_coverage(
    pdf: &SourceDomain, docx: &SourceDomain,
    comparisons: &[AlignedComparison],
    pdf_exclusions: &[ReviewedExclusion],
    docx_exclusions: &[ReviewedExclusion],
) -> CoverageReceipt {
    let mut residuals = Vec::new();
    validate_domain(pdf, "pdf", &mut residuals);
    validate_domain(docx, "docx", &mut residuals);
    let pdf_ids = pdf.spans.iter().map(|s| s.span_ref.as_str()).collect::<BTreeSet<_>>();
    let docx_ids = docx.spans.iter().map(|s| s.span_ref.as_str()).collect::<BTreeSet<_>>();
    let mut used_pdf = BTreeSet::new();
    let mut used_docx = BTreeSet::new();
    let mut exact = true;
    for c in comparisons {
        if !pdf_ids.contains(c.pdf_span_ref.as_str()) || !docx_ids.contains(c.docx_span_ref.as_str())
            || !used_pdf.insert(c.pdf_span_ref.as_str()) || !used_docx.insert(c.docx_span_ref.as_str()) {
            residuals.push("invalid_or_duplicate_alignment".into());
        }
        if c.pdf_batch_ref.is_empty() || c.docx_batch_ref.is_empty() || c.exhaustive_overlap_ref.is_empty() {
            residuals.push("missing_candidate_or_comparison_receipt".into());
        }
        if !c.semantic_multiset_exact {
            exact = false;
            residuals.push(format!("unmatched_semantic:{}", c.pdf_span_ref));
        }
        if c.other_mismatch && c.other_debris_review_ref.as_deref().unwrap_or("").is_empty() {
            exact = false;
            residuals.push(format!("unreviewed_other:{}", c.pdf_span_ref));
        }
    }
    for (side, ids, used, exclusions) in [
        ("pdf", &pdf_ids, &mut used_pdf, pdf_exclusions),
        ("docx", &docx_ids, &mut used_docx, docx_exclusions),
    ] {
        for e in exclusions {
            if !ids.contains(e.span_ref.as_str()) || !used.insert(e.span_ref.as_str())
                || e.review_ref.is_empty() || e.policy_ref.is_empty() {
                residuals.push(format!("{side}:invalid_exclusion:{}", e.span_ref));
            }
        }
        for id in ids {
            if !used.contains(id) {
                residuals.push(format!("{side}:unaccounted_span:{id}"));
            }
        }
    }
    residuals.sort();
    let complete = residuals.is_empty();
    let mut h = Sha256::new();
    add(&mut h, "declared-manifestation-coverage:v1");
    for domain in [pdf, docx] {
        for value in [&domain.extraction_ref, &domain.output_sha256_hex, &domain.parser_revision_ref] {
            add(&mut h, value);
        }
        add(&mut h, &domain.document_len.to_string());
        let mut spans = domain.spans.clone();
        spans.sort_by(|a,b| a.span_ref.cmp(&b.span_ref));
        for s in spans { add(&mut h, &s.span_ref); add(&mut h, &s.start.to_string()); add(&mut h, &s.end.to_string()); }
    }
    let mut comps = comparisons.to_vec();
    comps.sort_by(|a,b| (&a.pdf_span_ref, &a.docx_span_ref).cmp(&(&b.pdf_span_ref, &b.docx_span_ref)));
    for c in comps {
        for v in [&c.pdf_span_ref,&c.docx_span_ref,&c.pdf_batch_ref,&c.docx_batch_ref,&c.exhaustive_overlap_ref] { add(&mut h,v); }
        add(&mut h, &c.semantic_multiset_exact.to_string());
        add(&mut h, &c.other_mismatch.to_string());
        add(&mut h, c.other_debris_review_ref.as_deref().unwrap_or(""));
    }
    for exclusions in [pdf_exclusions, docx_exclusions] {
        let mut sorted = exclusions.to_vec();
        sorted.sort_by(|a,b| a.span_ref.cmp(&b.span_ref));
        for e in sorted { for v in [&e.span_ref,&e.review_ref,&e.policy_ref] { add(&mut h,v); } }
    }
    for residual in &residuals { add(&mut h,residual); }
    CoverageReceipt {
        receipt_ref: format!("declared-manifestation-coverage:sha256:{:x}",h.finalize()),
        complete_declared_domain: complete,
        transformation_residual_closed: complete && exact,
        residuals,
        creates_legal_controversy: false,
        creates_semantic_authority: false,
        independent_source_count_increment: 0,
    }
}
