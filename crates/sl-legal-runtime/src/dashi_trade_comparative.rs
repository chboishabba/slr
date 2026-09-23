//! M11 cross-domain acceptance fixtures for dashiTRADE.
//!
//! The shapes mirror dashiTRADE's quotient map, thesis-memory,
//! ShadowPolicyRunner, and Phase-9 justification-chain contracts. They are
//! bounded comparative adapters, not a trading engine.

use serde::{Deserialize, Serialize};

pub const DASHITRADE_COMPARATIVE_SOURCE_COMMIT: &str =
    "04ba378bd38df61a223af8c27e3ae4d18d2b0c3f";
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    compile_typed_change_set, explain_answer_changing_distinction, typed_locus,
    AnswerChangingDistinction, ChangeLayer, ChangeLocus, ComparativeDelta,
    ComparativeDeltaKind, ComparativeDeltaRole, TypedAnswerChangingExplanation,
    TypedChangeSet,
};

fn input_delta(
    delta_ref: &str,
    coordinate_ref: &str,
    before_ref: Option<String>,
    after_ref: Option<String>,
) -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: delta_ref.into(),
        kind: ComparativeDeltaKind::FactChanged,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(coordinate_ref.into()),
        route_ref: None,
        residual_ref: None,
        before_ref,
        after_ref,
        cause_refs: BTreeSet::new(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn outcome_delta(
    delta_ref: &str,
    route_ref: &str,
    before_ref: Option<String>,
    after_ref: Option<String>,
) -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: delta_ref.into(),
        kind: ComparativeDeltaKind::RouteStatusChanged,
        role: ComparativeDeltaRole::ProofOutcome,
        coordinate_ref: None,
        route_ref: Some(route_ref.into()),
        residual_ref: None,
        before_ref,
        after_ref,
        cause_refs: BTreeSet::new(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeQuotientState {
    pub raw_world_ref: String,
    pub representative_ref: String,
    pub contradiction_ref: String,
    pub nuisance_symmetry_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeQuotientReceipt {
    pub change_set: TypedChangeSet,
    pub raw_worlds_differ: bool,
    pub quotient_representations_equal: bool,
    pub declared_query_factors_through: bool,
    pub richer_query_factors_through: bool,
    pub erased_detail_recovered_by_recharting: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compare_dashi_trade_quotient_states(
    comparison_ref: &str,
    left: &DashiTradeQuotientState,
    right: &DashiTradeQuotientState,
) -> Result<DashiTradeQuotientReceipt, String> {
    let raw_worlds_differ = left.raw_world_ref != right.raw_world_ref;
    let quotient_representations_equal =
        left.representative_ref == right.representative_ref;
    let contradiction_preserved =
        left.contradiction_ref == right.contradiction_ref;

    let mut loci = Vec::new();
    if raw_worlds_differ {
        loci.push(typed_locus(
            "locus:dashitrade:raw-world",
            input_delta(
                "delta:dashitrade:raw-world",
                "coordinate:dashitrade:raw-field",
                Some(left.raw_world_ref.clone()),
                Some(right.raw_world_ref.clone()),
            ),
            ChangeLayer::World,
            Some("raw-market-field".into()),
            BTreeSet::new(),
            BTreeSet::from(["dashitrade:quotient-map:raw-field".into()]),
        )?);
    }
    if !quotient_representations_equal {
        loci.push(typed_locus(
            "locus:dashitrade:quotient-representation",
            input_delta(
                "delta:dashitrade:quotient-representation",
                "coordinate:dashitrade:quotient-representative",
                Some(left.representative_ref.clone()),
                Some(right.representative_ref.clone()),
            ),
            ChangeLayer::Representation,
            Some("quotient-representative".into()),
            BTreeSet::new(),
            BTreeSet::from(["dashitrade:quotient-map".into()]),
        )?);
    }

    let invariants = if quotient_representations_equal {
        vec![ChangeLayer::Representation]
    } else {
        vec![]
    };
    let change_set = compile_typed_change_set(comparison_ref, loci, invariants)?;

    Ok(DashiTradeQuotientReceipt {
        change_set,
        raw_worlds_differ,
        quotient_representations_equal,
        declared_query_factors_through: quotient_representations_equal
            && contradiction_preserved,
        richer_query_factors_through: false,
        erased_detail_recovered_by_recharting: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeShadowPolicyStep {
    pub world_ref: String,
    pub observation_ref: String,
    pub live_policy_ref: String,
    pub shadow_policy_ref: String,
    pub live_action_ref: String,
    pub shadow_action_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeShadowPolicyReceipt {
    pub change_set: TypedChangeSet,
    pub world_invariant: bool,
    pub observation_invariant: bool,
    pub policy_changed: bool,
    pub action_changed: bool,
    pub action_change_is_outcome_not_world: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compare_dashi_trade_live_shadow(
    comparison_ref: &str,
    step: &DashiTradeShadowPolicyStep,
) -> Result<DashiTradeShadowPolicyReceipt, String> {
    let policy_changed = step.live_policy_ref != step.shadow_policy_ref;
    let action_changed = step.live_action_ref != step.shadow_action_ref;
    let mut loci = Vec::new();

    if policy_changed {
        loci.push(typed_locus(
            "locus:dashitrade:live-shadow-policy",
            input_delta(
                "delta:dashitrade:live-shadow-policy",
                "coordinate:dashitrade:policy-representation",
                Some(step.live_policy_ref.clone()),
                Some(step.shadow_policy_ref.clone()),
            ),
            ChangeLayer::Representation,
            Some("live-vs-shadow-policy".into()),
            BTreeSet::from(["layer:world".into(), "layer:observation".into()]),
            BTreeSet::from(["dashitrade:ShadowPolicyRunner".into()]),
        )?);
    }

    if action_changed {
        loci.push(typed_locus(
            "locus:dashitrade:live-shadow-action",
            outcome_delta(
                "delta:dashitrade:live-shadow-action",
                "route:dashitrade:policy-action",
                Some(step.live_action_ref.clone()),
                Some(step.shadow_action_ref.clone()),
            ),
            ChangeLayer::ProofOutcome,
            Some("policy-action-output".into()),
            BTreeSet::from(["layer:world".into(), "layer:observation".into()]),
            BTreeSet::from([
                "dashitrade:ShadowPolicyRunner".into(),
                "dashitrade:policy-output".into(),
            ]),
        )?);
    }

    let change_set = compile_typed_change_set(
        comparison_ref,
        loci,
        [ChangeLayer::World, ChangeLayer::Observation],
    )?;

    Ok(DashiTradeShadowPolicyReceipt {
        change_set,
        world_invariant: true,
        observation_invariant: true,
        policy_changed,
        action_changed,
        action_change_is_outcome_not_world: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeThesisSnapshot {
    pub world_ref: String,
    pub observation_ref: String,
    pub thesis_direction: i8,
    pub thesis_strength: u8,
    pub thesis_age: u32,
    pub cooldown: u32,
    pub invalidation: u8,
    pub action_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeThesisReceipt {
    pub change_set: TypedChangeSet,
    pub world_changed: bool,
    pub observation_changed: bool,
    pub belief_changed: bool,
    pub action_changed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn thesis_ref(state: &DashiTradeThesisSnapshot) -> String {
    format!(
        "d={};s={};a={};c={};v={}",
        state.thesis_direction,
        state.thesis_strength,
        state.thesis_age,
        state.cooldown,
        state.invalidation
    )
}

pub fn compare_dashi_trade_thesis(
    comparison_ref: &str,
    before: &DashiTradeThesisSnapshot,
    after: &DashiTradeThesisSnapshot,
) -> Result<DashiTradeThesisReceipt, String> {
    let world_changed = before.world_ref != after.world_ref;
    let observation_changed = before.observation_ref != after.observation_ref;
    let belief_changed = thesis_ref(before) != thesis_ref(after);
    let action_changed = before.action_ref != after.action_ref;
    let mut loci = Vec::new();

    if world_changed {
        loci.push(typed_locus(
            "locus:dashitrade:world",
            input_delta(
                "delta:dashitrade:world",
                "coordinate:dashitrade:market-world",
                Some(before.world_ref.clone()),
                Some(after.world_ref.clone()),
            ),
            ChangeLayer::World,
            Some("market-state".into()),
            BTreeSet::new(),
            BTreeSet::from(["dashitrade:market-state".into()]),
        )?);
    }
    if observation_changed {
        loci.push(typed_locus(
            "locus:dashitrade:observation",
            input_delta(
                "delta:dashitrade:observation",
                "coordinate:dashitrade:observation",
                Some(before.observation_ref.clone()),
                Some(after.observation_ref.clone()),
            ),
            ChangeLayer::Observation,
            Some("plane-risk-observation".into()),
            BTreeSet::new(),
            BTreeSet::from(["dashitrade:thesis-inputs".into()]),
        )?);
    }
    if belief_changed {
        loci.push(typed_locus(
            "locus:dashitrade:thesis-memory",
            input_delta(
                "delta:dashitrade:thesis-memory",
                "coordinate:dashitrade:belief-thesis",
                Some(thesis_ref(before)),
                Some(thesis_ref(after)),
            ),
            ChangeLayer::Belief,
            Some("thesis-memory".into()),
            BTreeSet::new(),
            BTreeSet::from(["dashitrade:ThesisState".into()]),
        )?);
    }
    if action_changed {
        loci.push(typed_locus(
            "locus:dashitrade:action",
            outcome_delta(
                "delta:dashitrade:action",
                "route:dashitrade:final-action",
                Some(before.action_ref.clone()),
                Some(after.action_ref.clone()),
            ),
            ChangeLayer::ProofOutcome,
            Some("action-output".into()),
            BTreeSet::new(),
            BTreeSet::from(["dashitrade:apply_thesis_constraints".into()]),
        )?);
    }

    let change_set = compile_typed_change_set(comparison_ref, loci, [])?;

    Ok(DashiTradeThesisReceipt {
        change_set,
        world_changed,
        observation_changed,
        belief_changed,
        action_changed,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradePhase9Justification {
    pub regime_ref: String,
    pub posture_ref: String,
    pub actuator_ref: String,
    pub cost_model_ref: String,
    pub expected_surplus_ref: String,
    pub realised_surplus_ref: String,
}

impl DashiTradePhase9Justification {
    pub fn justification_refs(&self) -> BTreeSet<String> {
        BTreeSet::from([
            self.regime_ref.clone(),
            self.posture_ref.clone(),
            self.actuator_ref.clone(),
            self.cost_model_ref.clone(),
            self.expected_surplus_ref.clone(),
            self.realised_surplus_ref.clone(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradeJustificationBoundaryReceipt {
    pub justification_refs: BTreeSet<String>,
    pub proves_market_causation: bool,
    pub proves_global_theory_correctness: bool,
    pub creates_claim_truth: bool,
}

pub fn dashi_trade_phase9_justification_boundary(
    chain: &DashiTradePhase9Justification,
) -> DashiTradeJustificationBoundaryReceipt {
    DashiTradeJustificationBoundaryReceipt {
        justification_refs: chain.justification_refs(),
        proves_market_causation: false,
        proves_global_theory_correctness: false,
        creates_claim_truth: false,
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashiTradePhase9ExplanationReceipt {
    pub locus: ChangeLocus,
    pub explanation: TypedAnswerChangingExplanation,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn dashi_trade_phase9_answer_changing_explanation(
    query_ref: &str,
    baseline_answer_ref: &str,
    target_answer_ref: &str,
    semantic_coordinate_ref: &str,
    before_ref: &str,
    after_ref: &str,
    chain: &DashiTradePhase9Justification,
) -> Result<DashiTradePhase9ExplanationReceipt, String> {
    if query_ref.trim().is_empty()
        || semantic_coordinate_ref.trim().is_empty()
        || before_ref.trim().is_empty()
        || after_ref.trim().is_empty()
    {
        return Err("dashiTRADE Phase-9 explanation requires identity refs".into());
    }

    let delta_ref = "delta:dashitrade:phase9-applicability".to_string();
    let delta = ComparativeDelta {
        delta_ref: delta_ref.clone(),
        kind: ComparativeDeltaKind::ApplicabilityChanged,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(semantic_coordinate_ref.into()),
        route_ref: Some("route:dashitrade:phase9-action".into()),
        residual_ref: None,
        before_ref: Some(before_ref.into()),
        after_ref: Some(after_ref.into()),
        cause_refs: chain.justification_refs(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    let locus = typed_locus(
        "locus:dashitrade:phase9-applicability",
        delta,
        ChangeLayer::Applicability,
        Some("phase9-gate-refusal".into()),
        BTreeSet::from(["layer:world".into(), "layer:observation".into()]),
        chain.justification_refs(),
    )?;

    let distinction = AnswerChangingDistinction {
        query_ref: query_ref.into(),
        baseline_answer_ref: baseline_answer_ref.into(),
        target_answer_ref: target_answer_ref.into(),
        delta_refs: BTreeSet::from([delta_ref.clone()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    let explanation = explain_answer_changing_distinction(
        &distinction,
        &[locus.clone()],
        &BTreeMap::from([(
            delta_ref,
            "Phase-9 regime/posture/gate/cost chain changes action applicability; it does not prove the realised market outcome".into(),
        )]),
    )?;

    Ok(DashiTradePhase9ExplanationReceipt {
        locus,
        explanation,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotient_can_be_query_adequate_without_recovering_erased_raw_difference() {
        let left = DashiTradeQuotientState {
            raw_world_ref: "raw:micro-order-A".into(),
            representative_ref: "q:stable-contradiction".into(),
            contradiction_ref: "contradiction:wick-imbalance".into(),
            nuisance_symmetry_ref: "perm:trades".into(),
        };
        let right = DashiTradeQuotientState {
            raw_world_ref: "raw:micro-order-B".into(),
            representative_ref: "q:stable-contradiction".into(),
            contradiction_ref: "contradiction:wick-imbalance".into(),
            nuisance_symmetry_ref: "perm:trades".into(),
        };
        let receipt =
            compare_dashi_trade_quotient_states("comparison:q", &left, &right).unwrap();

        assert!(receipt.raw_worlds_differ);
        assert!(receipt.quotient_representations_equal);
        assert!(receipt.declared_query_factors_through);
        assert!(!receipt.richer_query_factors_through);
        assert!(!receipt.erased_detail_recovered_by_recharting);
        assert!(receipt.change_set.changed_layers.contains(&ChangeLayer::World));
        assert!(receipt
            .change_set
            .invariant_layers
            .contains(&ChangeLayer::Representation));
    }

    #[test]
    fn live_shadow_policy_changes_representation_with_world_and_observation_fixed() {
        let receipt = compare_dashi_trade_live_shadow(
            "comparison:shadow",
            &DashiTradeShadowPolicyStep {
                world_ref: "world:same".into(),
                observation_ref: "obs:same".into(),
                live_policy_ref: "policy:live".into(),
                shadow_policy_ref: "policy:futures".into(),
                live_action_ref: "action:hold".into(),
                shadow_action_ref: "action:long".into(),
            },
        )
        .unwrap();

        assert!(receipt.policy_changed);
        assert!(receipt.action_changed);
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::Representation));
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::ProofOutcome));
        assert!(receipt
            .change_set
            .invariant_layers
            .contains(&ChangeLayer::World));
        assert!(receipt
            .change_set
            .invariant_layers
            .contains(&ChangeLayer::Observation));
    }

    #[test]
    fn thesis_belief_may_change_while_world_and_action_stay_fixed() {
        let before = DashiTradeThesisSnapshot {
            world_ref: "world:same".into(),
            observation_ref: "obs:0".into(),
            thesis_direction: 1,
            thesis_strength: 1,
            thesis_age: 3,
            cooldown: 0,
            invalidation: 0,
            action_ref: "action:long".into(),
        };
        let after = DashiTradeThesisSnapshot {
            world_ref: "world:same".into(),
            observation_ref: "obs:1".into(),
            thesis_direction: 1,
            thesis_strength: 2,
            thesis_age: 4,
            cooldown: 0,
            invalidation: 0,
            action_ref: "action:long".into(),
        };
        let receipt =
            compare_dashi_trade_thesis("comparison:thesis", &before, &after).unwrap();

        assert!(!receipt.world_changed);
        assert!(receipt.observation_changed);
        assert!(receipt.belief_changed);
        assert!(!receipt.action_changed);
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::Observation));
        assert!(receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::Belief));
        assert!(!receipt
            .change_set
            .changed_layers
            .contains(&ChangeLayer::ProofOutcome));
    }


    #[test]
    fn phase9_justification_enters_shared_typed_explanation_abi() {
        let chain = DashiTradePhase9Justification {
            regime_ref: "regime:hazard-observe".into(),
            posture_ref: "posture:observe".into(),
            actuator_ref: "actuator:bar-exec".into(),
            cost_model_ref: "cost:phase9-v2".into(),
            expected_surplus_ref: "expected:nonpositive".into(),
            realised_surplus_ref: "realised:later-observed".into(),
        };
        let receipt = dashi_trade_phase9_answer_changing_explanation(
            "query:dashitrade:may-act",
            "answer:actionable",
            "answer:held",
            "coordinate:dashitrade:phase9-gate",
            "applicability:open",
            "applicability:hold",
            &chain,
        )
        .unwrap();

        assert_eq!(receipt.locus.layer, ChangeLayer::Applicability);
        assert!(receipt.explanation.all_minimal_deltas_typed);
        assert_eq!(receipt.explanation.steps.len(), 1);
        assert_eq!(
            receipt.explanation.steps[0].layer,
            ChangeLayer::Applicability
        );
        assert_eq!(receipt.explanation.steps[0].cause_refs.len(), 6);
        assert!(!receipt.explanation.claims_causation_beyond_receipts);
        assert!(!receipt.explanation.predicts_outcome);
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn phase9_justification_does_not_claim_market_causation() {
        let receipt = dashi_trade_phase9_justification_boundary(
            &DashiTradePhase9Justification {
                regime_ref: "regime:positive-edge".into(),
                posture_ref: "posture:trade-normal".into(),
                actuator_ref: "actuator:bar-exec".into(),
                cost_model_ref: "cost:phase9-v2".into(),
                expected_surplus_ref: "expected:positive".into(),
                realised_surplus_ref: "realised:positive".into(),
            },
        );
        assert_eq!(receipt.justification_refs.len(), 6);
        assert!(!receipt.proves_market_causation);
        assert!(!receipt.proves_global_theory_correctness);
        assert!(!receipt.creates_claim_truth);
    }
}
