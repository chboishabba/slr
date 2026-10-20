//! Candidate-only document-pair coverage, never a whole-judgment certificate
//! without independently reopened official extraction and comparison records.
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
pub struct AlignmentGroup {
    pub group_ref: String,
    pub pdf_span_refs: Vec<String>,
    pub docx_span_refs: Vec<String>,
    pub pdf_candidate_batch_refs: Vec<String>,
    pub docx_candidate_batch_refs: Vec<String>,
    pub comparator_revision_ref: String,
    pub comparison_receipt_refs: Vec<String>,
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
fn domain_ids(domain: &SourceDomain, side: &str, residuals: &mut Vec<String>) -> BTreeSet<String> {
    if domain.extraction_ref.is_empty() || domain.parser_revision_ref.is_empty()
        || domain.output_sha256_hex.len() != 64
        || !domain.output_sha256_hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        residuals.push(format!("{side}:invalid_extraction_identity"));
    }
    let mut ordered = domain.spans.clone();
    ordered.sort_by_key(|s| (s.start,s.end));
    let mut cursor = 0;
    let mut ids = BTreeSet::new();
    for s in &ordered {
        if s.span_ref.is_empty() || !ids.insert(s.span_ref.clone()) {
            residuals.push(format!("{side}:duplicate_span:{}",s.span_ref));
        }
        if s.start != cursor || s.end <= s.start || s.end > domain.document_len {
            residuals.push(format!("{side}:partition_gap_or_overlap:{}",s.span_ref));
        }
        cursor = s.end;
    }
    if ordered.is_empty() || cursor != domain.document_len {
        residuals.push(format!("{side}:incomplete_partition"));
    }
    ids
}
/// Aggregate over a *document pair*, allowing arbitrary segmentation per side.
/// Every declared source interval must be accounted for exactly once. A
/// successful pure assessment does not prove the declared extraction is official.
pub fn assess_document_pair_manifestation_coverage(
    pdf: &SourceDomain, docx: &SourceDomain,
    groups: &[AlignmentGroup],
    pdf_exclusions: &[ReviewedExclusion],
    docx_exclusions: &[ReviewedExclusion],
) -> CoverageReceipt {
    let mut residuals = Vec::new();
    let pdf_ids = domain_ids(pdf,"pdf",&mut residuals);
    let docx_ids = domain_ids(docx,"docx",&mut residuals);
    let mut used_pdf = BTreeSet::new();
    let mut used_docx = BTreeSet::new();
    let mut group_ids = BTreeSet::new();
    let mut semantic_exact = true;
    for g in groups {
        if g.group_ref.is_empty() || !group_ids.insert(g.group_ref.clone()) {
            residuals.push(format!("duplicate_or_empty_group:{}",g.group_ref));
        }
        if g.pdf_span_refs.is_empty() || g.docx_span_refs.is_empty() {
            residuals.push(format!("empty_alignment_side:{}",g.group_ref));
        }
        for (side, members, ids, used) in [
            ("pdf",&g.pdf_span_refs,&pdf_ids,&mut used_pdf),
            ("docx",&g.docx_span_refs,&docx_ids,&mut used_docx),
        ] {
            for member in members {
                if !ids.contains(member) || !used.insert(member.clone()) {
                    residuals.push(format!("{side}:invalid_or_duplicate_membership:{member}"));
                }
            }
        }
        if g.pdf_candidate_batch_refs.is_empty() || g.docx_candidate_batch_refs.is_empty()
            || g.comparator_revision_ref.is_empty() || g.comparison_receipt_refs.is_empty()
            || g.pdf_candidate_batch_refs.iter().any(String::is_empty)
            || g.docx_candidate_batch_refs.iter().any(String::is_empty)
            || g.comparison_receipt_refs.iter().any(String::is_empty) {
            residuals.push(format!("missing_group_comparison_ancestry:{}",g.group_ref));
        }
        if !g.semantic_multiset_exact {
            semantic_exact = false;
            residuals.push(format!("unmatched_semantic_group:{}",g.group_ref));
        }
        if g.other_mismatch && g.other_debris_review_ref.as_deref().unwrap_or("").is_empty() {
            semantic_exact = false;
            residuals.push(format!("unreviewed_other_group:{}",g.group_ref));
        }
    }
    for (side, ids, used, exclusions) in [
        ("pdf",&pdf_ids,&mut used_pdf,pdf_exclusions),
        ("docx",&docx_ids,&mut used_docx,docx_exclusions),
    ] {
        for e in exclusions {
            if !ids.contains(&e.span_ref) || !used.insert(e.span_ref.clone())
                || e.review_ref.is_empty() || e.policy_ref.is_empty() {
                residuals.push(format!("{side}:invalid_exclusion:{}",e.span_ref));
            }
        }
        for id in ids {
            if !used.contains(&id) {
                residuals.push(format!("{side}:unaccounted_span:{id}"));
            }
        }
    }
    if groups.is_empty() { residuals.push("no_comparison_groups".into()); }
    residuals.sort();
    let complete = residuals.is_empty();
    let mut h = Sha256::new();
    add(&mut h,"document-pair-declared-coverage:v2");
    for d in [pdf,docx] {
        for v in [&d.extraction_ref,&d.output_sha256_hex,&d.parser_revision_ref] { add(&mut h,v); }
        add(&mut h,&d.document_len.to_string());
        let mut spans = d.spans.clone();
        spans.sort_by(|a,b| a.span_ref.cmp(&b.span_ref));
        for s in spans { add(&mut h,&s.span_ref); add(&mut h,&s.start.to_string()); add(&mut h,&s.end.to_string()); }
    }
    let mut ordered_groups = groups.to_vec();
    ordered_groups.sort_by(|a,b| a.group_ref.cmp(&b.group_ref));
    for g in ordered_groups {
        add(&mut h,&g.group_ref);
        for mut vs in [g.pdf_span_refs,g.docx_span_refs,g.pdf_candidate_batch_refs,
            g.docx_candidate_batch_refs,g.comparison_receipt_refs] {
            vs.sort();
            add(&mut h,&vs.len().to_string());
            for v in vs { add(&mut h,&v); }
        }
        add(&mut h,&g.comparator_revision_ref);
        add(&mut h,&g.semantic_multiset_exact.to_string());
        add(&mut h,&g.other_mismatch.to_string());
        add(&mut h,g.other_debris_review_ref.as_deref().unwrap_or(""));
    }
    for exclusions in [pdf_exclusions,docx_exclusions] {
        let mut sorted = exclusions.to_vec();
        sorted.sort_by(|a,b| a.span_ref.cmp(&b.span_ref));
        for e in sorted { for v in [&e.span_ref,&e.review_ref,&e.policy_ref] { add(&mut h,v); } }
    }
    for r in &residuals { add(&mut h,r); }
    CoverageReceipt {
        receipt_ref:format!("document-pair-declared-coverage:sha256:{:x}",h.finalize()),
        complete_declared_domain:complete,
        transformation_residual_closed:complete && semantic_exact,
        residuals,
        creates_legal_controversy:false,
        creates_semantic_authority:false,
        independent_source_count_increment:0,
    }
}
/// Compatibility entrypoint: existing single-span v3 witnesses are projected
/// to singleton groups. It does not assert whole-document coverage.
pub fn assess_declared_manifestation_coverage(
    pdf: &SourceDomain, docx: &SourceDomain,
    comparisons: &[AlignedComparison],
    pdf_exclusions: &[ReviewedExclusion],
    docx_exclusions: &[ReviewedExclusion],
) -> CoverageReceipt {
    let groups = comparisons.iter().enumerate().map(|(i,c)| AlignmentGroup {
        group_ref:format!("legacy-singleton-{i}"),
        pdf_span_refs:vec![c.pdf_span_ref.clone()],
        docx_span_refs:vec![c.docx_span_ref.clone()],
        pdf_candidate_batch_refs:vec![c.pdf_batch_ref.clone()],
        docx_candidate_batch_refs:vec![c.docx_batch_ref.clone()],
        comparator_revision_ref:"official-primary-manifestation-predicate-pnf-exhaustive-overlap:v3".into(),
        comparison_receipt_refs:vec![c.exhaustive_overlap_ref.clone()],
        semantic_multiset_exact:c.semantic_multiset_exact,
        other_mismatch:c.other_mismatch,
        other_debris_review_ref:c.other_debris_review_ref.clone(),
    }).collect::<Vec<_>>();
    assess_document_pair_manifestation_coverage(pdf,docx,&groups,pdf_exclusions,docx_exclusions)
}
