use sensiblaw_legal_runtime::{
    newton_gr_effective_lineage, run_comparative_empirical_battery,
    run_observation_refinement, run_pabai_comparative_regression,
    run_same_world_theory_change, run_state_change_regularity_invariant,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let battery = run_comparative_empirical_battery()?;
    let theory = run_same_world_theory_change()?;
    let observation = run_observation_refinement()?;
    let state = run_state_change_regularity_invariant()?;
    let lineage = newton_gr_effective_lineage()?;
    let pabai = run_pabai_comparative_regression()?;

    println!("world_change_theory_invariant={}", battery.world_change_theory_invariant);
    println!("theory_change_world_invariant={}", battery.theory_change_world_invariant);
    println!("observation_change_world_invariant={}", battery.observation_change_world_invariant);
    println!(
        "consumer_projection_change_world_invariant={}",
        battery.consumer_projection_change_world_invariant
    );
    println!("legal_defeater_changes_route={}", battery.legal_defeater_changes_route);
    println!(
        "irrelevant_revision_changes_query_projection={}",
        battery.irrelevant_revision_changes_query_projection
    );
    println!(
        "irrelevant_revision_reopens_research={}",
        battery.irrelevant_revision_reopens_research
    );

    println!(
        "coarse_theory_query_answer_changed={}",
        theory.coarse_query_answer_changed
    );
    println!(
        "strong_theory_query_answer_changed={}",
        theory.strong_query_answer_changed
    );
    println!(
        "strong_theory_answer_changing_delta={}",
        theory
            .strong_query_distinction
            .delta_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "observation_answer_changing_delta={}",
        observation
            .discriminating_query_distinction
            .delta_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!("regularity_changed={}", state.regularity_changed);
    println!("theory_changed_during_state_change={}", state.theory_changed);

    println!("effective_theory_relation={:?}", lineage.relation);
    println!(
        "effective_valid_regimes={}",
        lineage
            .valid_regime_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "effective_outside_regimes={}",
        lineage
            .outside_regime_refs
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );

    println!(
        "pabai_D_layer={:?}",
        pabai.w0_to_w1_explanation.steps[0].layer
    );
    println!(
        "pabai_D_explanation={}",
        pabai.w0_to_w1_explanation.steps[0].explanation_ref
    );
    println!(
        "pabai_C_layer={:?}",
        pabai.w1_to_w2_explanation.steps[0].layer
    );
    println!(
        "pabai_C_explanation={}",
        pabai.w1_to_w2_explanation.steps[0].explanation_ref
    );

    println!("all_modes_candidate_only={}", battery.all_modes_candidate_only);
    println!(
        "creates_semantic_authority={}",
        battery.any_mode_creates_semantic_authority
    );
    println!("creates_claim_truth={}", battery.any_mode_creates_claim_truth);

    Ok(())
}
