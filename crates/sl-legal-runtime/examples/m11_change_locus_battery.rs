use sensiblaw_legal_runtime::{
    compile_typed_change_set, legal_route_comparative_receipt,
    newton_gr_effective_lineage, run_comparative_empirical_battery,
    run_observation_refinement, run_pabai_comparative_regression,
    run_same_world_theory_change, run_state_change_regularity_invariant,
    typed_comparative_receipt_extension,
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

    let pabai_base = legal_route_comparative_receipt(
        "comparison:pabai:w0-w1",
        &pabai.w0_to_w1,
        "query:pabai:duty-route",
        "consumer:pabai-climate-duty",
        Some(&pabai.w0_to_w1_distinction),
    )?;
    let pabai_change_set = compile_typed_change_set(
        "comparison:pabai:w0-w1",
        [pabai.w0_to_w1_locus.clone()],
        [],
    )?;
    let pabai_typed = typed_comparative_receipt_extension(
        &pabai_base,
        &pabai_change_set,
        Some(&pabai.w0_to_w1_explanation),
    )?;

    println!(
        "typed_receipt_schema={}",
        pabai_typed.schema_version
    );
    println!(
        "typed_receipt_base_schema={}",
        pabai_typed.base_schema_version
    );
    println!(
        "typed_receipt_all_answer_changing_deltas_typed={}",
        pabai_typed.all_answer_changing_deltas_typed
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
