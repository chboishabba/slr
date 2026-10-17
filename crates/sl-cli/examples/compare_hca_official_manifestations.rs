//! Compare official HCA PDF/DOCX extracted texts as transformation evidence.
//!
//! Formatting-only differences do not reopen legal review. Material divergence
//! creates a source-transformation residual requiring PNF/review; it does not
//! create a legal controversy or a second authority.

use std::{env, fs, process};

use sensiblaw_pg_source_store::{
    compare_and_persist_official_primary_manifestations, load_database_config,
    ManifestationComparisonDisposition,
};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: compare_hca_official_manifestations <pdf-extraction-ref> <docx-extraction-ref> <pdf-text-path> <docx-text-path>"
                .into(),
        );
    }
    let pdf_text = fs::read_to_string(&args[2]).map_err(|error| error.to_string())?;
    let docx_text = fs::read_to_string(&args[3]).map_err(|error| error.to_string())?;
    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let receipt = compare_and_persist_official_primary_manifestations(
        &config,
        &args[0],
        &args[1],
        &pdf_text,
        &docx_text,
    )
    .map_err(|error| error.to_string())?;

    let disposition = match receipt.disposition {
        ManifestationComparisonDisposition::ExactBytes => "exact_bytes",
        ManifestationComparisonDisposition::FormattingOnly => "formatting_only",
        ManifestationComparisonDisposition::RequiresPnfReview => "requires_pnf_review",
    };
    println!("comparison_ref={}", receipt.comparison_ref);
    println!("citation={}", receipt.citation);
    println!("disposition={disposition}");
    println!("requires_pnf_review={}", receipt.requires_pnf_review);
    println!("creates_legal_controversy=false");
    println!("independent_source_count_increment=0");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("claim_truth_promoted=false");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
