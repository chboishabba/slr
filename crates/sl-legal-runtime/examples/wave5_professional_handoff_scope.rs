//! M9 real Wave-5 handoff gate with no fabricated human scope decisions.
//!
//! This executable intentionally supplies an empty scope-decision set. The
//! reviewed therapist-note coordinate must therefore remain unresolved for
//! professional handoff.

use sensiblaw_legal_runtime::compile_wave5_professional_handoff_receipt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let receipt =
        compile_wave5_professional_handoff_receipt(&[], std::iter::empty::<String>())?;

    println!("source_run_ref={}", receipt.source_run_ref);
    println!(
        "review_state_by_coordinate={:?}",
        receipt.review_state_by_coordinate
    );
    println!(
        "unresolved_scope_coordinates={}",
        receipt
            .scope
            .unresolved_coordinate_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "included_consumer_count={}",
        receipt.scope.included_by_consumer.len()
    );
    println!(
        "changed_coordinate_count={}",
        receipt.changed_coordinate_refs.len()
    );
    println!("candidate_only={}", receipt.candidate_only);
    println!(
        "creates_semantic_authority={}",
        receipt.creates_semantic_authority
    );
    println!("creates_claim_truth={}", receipt.creates_claim_truth);

    Ok(())
}
