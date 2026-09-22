//! M8.1 executable Yindjibarndi finite-cut recompute experiment.

use sensiblaw_legal_runtime::run_yindjibarndi_finite_cut_experiment;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let experiment = run_yindjibarndi_finite_cut_experiment()?;

    println!("support_reachable={}", experiment.support_reachable);
    println!("support_cut={:?}", experiment.support_cut);
    println!("defeated_reachable={}", experiment.defeated_reachable);
    println!("defeated_cut={:?}", experiment.defeated_cut);
    println!(
        "mabo_only_repair_reachable={}",
        experiment.mabo_repair_reachable
    );
    println!("mabo_only_repair_cut={:?}", experiment.mabo_repair_cut);
    println!(
        "fully_repaired_candidate_reachable={}",
        experiment.fully_repaired_candidate_reachable
    );
    println!(
        "fully_repaired_candidate_cut={:?}",
        experiment.fully_repaired_candidate_cut
    );
    println!(
        "recomputed_cut_changed={}",
        experiment.recompute_receipt.cut_changed
    );
    println!("candidate_only={}", experiment.candidate_only);
    println!(
        "creates_semantic_authority={}",
        experiment.creates_semantic_authority
    );
    println!("creates_claim_truth={}", experiment.creates_claim_truth);

    Ok(())
}
