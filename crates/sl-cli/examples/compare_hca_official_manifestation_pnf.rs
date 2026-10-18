//! Resolve an official HCA PDF↔DOCX transformation residual through the
//! existing candidate Predicate-PNF / relational comparison machinery.
//!
//! The caller supplies already-persisted candidate-PNF batch and predicate
//! coordinates for corresponding exact spans.  This command never guesses a
//! predicate, manufactures semantic alignment, or creates legal authority.

use std::{env, process};

use sensiblaw_pg_source_store::{
    compare_and_persist_official_primary_manifestation_pnf_overlap, load_database_config,
};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(
            "usage: compare_hca_official_manifestation_pnf <manifestation-comparison-ref> <pdf-batch-ref> <docx-batch-ref> <pdf-predicate-candidate-ref> <docx-predicate-candidate-ref>"
                .into(),
        );
    }
    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let receipt = compare_and_persist_official_primary_manifestation_pnf_overlap(
        &config,
        &args[0],
        &args[1],
        &args[2],
        &args[3],
        &args[4],
    )
    .map_err(|error| error.to_string())?;

    println!("overlap_ref={}", receipt.overlap_ref);
    println!("relational_comparison_ref={}", receipt.relational_comparison_ref);
    println!("finding={:?}", receipt.finding);
    println!("disposition={:?}", receipt.classification.disposition);
    println!(
        "requires_human_pnf_review={}",
        receipt.classification.requires_human_pnf_review
    );
    println!(
        "closes_source_transformation_residual={}",
        receipt.classification.closes_source_transformation_residual
    );
    println!(
        "independent_source_count_increment={}",
        receipt.classification.independent_source_count_increment
    );
    println!("creates_legal_controversy=false");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    for residual in receipt.residual_obligation_refs {
        println!("residual_obligation_ref={residual}");
    }
    for witness in receipt.alignment_witness_refs {
        println!("alignment_witness_ref={witness}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
