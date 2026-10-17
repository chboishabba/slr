use sensiblaw_pg_source_store::{
    compare_and_persist_official_primary_manifestation_pnf_exhaustive_overlap,
    load_database_config,
};

fn main() -> Result<(), String> {
    let overlap_ref = std::env::args()
        .nth(1)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            "usage: compare_hca_official_manifestation_pnf_exhaustive <overlap_ref>".to_owned()
        })?;
    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let receipt = compare_and_persist_official_primary_manifestation_pnf_exhaustive_overlap(
        &config,
        &overlap_ref,
    )
    .map_err(|error| error.to_string())?;
    let c = &receipt.coverage;

    println!("exhaustive_overlap_ref={}", receipt.exhaustive_overlap_ref);
    println!("base_overlap_ref={}", receipt.overlap_ref);
    println!("comparator_revision_ref={}", receipt.comparator_revision_ref);
    println!("comparison_projection={}", c.projection_ref);
    println!("pdf_parser_factor_count={}", c.pdf_parser_factor_count);
    println!("docx_parser_factor_count={}", c.docx_parser_factor_count);
    println!("pdf_semantic_factor_count={}", c.pdf_semantic_factor_count);
    println!("docx_semantic_factor_count={}", c.docx_semantic_factor_count);
    println!("matched_semantic_factor_count={}", c.matched_semantic_factor_count);
    println!("unmatched_pdf_semantic_count={}", c.unmatched_pdf_semantic_count);
    println!("unmatched_docx_semantic_count={}", c.unmatched_docx_semantic_count);
    println!("ignored_pdf_nonsemantic_count={}", c.ignored_pdf_nonsemantic_count);
    println!("ignored_docx_nonsemantic_count={}", c.ignored_docx_nonsemantic_count);
    println!("semantic_factor_multiset_exact={}", c.semantic_factor_multiset_exact);
    println!(
        "closes_source_transformation_residual={}",
        c.closes_source_transformation_residual
    );
    println!("requires_human_pnf_review={}", c.requires_human_pnf_review);
    println!("independent_source_count_increment=0");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
