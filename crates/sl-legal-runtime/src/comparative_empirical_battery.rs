//! M11.4 cross-domain empirical/regression battery.
//!
//! One generic comparison programme must distinguish:
//! 1. world/state changes while theory does not;
//! 2. theory changes while world does not;
//! 3. observation changes while world does not;
//! 4. consumer projection changes while world does not;
//! 5. a legal defeater changes the route;
//! 6. an irrelevant revision does not change the query answer/projection.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    compare_dashi_trade_live_shadow, compare_dashi_trade_quotient_states,
    compare_dashi_trade_thesis, compare_worldmonitor_forecast_runs,
    compile_query_world_impact, consumer_fibre_change_set, run_observation_refinement,
    run_pabai_comparative_regression, run_personal_professional_comparison,
    run_same_world_theory_change, run_state_change_regularity_invariant,
    temporal_change_set, temporal_comparison_from_query_world_impact,
    dashi_trade_phase9_answer_changing_explanation, dashi_trade_phase9_justification_boundary,
    ChangeLayer, ConsumerAxis, DashiTradePhase9Justification, DashiTradeQuotientState,
    DashiTradeShadowPolicyStep, DashiTradeThesisSnapshot, LegalWorldCoordinate,
    ProjectionGraph, ProjectionKind, ProjectionNode, QueryDependencySlice,
    RevisionDependencyIndex, WorldMonitorForecastRun,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeEmpiricalBatteryReceipt {
    pub world_change_theory_invariant: bool,
    pub theory_change_world_invariant: bool,
    pub observation_change_world_invariant: bool,
    pub consumer_projection_change_world_invariant: bool,
    pub consumer_projection_change_typed: bool,
    pub legal_defeater_changes_route: bool,
    pub legal_defeater_change_typed: bool,
    pub irrelevant_revision_changes_query_projection: bool,
    pub irrelevant_revision_change_typed_world_evidence: bool,
    pub irrelevant_revision_reopens_research: bool,
    pub worldmonitor_forecast_change_not_world_change: bool,
    pub worldmonitor_model_and_dashboard_axes_typed: bool,
    pub dashitrade_quotient_query_relative_nonfactorability: bool,
    pub dashitrade_shadow_same_world_policy_delta: bool,
    pub dashitrade_belief_separate_from_world_and_action: bool,
    pub dashitrade_justification_not_causal_proof: bool,
    pub dashitrade_phase9_uses_shared_typed_explanation_abi: bool,
    pub all_modes_candidate_only: bool,
    pub any_mode_creates_semantic_authority: bool,
    pub any_mode_creates_claim_truth: bool,
}

fn temporal_world(
    world_ref: &str,
    relevant_revision: &str,
    irrelevant_revision: &str,
) -> LegalWorldCoordinate {
    LegalWorldCoordinate {
        world_ref: world_ref.into(),
        matter_ref: "matter:m11-battery".into(),
        jurisdiction_ref: "AU".into(),
        as_at: "2026-09-23".into(),
        source_revisions: BTreeMap::from([
            ("source:relevant".into(), relevant_revision.into()),
            ("source:noise".into(), irrelevant_revision.into()),
        ]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn temporal_graph() -> ProjectionGraph {
    ProjectionGraph {
        kind: ProjectionKind::IssueProof,
        nodes: vec![
            ProjectionNode {
                semantic_ref: "prop:q".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:q".into()],
                source_revision_refs: vec!["rev:relevant:1".into()],
                span_refs: vec!["span:q".into()],
                projection_role: "IssueProof".into(),
            },
            ProjectionNode {
                semantic_ref: "prop:noise".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:noise".into()],
                source_revision_refs: vec!["rev:noise:1".into()],
                span_refs: vec!["span:noise".into()],
                projection_role: "IssueProof".into(),
            },
        ],
        edges: vec![],
        deterministic_digest: "sha256:m11-battery-temporal".into(),
        projection_only: true,
        creates_semantic_authority: false,
    }
}

fn temporal_dependencies() -> RevisionDependencyIndex {
    RevisionDependencyIndex {
        source_to_propositions: BTreeMap::from([
            (
                "source:relevant".into(),
                BTreeSet::from(["prop:q".into()]),
            ),
            (
                "source:noise".into(),
                BTreeSet::from(["prop:noise".into()]),
            ),
        ]),
        proposition_dependents: BTreeMap::new(),
    }
}

fn temporal_slice() -> QueryDependencySlice {
    QueryDependencySlice {
        query_ref: "query:m11-battery".into(),
        required_axes: BTreeSet::from([
            ConsumerAxis::SemanticIdentity,
            ConsumerAxis::SourceRevision,
        ]),
        semantic_refs: BTreeSet::from(["prop:q".into()]),
        proof_refs: BTreeSet::new(),
        source_refs: BTreeSet::from(["source:relevant".into()]),
        source_revision_refs: BTreeSet::from(["rev:relevant:1".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn run_comparative_empirical_battery(
) -> Result<ComparativeEmpiricalBatteryReceipt, String> {
    let state = run_state_change_regularity_invariant()?;
    let theory = run_same_world_theory_change()?;
    let observation = run_observation_refinement()?;
    let fibres = run_personal_professional_comparison()?;
    let pabai = run_pabai_comparative_regression()?;

    let old = temporal_world("world:battery:t0", "rev:relevant:1", "rev:noise:1");
    let irrelevant =
        temporal_world("world:battery:t1", "rev:relevant:1", "rev:noise:2");
    let temporal_impact = compile_query_world_impact(
        &old,
        &irrelevant,
        &temporal_dependencies(),
        &temporal_slice(),
        &temporal_graph(),
    )?;
    let temporal = temporal_comparison_from_query_world_impact(&temporal_impact)?;
    let temporal_changes =
        temporal_change_set("comparison:battery:temporal", &temporal)?;
    let regulator_changes = consumer_fibre_change_set(
        "comparison:battery:personal-regulator",
        &fibres.personal_to_regulator,
    )?;

    let wm0 = WorldMonitorForecastRun {
        run_ref: "wm:0".into(),
        forecast_depth: "standard".into(),
        deep_forecast_status: "complete".into(),
        state_labels: BTreeSet::from(["stable".into()]),
        top_forecast_titles: BTreeSet::from(["baseline".into()]),
        traced_forecast_count: 2,
        impact_expansion_candidate_count: 1,
        impact_expansion_mapped_signal_count: 1,
        simulation_interaction_count: 3,
        reportable_interaction_count: 2,
        published_domain_counts: BTreeMap::from([("geopolitics".into(), 1)]),
        source_measurement_refs: BTreeSet::from(["signal:1".into()]),
        forecast_model_ref: Some("model:v1".into()),
        dashboard_projection_ref: Some("risk:v1".into()),
    };
    let mut wm1 = wm0.clone();
    wm1.run_ref = "wm:1".into();
    wm1.state_labels.insert("escalating".into());
    wm1.forecast_model_ref = Some("model:v2".into());
    wm1.dashboard_projection_ref = Some("risk:v2".into());
    let worldmonitor =
        compare_worldmonitor_forecast_runs("comparison:battery:worldmonitor", &wm0, &wm1)?;

    let quotient = compare_dashi_trade_quotient_states(
        "comparison:battery:quotient",
        &DashiTradeQuotientState {
            raw_world_ref: "raw:A".into(),
            representative_ref: "q:same".into(),
            contradiction_ref: "contradiction:same".into(),
            nuisance_symmetry_ref: "perm:trades".into(),
        },
        &DashiTradeQuotientState {
            raw_world_ref: "raw:B".into(),
            representative_ref: "q:same".into(),
            contradiction_ref: "contradiction:same".into(),
            nuisance_symmetry_ref: "perm:trades".into(),
        },
    )?;

    let shadow = compare_dashi_trade_live_shadow(
        "comparison:battery:shadow",
        &DashiTradeShadowPolicyStep {
            world_ref: "world:same".into(),
            observation_ref: "observation:same".into(),
            live_policy_ref: "policy:live".into(),
            shadow_policy_ref: "policy:shadow".into(),
            live_action_ref: "action:hold".into(),
            shadow_action_ref: "action:long".into(),
        },
    )?;

    let thesis = compare_dashi_trade_thesis(
        "comparison:battery:thesis",
        &DashiTradeThesisSnapshot {
            world_ref: "world:same".into(),
            observation_ref: "obs:0".into(),
            thesis_direction: 1,
            thesis_strength: 1,
            thesis_age: 3,
            cooldown: 0,
            invalidation: 0,
            action_ref: "action:long".into(),
        },
        &DashiTradeThesisSnapshot {
            world_ref: "world:same".into(),
            observation_ref: "obs:1".into(),
            thesis_direction: 1,
            thesis_strength: 2,
            thesis_age: 4,
            cooldown: 0,
            invalidation: 0,
            action_ref: "action:long".into(),
        },
    )?;

    let trade_justification = dashi_trade_phase9_justification_boundary(
        &DashiTradePhase9Justification {
            regime_ref: "regime:positive-edge".into(),
            posture_ref: "posture:trade-normal".into(),
            actuator_ref: "actuator:bar-exec".into(),
            cost_model_ref: "cost:phase9".into(),
            expected_surplus_ref: "expected:positive".into(),
            realised_surplus_ref: "realised:positive".into(),
        },
    );

    let trade_explanation = dashi_trade_phase9_answer_changing_explanation(
        "query:battery:dashitrade-actionability",
        "answer:actionable",
        "answer:held",
        "coordinate:dashitrade:phase9-gate",
        "applicability:open",
        "applicability:hold",
        &DashiTradePhase9Justification {
            regime_ref: "regime:hazard-observe".into(),
            posture_ref: "posture:observe".into(),
            actuator_ref: "actuator:bar-exec".into(),
            cost_model_ref: "cost:phase9".into(),
            expected_surplus_ref: "expected:nonpositive".into(),
            realised_surplus_ref: "realised:later-observed".into(),
        },
    )?;

    let consumer_projection_change_world_invariant =
        fibres.personal_to_lawyer.world_ref == fibres.personal_to_regulator.world_ref
            && (fibres.personal_to_lawyer.shared_visible_coordinate_refs
                != fibres.personal_to_regulator.shared_visible_coordinate_refs
                || fibres
                    .personal_to_lawyer
                    .right_dependency_irrelevant_coordinate_refs
                    != fibres
                        .personal_to_regulator
                        .right_dependency_irrelevant_coordinate_refs);

    Ok(ComparativeEmpiricalBatteryReceipt {
        world_change_theory_invariant: state.state_changed
            && !state.theory_changed
            && state
                .typed_changes
                .changed_layers
                .contains(&ChangeLayer::World),
        theory_change_world_invariant: theory.world_identity_held_fixed
            && theory
                .typed_changes
                .changed_layers
                .contains(&ChangeLayer::Theory)
            && theory
                .typed_changes
                .invariant_layers
                .contains(&ChangeLayer::World),
        observation_change_world_invariant: observation.world_identity_held_fixed
            && observation
                .typed_changes
                .changed_layers
                .contains(&ChangeLayer::Observation)
            && observation
                .typed_changes
                .invariant_layers
                .contains(&ChangeLayer::World),
        consumer_projection_change_world_invariant,
        consumer_projection_change_typed: regulator_changes
            .changed_layers
            .contains(&ChangeLayer::ConsumerProjection)
            && regulator_changes.changed_layers.contains(&ChangeLayer::Scope)
            && regulator_changes.invariant_layers.contains(&ChangeLayer::World),
        legal_defeater_changes_route: pabai
            .w0_to_w1
            .changed_route_refs
            .contains("route:pabai:comparative-duty")
            && pabai
                .w0_to_w1_distinction
                .delta_refs
                .contains("delta:pabai:w0-w1:defeater"),
        legal_defeater_change_typed:
            pabai.w0_to_w1_locus.layer == ChangeLayer::Applicability,
        irrelevant_revision_changes_query_projection: temporal.query_projection_changed,
        irrelevant_revision_change_typed_world_evidence: temporal_changes
            .changed_layers
            .contains(&ChangeLayer::WorldEvidence)
            && !temporal_changes.changed_layers.contains(&ChangeLayer::World),
        irrelevant_revision_reopens_research: temporal.reopens_consumer_research,
        worldmonitor_forecast_change_not_world_change:
            worldmonitor
                .change_set
                .changed_layers
                .contains(&ChangeLayer::Representation)
            && worldmonitor
                .change_set
                .invariant_layers
                .contains(&ChangeLayer::World)
            && !worldmonitor.world_changed_inferred,
        worldmonitor_model_and_dashboard_axes_typed:
            worldmonitor.change_set.changed_layers.contains(&ChangeLayer::Theory)
            && worldmonitor
                .change_set
                .changed_layers
                .contains(&ChangeLayer::ConsumerProjection),
        dashitrade_quotient_query_relative_nonfactorability:
            quotient.raw_worlds_differ
            && quotient.quotient_representations_equal
            && quotient.declared_query_factors_through
            && !quotient.richer_query_factors_through
            && !quotient.erased_detail_recovered_by_recharting,
        dashitrade_shadow_same_world_policy_delta:
            shadow.world_invariant
            && shadow.observation_invariant
            && shadow.policy_changed
            && shadow
                .change_set
                .changed_layers
                .contains(&ChangeLayer::Representation),
        dashitrade_belief_separate_from_world_and_action:
            !thesis.world_changed
            && thesis.belief_changed
            && !thesis.action_changed
            && thesis
                .change_set
                .changed_layers
                .contains(&ChangeLayer::Belief),
        dashitrade_justification_not_causal_proof:
            !trade_justification.proves_market_causation
            && !trade_justification.proves_global_theory_correctness
            && !trade_justification.creates_claim_truth,
        dashitrade_phase9_uses_shared_typed_explanation_abi:
            trade_explanation.locus.layer == ChangeLayer::Applicability
            && trade_explanation.explanation.all_minimal_deltas_typed
            && !trade_explanation.explanation.claims_causation_beyond_receipts
            && !trade_explanation.explanation.predicts_outcome,
        all_modes_candidate_only: state.candidate_only
            && theory.candidate_only
            && observation.candidate_only
            && fibres.candidate_only
            && pabai.candidate_only
            && temporal.candidate_only,
        any_mode_creates_semantic_authority: state.creates_semantic_authority
            || theory.creates_semantic_authority
            || observation.creates_semantic_authority
            || fibres.creates_semantic_authority
            || pabai.creates_semantic_authority
            || temporal.creates_semantic_authority,
        any_mode_creates_claim_truth: state.creates_claim_truth
            || theory.creates_claim_truth
            || observation.creates_claim_truth
            || fibres.creates_claim_truth
            || pabai.creates_claim_truth
            || temporal.creates_claim_truth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_comparison_battery_distinguishes_all_six_change_patterns() {
        let receipt = run_comparative_empirical_battery().unwrap();

        assert!(receipt.world_change_theory_invariant);
        assert!(receipt.theory_change_world_invariant);
        assert!(receipt.observation_change_world_invariant);
        assert!(receipt.consumer_projection_change_world_invariant);
        assert!(receipt.consumer_projection_change_typed);
        assert!(receipt.legal_defeater_changes_route);
        assert!(receipt.legal_defeater_change_typed);
        assert!(!receipt.irrelevant_revision_changes_query_projection);
        assert!(receipt.irrelevant_revision_change_typed_world_evidence);
        assert!(!receipt.irrelevant_revision_reopens_research);
        assert!(receipt.worldmonitor_forecast_change_not_world_change);
        assert!(receipt.worldmonitor_model_and_dashboard_axes_typed);
        assert!(receipt.dashitrade_quotient_query_relative_nonfactorability);
        assert!(receipt.dashitrade_shadow_same_world_policy_delta);
        assert!(receipt.dashitrade_belief_separate_from_world_and_action);
        assert!(receipt.dashitrade_justification_not_causal_proof);
        assert!(receipt.dashitrade_phase9_uses_shared_typed_explanation_abi);
        assert!(receipt.all_modes_candidate_only);
        assert!(!receipt.any_mode_creates_semantic_authority);
        assert!(!receipt.any_mode_creates_claim_truth);
    }
}
