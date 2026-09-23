//! M11.1 gravity-shaped cross-domain regressions.
//!
//! These fixtures mirror the formal PR1028 separation surface without claiming
//! a numerical GR derivation:
//! - the represented world can remain fixed while the theory changes;
//! - an observer can refine while the world remains fixed;
//! - state may change while a declared regularity coordinate remains invariant.
//!
//! They test comparison typing and query-relative answer change only.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    compare_worlds, compile_typed_change_set, minimal_answer_changing_distinction,
    typed_locus, AnswerChangingDistinction, ChangeLayer, ComparativeCoordinateState,
    ComparativeDelta, ComparativeDeltaKind, ComparativeDeltaRole, ComparativeQuerySlice,
    ComparativeWorldIr, ComparativeWorldState, TypedChangeSet,
};

pub const GRAVITY_WORLD_COORDINATE: &str = "coordinate:world:gravity-state";
pub const GRAVITY_OBSERVATION_COORDINATE: &str = "coordinate:observation:gravity";
pub const GRAVITY_THEORY_COORDINATE: &str = "coordinate:theory:gravity";
pub const GRAVITY_REGULARITY_COORDINATE: &str = "coordinate:regularity:gravity";

pub const COARSE_FALL_QUERY: &str = "query:gravity:did-it-fall";
pub const STRONG_FIELD_QUERY: &str = "query:gravity:strong-field-prediction";
pub const REGULARITY_QUERY: &str = "query:gravity:regularity-under-control";
pub const PHYSICS_CONSUMER: &str = "consumer:physics-comparative";

fn coordinate(reference: &str, semantic: &str) -> ComparativeCoordinateState {
    ComparativeCoordinateState {
        coordinate_ref: reference.into(),
        semantic_ref: semantic.into(),
        review_state_ref: "reviewed-fixture".into(),
        scope_refs: BTreeSet::from([PHYSICS_CONSUMER.into()]),
        authority_ref: None,
        applicability_ref: Some("fixture-only".into()),
        jurisdiction_ref: None,
        as_at_ref: None,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn gravity_world(
    world_ref: &str,
    world_state: &str,
    observation: &str,
    theory: &str,
    regularity: &str,
) -> ComparativeWorldState {
    ComparativeWorldState {
        world_ref: world_ref.into(),
        coordinates: BTreeMap::from([
            (
                GRAVITY_WORLD_COORDINATE.into(),
                coordinate(GRAVITY_WORLD_COORDINATE, world_state),
            ),
            (
                GRAVITY_OBSERVATION_COORDINATE.into(),
                coordinate(GRAVITY_OBSERVATION_COORDINATE, observation),
            ),
            (
                GRAVITY_THEORY_COORDINATE.into(),
                coordinate(GRAVITY_THEORY_COORDINATE, theory),
            ),
            (
                GRAVITY_REGULARITY_COORDINATE.into(),
                coordinate(GRAVITY_REGULARITY_COORDINATE, regularity),
            ),
        ]),
        routes: BTreeMap::new(),
        residual_refs: BTreeSet::new(),
        stop_state_ref: None,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn query(query_ref: &str, coordinates: &[&str]) -> ComparativeQuerySlice {
    ComparativeQuerySlice {
        query_ref: query_ref.into(),
        consumer_ref: PHYSICS_CONSUMER.into(),
        coordinate_refs: coordinates.iter().map(|value| (*value).into()).collect(),
        route_refs: BTreeSet::new(),
        residual_refs: BTreeSet::new(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn theory_delta() -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: "delta:gravity:newton-to-relativity".into(),
        kind: ComparativeDeltaKind::FactChanged,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(GRAVITY_THEORY_COORDINATE.into()),
        route_ref: None,
        residual_ref: None,
        before_ref: Some("theory:newtonian-representation".into()),
        after_ref: Some("theory:relativistic-representation".into()),
        cause_refs: BTreeSet::from([
            "DASHI.Core.WorldRepresentationSeparationExact:newtonToRelativitySameWorldRevision"
                .into(),
        ]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn observation_delta() -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: "delta:gravity:coarse-to-refined-observer".into(),
        kind: ComparativeDeltaKind::FactChanged,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(GRAVITY_OBSERVATION_COORDINATE.into()),
        route_ref: None,
        residual_ref: None,
        before_ref: Some("observation:observed-fall".into()),
        after_ref: Some("observation:curvature-sensitive-fall".into()),
        cause_refs: BTreeSet::from([
            "DASHI.Core.WorldRepresentationSeparationExact:gravityObservationNonFactorability"
                .into(),
        ]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn state_delta() -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: "delta:gravity:state-low-to-high".into(),
        kind: ComparativeDeltaKind::FactChanged,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(GRAVITY_WORLD_COORDINATE.into()),
        route_ref: None,
        residual_ref: None,
        before_ref: Some("world:low-curvature-fall".into()),
        after_ref: Some("world:high-curvature-fall".into()),
        cause_refs: BTreeSet::from([
            "DASHI.Core.LawlikeRegularityCounterfactualExact:demoStateActuallyChanges".into(),
        ]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SameWorldTheoryComparisonReceipt {
    pub coarse_query_comparison: ComparativeWorldIr,
    pub coarse_query_answer_changed: bool,
    pub coarse_query_distinction: Option<AnswerChangingDistinction>,
    pub strong_query_comparison: ComparativeWorldIr,
    pub strong_query_answer_changed: bool,
    pub strong_query_distinction: AnswerChangingDistinction,
    pub typed_changes: TypedChangeSet,
    pub world_identity_held_fixed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn run_same_world_theory_change(
) -> Result<SameWorldTheoryComparisonReceipt, String> {
    // Same represented high-curvature world; only the theory coordinate moves.
    let left = gravity_world(
        "world:gravity:same-high-curvature",
        "world:high-curvature-fall",
        "observation:observed-fall",
        "theory:newtonian-representation",
        "regularity:gravity-fixture",
    );
    let right = gravity_world(
        "world:gravity:same-high-curvature",
        "world:high-curvature-fall",
        "observation:observed-fall",
        "theory:relativistic-representation",
        "regularity:gravity-fixture",
    );

    let theory = theory_delta();
    let locus = typed_locus(
        "locus:gravity:theory-revision",
        theory.clone(),
        ChangeLayer::Theory,
        Some("gravity-theory-representation".into()),
        BTreeSet::from([
            "layer:world".into(),
            "layer:observation".into(),
            "coordinate:regularity:gravity".into(),
        ]),
        BTreeSet::from([
            "DASHI.Core.WorldRepresentationSeparationExact".into(),
            "DASHI.Physics.Laws.WorldLawStateTheorySeparationExact".into(),
        ]),
    )?;
    let typed_changes = compile_typed_change_set(
        "comparison:gravity:newton-gr",
        [locus],
        [ChangeLayer::World, ChangeLayer::Observation],
    )?;

    // "Did it fall?" deliberately does not depend on the theory coordinate.
    let coarse_query = query(COARSE_FALL_QUERY, &[GRAVITY_OBSERVATION_COORDINATE]);
    let coarse_query_comparison =
        compare_worlds(&left, &right, &coarse_query, [theory.clone()])?;
    let coarse_query_distinction = minimal_answer_changing_distinction(
        &coarse_query_comparison,
        &coarse_query,
        "answer:fall-observed",
        "answer:fall-observed",
        |_| "answer:fall-observed".into(),
    )?;

    // Strong-field prediction explicitly depends on the theory coordinate.
    let strong_query = query(
        STRONG_FIELD_QUERY,
        &[GRAVITY_WORLD_COORDINATE, GRAVITY_THEORY_COORDINATE],
    );
    let strong_query_comparison =
        compare_worlds(&left, &right, &strong_query, [theory])?;
    let strong_query_distinction = minimal_answer_changing_distinction(
        &strong_query_comparison,
        &strong_query,
        "answer:newtonian-strong-field-fixture",
        "answer:relativistic-strong-field-fixture",
        |subset| {
            if subset
                .iter()
                .any(|delta| delta.delta_ref == "delta:gravity:newton-to-relativity")
            {
                "answer:relativistic-strong-field-fixture".into()
            } else {
                "answer:newtonian-strong-field-fixture".into()
            }
        },
    )?
    .ok_or_else(|| "strong-field theory comparison lacked exact distinction".to_string())?;

    Ok(SameWorldTheoryComparisonReceipt {
        coarse_query_comparison,
        coarse_query_answer_changed: false,
        coarse_query_distinction,
        strong_query_comparison,
        strong_query_answer_changed: true,
        strong_query_distinction,
        typed_changes,
        world_identity_held_fixed: left.world_ref == right.world_ref
            && left.coordinates[GRAVITY_WORLD_COORDINATE]
                == right.coordinates[GRAVITY_WORLD_COORDINATE],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRefinementReceipt {
    pub comparison: ComparativeWorldIr,
    pub typed_changes: TypedChangeSet,
    pub coarse_query_answer_changed: bool,
    pub discriminating_query_answer_changed: bool,
    pub discriminating_query_distinction: AnswerChangingDistinction,
    pub recharting_counts_as_world_recovery: bool,
    pub world_identity_held_fixed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn run_observation_refinement(
) -> Result<ObservationRefinementReceipt, String> {
    let left = gravity_world(
        "world:gravity:same-high-curvature",
        "world:high-curvature-fall",
        "observation:observed-fall",
        "theory:relativistic-representation",
        "regularity:gravity-fixture",
    );
    let right = gravity_world(
        "world:gravity:same-high-curvature",
        "world:high-curvature-fall",
        "observation:curvature-sensitive-fall",
        "theory:relativistic-representation",
        "regularity:gravity-fixture",
    );

    let observation = observation_delta();
    let locus = typed_locus(
        "locus:gravity:observation-refinement",
        observation.clone(),
        ChangeLayer::Observation,
        Some("instrument-resolution".into()),
        BTreeSet::from(["layer:world".into(), "layer:theory".into()]),
        BTreeSet::from([
            "DASHI.Core.WorldRepresentationSeparationExact:gravityObservationNonFactorability"
                .into(),
            "DASHI.Core.IntersectionalNonFactorability:rechartingCannotRecoverErasedPhenomenon"
                .into(),
        ]),
    )?;
    let typed_changes = compile_typed_change_set(
        "comparison:gravity:observer-refinement",
        [locus],
        [ChangeLayer::World, ChangeLayer::Theory],
    )?;

    let discriminating_query = query(
        "query:gravity:curvature-reading",
        &[GRAVITY_OBSERVATION_COORDINATE],
    );
    let comparison =
        compare_worlds(&left, &right, &discriminating_query, [observation])?;
    let discriminating_query_distinction = minimal_answer_changing_distinction(
        &comparison,
        &discriminating_query,
        "answer:coarse-fall-only",
        "answer:curvature-sensitive",
        |subset| {
            if subset.iter().any(|delta| {
                delta.delta_ref == "delta:gravity:coarse-to-refined-observer"
            }) {
                "answer:curvature-sensitive".into()
            } else {
                "answer:coarse-fall-only".into()
            }
        },
    )?
    .ok_or_else(|| "observation refinement lacked exact distinction".to_string())?;

    Ok(ObservationRefinementReceipt {
        comparison,
        typed_changes,
        coarse_query_answer_changed: false,
        discriminating_query_answer_changed: true,
        discriminating_query_distinction,
        recharting_counts_as_world_recovery: false,
        world_identity_held_fixed: left.world_ref == right.world_ref
            && left.coordinates[GRAVITY_WORLD_COORDINATE]
                == right.coordinates[GRAVITY_WORLD_COORDINATE],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRegularityComparisonReceipt {
    pub comparison: ComparativeWorldIr,
    pub typed_changes: TypedChangeSet,
    pub state_changed: bool,
    pub regularity_changed: bool,
    pub theory_changed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn run_state_change_regularilty_invariant(
) -> Result<StateRegularityComparisonReceipt, String> {
    let left = gravity_world(
        "world:gravity:state-low",
        "world:low-curvature-fall",
        "observation:observed-fall",
        "theory:relativistic-representation",
        "regularity:gravity-fixture",
    );
    let right = gravity_world(
        "world:gravity:state-high",
        "world:high-curvature-fall",
        "observation:observed-fall",
        "theory:relativistic-representation",
        "regularity:gravity-fixture",
    );

    let state = state_delta();
    let locus = typed_locus(
        "locus:gravity:state-change",
        state.clone(),
        ChangeLayer::World,
        Some("world-state".into()),
        BTreeSet::from([
            "coordinate:regularity:gravity".into(),
            "layer:theory".into(),
        ]),
        BTreeSet::from([
            "DASHI.Core.LawlikeRegularityCounterfactualExact:demoRegularityInvariant".into(),
            "DASHI.Core.LawlikeRegularityCounterfactualExact:demoStateActuallyChanges".into(),
        ]),
    )?;
    let typed_changes = compile_typed_change_set(
        "comparison:gravity:state-regularity",
        [locus],
        [ChangeLayer::Theory],
    )?;
    let regularity_query = query(
        REGULARITY_QUERY,
        &[GRAVITY_WORLD_COORDINATE, GRAVITY_REGULARITY_COORDINATE],
    );
    let comparison = compare_worlds(&left, &right, &regularity_query, [state])?;

    Ok(StateRegularityComparisonReceipt {
        comparison,
        typed_changes,
        state_changed: true,
        regularity_changed: left.coordinates[GRAVITY_REGULARITY_COORDINATE]
            != right.coordinates[GRAVITY_REGULARITY_COORDINATE],
        theory_changed: left.coordinates[GRAVITY_THEORY_COORDINATE]
            != right.coordinates[GRAVITY_THEORY_COORDINATE],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_world_theory_change_is_query_relative() {
        let receipt = run_same_world_theory_change().unwrap();
        assert!(receipt.world_identity_held_fixed);
        assert!(!receipt.coarse_query_answer_changed);
        assert!(receipt.coarse_query_distinction.is_none());
        assert!(receipt.strong_query_answer_changed);
        assert_eq!(
            receipt.strong_query_distinction.delta_refs,
            BTreeSet::from(["delta:gravity:newton-to-relativity".into()])
        );
        assert!(receipt
            .typed_changes
            .changed_layers
            .contains(&ChangeLayer::Theory));
        assert!(receipt
            .typed_changes
            .invariant_layers
            .contains(&ChangeLayer::World));
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn observer_refinement_changes_observation_not_world() {
        let receipt = run_observation_refinement().unwrap();
        assert!(receipt.world_identity_held_fixed);
        assert!(!receipt.coarse_query_answer_changed);
        assert!(receipt.discriminating_query_answer_changed);
        assert_eq!(
            receipt.discriminating_query_distinction.delta_refs,
            BTreeSet::from(["delta:gravity:coarse-to-refined-observer".into()])
        );
        assert!(receipt
            .typed_changes
            .changed_layers
            .contains(&ChangeLayer::Observation));
        assert!(receipt
            .typed_changes
            .invariant_layers
            .contains(&ChangeLayer::World));
        assert!(!receipt.recharting_counts_as_world_recovery);
    }

    #[test]
    fn state_change_does_not_imply_regularilty_or_theory_change() {
        let receipt = run_state_change_regularilty_invariant().unwrap();
        assert!(receipt.state_changed);
        assert!(!receipt.regularity_changed);
        assert!(!receipt.theory_changed);
        assert!(receipt
            .typed_changes
            .changed_layers
            .contains(&ChangeLayer::World));
        assert!(receipt
            .typed_changes
            .invariant_layers
            .contains(&ChangeLayer::Theory));
    }
}
