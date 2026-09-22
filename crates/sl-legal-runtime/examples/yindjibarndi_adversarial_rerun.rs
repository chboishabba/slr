//! M8.1 Yindjibarndi adversarial shared-world rerun.
//!
//! Prints a candidate-only machine-readable-ish summary.  It does not predict
//! or state a judicial outcome.

use sensiblaw_legal_runtime::run_yindjibarndi_live_case;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let run = run_yindjibarndi_live_case()?;

    println!("consumer_ref=consumer:yindjibarndi-compensation");
    println!("route_ref={}", run.adversarial.route_ref);
    println!("route_status={:?}", run.adversarial.route_status);
    println!(
        "reused_coordinates={}",
        run.reuse_decisions
            .iter()
            .filter(|decision| decision.reusable)
            .map(|decision| decision.coordinate_ref.as_str())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "active_defeaters={}",
        run.adversarial
            .active_defeater_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "counter_defeaters={}",
        run.adversarial
            .counter_defeater_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "paid_atoms={}",
        run.adversarial
            .summary
            .paid_atom_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "defeated_routes={}",
        run.adversarial
            .summary
            .defeated_route_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "contested_coordinates={}",
        run.adversarial
            .summary
            .contested_coordinate_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "wrong_type_gaps={}",
        run.adversarial.summary.wrong_type_gap_refs.len()
    );
    println!(
        "authority_gaps={}",
        run.adversarial.summary.authority_gap_refs.len()
    );
    println!("fact_gaps={}", run.adversarial.summary.fact_gap_refs.len());
    println!(
        "applicability_gaps={}",
        run.adversarial
            .summary
            .applicability_gap_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "remaining_reopening_cut={}",
        run.adversarial
            .summary
            .remaining_reopening_cut_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    for gap in &run.adversarial.typed_gaps {
        println!(
            "gap={}|kind={:?}|coordinate={}|proposition={}",
            gap.gap_ref, gap.kind, gap.coordinate_ref, gap.proposition_ref
        );
    }
    for demand in &run.adversarial.next_demands {
        println!(
            "next_demand={}|role={:?}|target={}",
            demand.demand_ref, demand.role, demand.target_atom_or_residual_ref
        );
    }
    println!(
        "generic_mabo_join_rejected={}",
        run.generic_mabo_join_rejected
    );
    println!("candidate_only={}", run.candidate_only);
    println!(
        "creates_semantic_authority={}",
        run.creates_semantic_authority
    );
    println!("creates_claim_truth={}", run.creates_claim_truth);

    Ok(())
}
