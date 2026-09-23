//! M11 bounded comparative receipt surface.
//!
//! Emits the three first comparison modes without ranking or semantic
//! promotion:
//!   A. Pabai legal route change;
//!   B. personal/professional fibre change;
//!   C. adversarial-party treatment change.

use sensiblaw_legal_runtime::{
    run_pabai_comparative_regression, run_personal_professional_comparison,
    run_yindjibarndi_party_comparison,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pabai = run_pabai_comparative_regression()?;
    let fibres = run_personal_professional_comparison()?;
    let parties = run_yindjibarndi_party_comparison()?;

    println!(
        "pabai_w0_w1_distinction={}",
        pabai
            .w0_to_w1_distinction
            .delta_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "pabai_w1_w2_distinction={}",
        pabai
            .w1_to_w2_distinction
            .delta_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "pabai_w0_w1_changed_routes={}",
        pabai
            .w0_to_w1
            .changed_route_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );

    println!(
        "personal_lawyer_shared={}",
        fibres
            .personal_to_lawyer
            .shared_visible_coordinate_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "personal_regulator_scope_blocked={}",
        fibres
            .personal_to_regulator
            .right_scope_blocked_coordinate_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "personal_regulator_dependency_irrelevant={}",
        fibres
            .personal_to_regulator
            .right_dependency_irrelevant_coordinate_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );

    println!(
        "applicant_state_changed_treatment={}",
        parties
            .applicant_vs_state
            .coordinates_with_different_treatment
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "applicant_fmg_changed_treatment={}",
        parties
            .applicant_vs_fmg
            .coordinates_with_different_treatment
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("submissions_are_not_holdings={}", parties.submissions_are_not_holdings);
    println!("predicts_outcome={}", parties.predicts_outcome);

    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("creates_claim_truth=false");

    Ok(())
}
