//! M11 / S26.4 Pabai comparative-world regression.
//!
//! W0: reviewed support only -> reachable candidate.
//! W1: exact defeater admitted -> defeated.
//! W2: exact counter-defeater admitted -> reopened candidate.
//!
//! The comparative receipt identifies the reviewed input atom that changes the
//! proof route, rather than reporting only a static left/right set difference.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    compare_worlds, compile_reviewed_treatment_rerun,
    minimal_answer_changing_distinction, AnswerChangingDistinction,
    CandidateRouteStatus, ComparativeCoordinateState, ComparativeDelta,
    ComparativeDeltaKind, ComparativeDeltaRole, ComparativeQuerySlice,
    ComparativeRouteState, ComparativeWorldIr, ComparativeWorldState,
    ReviewedTreatmentCoordinate, ReviewedTreatmentRole,
};

pub const PABAI_COMPARATIVE_CONSUMER: &str = "consumer:pabai-climate-duty";
pub const PABAI_COMPARATIVE_QUERY: &str = "query:pabai:duty-route";
pub const PABAI_COMPARATIVE_ROUTE: &str = "route:pabai:comparative-duty";
pub const PABAI_COMPARATIVE_TARGET: &str = "proposition:pabai:duty-route-candidate";
pub const PABAI_SUPPORT_COORDINATE: &str = "coordinate:pabai:comparative:support";
pub const PABAI_DEFEATER_COORDINATE: &str = "coordinate:pabai:comparative:defeater";
pub const PABAI_COUNTER_COORDINATE: &str = "coordinate:pabai:comparative:counter-defeater";

fn treatment(
    treatment_ref: &str,
    coordinate_ref: &str,
    proposition_ref: &str,
    role: ReviewedTreatmentRole,
) -> ReviewedTreatmentCoordinate {
    ReviewedTreatmentCoordinate {
        treatment_ref: treatment_ref.into(),
        coordinate_ref: coordinate_ref.into(),
        source_ref: format!("source:pabai:comparative:{treatment_ref}"),
        proposition_ref: proposition_ref.into(),
        cited_authority_ref: "case:au:fca:2025:796".into(),
        role,
        reviewed: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn support() -> ReviewedTreatmentCoordinate {
    treatment(
        "support",
        PABAI_SUPPORT_COORDINATE,
        "proposition:pabai:comparative:support",
        ReviewedTreatmentRole::Support,
    )
}

fn defeater() -> ReviewedTreatmentCoordinate {
    treatment(
        "defeater",
        PABAI_DEFEATER_COORDINATE,
        "proposition:pabai:comparative:defeater",
        ReviewedTreatmentRole::Defeater,
    )
}

fn counter_defeater() -> ReviewedTreatmentCoordinate {
    // Counter-defeat uses the exact same defeated coordinate in the generic
    // treatment compiler. The comparative coordinate remains separately named
    // so the world delta can identify the newly admitted distinction itself.
    treatment(
        "counter-defeater",
        PABAI_DEFEATER_COORDINATE,
        "proposition:pabai:comparative:distinction",
        ReviewedTreatmentRole::CounterDefeater,
    )
}

fn coordinate(
    coordinate_ref: &str,
    semantic_ref: &str,
) -> ComparativeCoordinateState {
    ComparativeCoordinateState {
        coordinate_ref: coordinate_ref.into(),
        semantic_ref: semantic_ref.into(),
        review_state_ref: "reviewed".into(),
        scope_refs: BTreeSet::from([PABAI_COMPARATIVE_CONSUMER.into()]),
        authority_ref: Some("case:au:fca:2025:796".into()),
        applicability_ref: Some("candidate-applicability".into()),
        jurisdiction_ref: Some("AU".into()),
        as_at_ref: None,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn route_state(
    status: CandidateRouteStatus,
    active_defeaters: BTreeSet<String>,
    counters: BTreeSet<String>,
) -> ComparativeRouteState {
    ComparativeRouteState {
        route_ref: PABAI_COMPARATIVE_ROUTE.into(),
        status_ref: format!("{status:?}"),
        active_defeater_refs: active_defeaters,
        counter_defeater_refs: counters,
    }
}

fn world_from_treatments(
    world_ref: &str,
    treatments: &[ReviewedTreatmentCoordinate],
) -> Result<ComparativeWorldState, String> {
    let receipt = compile_reviewed_treatment_rerun(
        PABAI_COMPARATIVE_CONSUMER,
        PABAI_COMPARATIVE_ROUTE,
        PABAI_COMPARATIVE_TARGET,
        treatments,
    )?;

    let coordinates = treatments
        .iter()
        .map(|treatment| {
            (
                match treatment.role {
                    ReviewedTreatmentRole::Support => PABAI_SUPPORT_COORDINATE.to_owned(),
                    ReviewedTreatmentRole::Defeater => PABAI_DEFEATER_COORDINATE.to_owned(),
                    ReviewedTreatmentRole::CounterDefeater => PABAI_COUNTER_COORDINATE.to_owned(),
                    ReviewedTreatmentRole::AuthorityScope => treatment.coordinate_ref.clone(),
                },
                coordinate(
                    match treatment.role {
                        ReviewedTreatmentRole::Support => PABAI_SUPPORT_COORDINATE,
                        ReviewedTreatmentRole::Defeater => PABAI_DEFEATER_COORDINATE,
                        ReviewedTreatmentRole::CounterDefeater => PABAI_COUNTER_COORDINATE,
                        ReviewedTreatmentRole::AuthorityScope => &treatment.coordinate_ref,
                    },
                    &treatment.proposition_ref,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    Ok(ComparativeWorldState {
        world_ref: world_ref.into(),
        coordinates,
        routes: BTreeMap::from([(
            PABAI_COMPARATIVE_ROUTE.into(),
            route_state(
                receipt.route_status,
                receipt.active_defeater_refs.clone(),
                receipt.counter_defeater_refs.clone(),
            ),
        )]),
        residual_refs: receipt
            .typed_gaps
            .iter()
            .map(|gap| gap.gap_ref.clone())
            .collect(),
        stop_state_ref: Some(if receipt.active_defeater_refs.is_empty() {
            "search-defeaters".into()
        } else {
            "search-counter-defeaters".into()
        }),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn pabai_w0() -> Result<ComparativeWorldState, String> {
    world_from_treatments("world:pabai:w0", &[support()])
}

pub fn pabai_w1() -> Result<ComparativeWorldState, String> {
    world_from_treatments("world:pabai:w1", &[support(), defeater()])
}

pub fn pabai_w2() -> Result<ComparativeWorldState, String> {
    world_from_treatments(
        "world:pabai:w2",
        &[support(), defeater(), counter_defeater()],
    )
}

pub fn pabai_query_slice() -> ComparativeQuerySlice {
    ComparativeQuerySlice {
        query_ref: PABAI_COMPARATIVE_QUERY.into(),
        consumer_ref: PABAI_COMPARATIVE_CONSUMER.into(),
        coordinate_refs: BTreeSet::from([
            PABAI_SUPPORT_COORDINATE.into(),
            PABAI_DEFEATER_COORDINATE.into(),
            PABAI_COUNTER_COORDINATE.into(),
        ]),
        route_refs: BTreeSet::from([PABAI_COMPARATIVE_ROUTE.into()]),
        residual_refs: BTreeSet::new(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn explicit_defeater_delta() -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: "delta:pabai:w0-w1:defeater".into(),
        kind: ComparativeDeltaKind::DefeaterAdded,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(PABAI_DEFEATER_COORDINATE.into()),
        route_ref: Some(PABAI_COMPARATIVE_ROUTE.into()),
        residual_ref: None,
        before_ref: None,
        after_ref: Some("proposition:pabai:comparative:defeater".into()),
        cause_refs: BTreeSet::from(["review:pabai:defeater".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn explicit_counter_delta() -> ComparativeDelta {
    ComparativeDelta {
        delta_ref: "delta:pabai:w1-w2:counter-defeater".into(),
        kind: ComparativeDeltaKind::CounterDefeaterAdded,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(PABAI_COUNTER_COORDINATE.into()),
        route_ref: Some(PABAI_COMPARATIVE_ROUTE.into()),
        residual_ref: None,
        before_ref: None,
        after_ref: Some("proposition:pabai:comparative:distinction".into()),
        cause_refs: BTreeSet::from(["review:pabai:counter-defeater".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PabaiComparativeReceipt {
    pub w0_to_w1: ComparativeWorldIr,
    pub w0_to_w1_distinction: AnswerChangingDistinction,
    pub w1_to_w2: ComparativeWorldIr,
    pub w1_to_w2_distinction: AnswerChangingDistinction,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn run_pabai_comparative_regression() -> Result<PabaiComparativeReceipt, String> {
    let query = pabai_query_slice();
    let w0 = pabai_w0()?;
    let w1 = pabai_w1()?;
    let w2 = pabai_w2()?;

    let w0_to_w1 = compare_worlds(&w0, &w1, &query, [explicit_defeater_delta()])?;
    let w0_to_w1_distinction = minimal_answer_changing_distinction(
        &w0_to_w1,
        &query,
        "ReachableCandidate",
        "Defeated",
        |subset| {
            if subset
                .iter()
                .any(|delta| delta.kind == ComparativeDeltaKind::DefeaterAdded)
            {
                "Defeated".into()
            } else {
                "ReachableCandidate".into()
            }
        },
    )?
    .ok_or_else(|| "Pabai W0→W1 has no answer-changing distinction".to_string())?;

    let w1_to_w2 = compare_worlds(&w1, &w2, &query, [explicit_counter_delta()])?;
    let w1_to_w2_distinction = minimal_answer_changing_distinction(
        &w1_to_w2,
        &query,
        "Defeated",
        "ReachableCandidate",
        |subset| {
            if subset.iter().any(|delta| {
                delta.kind == ComparativeDeltaKind::CounterDefeaterAdded
            }) {
                "ReachableCandidate".into()
            } else {
                "Defeated".into()
            }
        },
    )?
    .ok_or_else(|| "Pabai W1→W2 has no answer-changing distinction".to_string())?;

    Ok(PabaiComparativeReceipt {
        w0_to_w1,
        w0_to_w1_distinction,
        w1_to_w2,
        w1_to_w2_distinction,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pabai_w0_w1_w2_replays_reachable_defeated_reopened() {
        let w0 = pabai_w0().unwrap();
        let w1 = pabai_w1().unwrap();
        let w2 = pabai_w2().unwrap();

        assert_eq!(
            w0.routes[PABAI_COMPARATIVE_ROUTE].status_ref,
            "ReachableCandidate"
        );
        assert_eq!(w1.routes[PABAI_COMPARATIVE_ROUTE].status_ref, "Defeated");
        assert_eq!(
            w2.routes[PABAI_COMPARATIVE_ROUTE].status_ref,
            "ReachableCandidate"
        );
    }

    #[test]
    fn pabai_comparison_identifies_exact_answer_changing_atoms() {
        let receipt = run_pabai_comparative_regression().unwrap();

        assert_eq!(
            receipt.w0_to_w1_distinction.delta_refs,
            BTreeSet::from(["delta:pabai:w0-w1:defeater".into()])
        );
        assert_eq!(
            receipt.w1_to_w2_distinction.delta_refs,
            BTreeSet::from(["delta:pabai:w1-w2:counter-defeater".into()])
        );
        assert!(receipt
            .w0_to_w1
            .changed_route_refs
            .contains(PABAI_COMPARATIVE_ROUTE));
        assert!(receipt
            .w1_to_w2
            .changed_route_refs
            .contains(PABAI_COMPARATIVE_ROUTE));
        assert!(!receipt.creates_claim_truth);
    }
}
