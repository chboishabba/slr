//! S21.1 live Yindjibarndi case-run weld.
//!
//! This is the first production composition of:
//!   source-grounded reviewed treatments
//!   -> exact shared-world joins
//!   -> generic reviewed-treatment compiler
//!   -> support/defeat/counter-defeat rerun
//!   -> genuinely unpaid typed gaps.
//!
//! It contains no Yindjibarndi-specific defeat algorithm.  The only
//! matter-specific content is the reviewed source packet and the target route
//! identity.  Treatment compilation and counter-defeat matching are generic.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{
    compile_reviewed_treatment_rerun, generic_mabo_join_is_rejected,
    reviewed_yindjibarndi_authority_joins, yindjibarndi_primary_source_packet,
    yindjibarndi_reviewed_dependency_slice, AdversarialRerunReceipt,
    EmpiricalArgumentRole, ReviewedTreatmentCoordinate, ReviewedTreatmentRole,
    SharedCoordinateKind, SharedWorld, SharedWorldCoordinate,
    MABO_ACQUISITION_DISTINCTION_COORDINATE, YINDJIBARNDI_CONSUMER,
    YUNUPINGU_ACQUISITION_COORDINATE,
};

pub const YINDJIBARNDI_ROUTE: &str =
    "route:yindjibarndi:compensable-acquisition-candidate";
pub const YINDJIBARNDI_TARGET: &str =
    "proposition:yindjibarndi:compensable-acquisition-candidate";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseRunStopReason {
    RouteReachableCandidate,
    RouteDefeated,
    Contested,
    NeedsAuthority,
    NeedsFact,
    NeedsApplicability,
    ExplicitlyUnresolved,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YindjibarndiMachineReceipt {
    pub consumer_ref: String,
    pub route_ref: String,
    pub source_refs: BTreeSet<String>,
    pub proposition_refs: BTreeSet<String>,
    pub shared_world_coordinate_refs: BTreeSet<String>,
    pub reviewed_join_refs: BTreeSet<String>,
    pub paid_atom_refs: BTreeSet<String>,
    pub active_defeater_refs: BTreeSet<String>,
    pub counter_defeater_refs: BTreeSet<String>,
    pub wrong_type_gap_refs: BTreeSet<String>,
    pub authority_gap_refs: BTreeSet<String>,
    pub fact_gap_refs: BTreeSet<String>,
    pub applicability_gap_refs: BTreeSet<String>,
    pub remaining_reopening_cut_refs: BTreeSet<String>,
    pub research_demand_refs: BTreeSet<String>,
    pub stop_reason: CaseRunStopReason,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedWorldReuseDecision {
    pub coordinate_ref: String,
    pub exact_dependency: bool,
    pub reviewed_join: bool,
    pub reusable: bool,
    pub wrong_type: bool,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YindjibarndiLiveCaseRun {
    pub shared_world: SharedWorld,
    pub reuse_decisions: Vec<SharedWorldReuseDecision>,
    pub adversarial: AdversarialRerunReceipt,
    pub generic_mabo_join_rejected: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn reviewed_role(role: EmpiricalArgumentRole) -> ReviewedTreatmentRole {
    match role {
        EmpiricalArgumentRole::Support => ReviewedTreatmentRole::Support,
        EmpiricalArgumentRole::Defeater => ReviewedTreatmentRole::Defeater,
        EmpiricalArgumentRole::CounterDefeater => ReviewedTreatmentRole::CounterDefeater,
        EmpiricalArgumentRole::AuthorityScope => ReviewedTreatmentRole::AuthorityScope,
    }
}

pub fn yindjibarndi_reviewed_treatments() -> Result<Vec<ReviewedTreatmentCoordinate>, String> {
    yindjibarndi_primary_source_packet()
        .into_iter()
        .enumerate()
        .map(|(index, coordinate)| {
            coordinate.validate()?;
            Ok(ReviewedTreatmentCoordinate {
                treatment_ref: format!("yindjibarndi:{index}:{}", coordinate.proposition_ref),
                coordinate_ref: coordinate.coordinate_ref,
                source_ref: coordinate.source_ref,
                proposition_ref: coordinate.proposition_ref,
                cited_authority_ref: coordinate.cited_authority_ref,
                role: reviewed_role(coordinate.role),
                reviewed: true,
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            })
        })
        .collect()
}

fn shared_coordinate(
    coordinate_ref: &str,
    semantic_ref: &str,
    source_refs: BTreeSet<String>,
) -> SharedWorldCoordinate {
    SharedWorldCoordinate {
        coordinate_ref: coordinate_ref.into(),
        kind: SharedCoordinateKind::Treatment,
        semantic_ref: semantic_ref.into(),
        source_revision_refs: source_refs.clone(),
        provenance_refs: source_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn build_yindjibarndi_shared_world() -> Result<SharedWorld, String> {
    let packet = yindjibarndi_primary_source_packet();
    let mut world = SharedWorld::new();
    world.set_dependency_slice(yindjibarndi_reviewed_dependency_slice())?;

    for (coordinate_ref, semantic_ref) in [
        (
            YUNUPINGU_ACQUISITION_COORDINATE,
            "semantic:yunupingu:s51xxxi-native-title-acquisition",
        ),
        (
            MABO_ACQUISITION_DISTINCTION_COORDINATE,
            "semantic:mabo-no2:brennan-60-acquisition-distinction",
        ),
    ] {
        let source_refs = packet
            .iter()
            .filter(|item| item.coordinate_ref == coordinate_ref)
            .map(|item| item.source_ref.clone())
            .collect::<BTreeSet<_>>();
        world.admit_coordinate(shared_coordinate(
            coordinate_ref,
            semantic_ref,
            source_refs,
        ))?;
    }

    for witness in reviewed_yindjibarndi_authority_joins()? {
        world.admit_reviewed_join(witness)?;
    }
    Ok(world)
}

pub fn run_yindjibarndi_live_case() -> Result<YindjibarndiLiveCaseRun, String> {
    let world = build_yindjibarndi_shared_world()?;
    let slice = world
        .dependency_slices
        .get(YINDJIBARNDI_CONSUMER)
        .ok_or_else(|| "Yindjibarndi dependency slice missing".to_string())?;

    let reuse_decisions = [
        YUNUPINGU_ACQUISITION_COORDINATE,
        MABO_ACQUISITION_DISTINCTION_COORDINATE,
    ]
    .into_iter()
    .map(|coordinate_ref| SharedWorldReuseDecision {
        coordinate_ref: coordinate_ref.into(),
        exact_dependency: slice.requires(coordinate_ref),
        reviewed_join: world
            .reviewed_joins
            .contains_key(&(YINDJIBARNDI_CONSUMER.into(), coordinate_ref.into())),
        reusable: world.coordinate_paid_for_consumer(
            YINDJIBARNDI_CONSUMER,
            coordinate_ref,
        ),
        wrong_type: false,
        candidate_only: true,
        creates_claim_truth: false,
    })
    .collect::<Vec<_>>();

    let adversarial = compile_reviewed_treatment_rerun(
        YINDJIBARNDI_CONSUMER,
        YINDJIBARNDI_ROUTE,
        YINDJIBARNDI_TARGET,
        &yindjibarndi_reviewed_treatments()?,
    )?;

    Ok(YindjibarndiLiveCaseRun {
        shared_world: world,
        reuse_decisions,
        adversarial,
        generic_mabo_join_rejected: generic_mabo_join_is_rejected(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn compile_yindjibarndi_machine_receipt(
    run: &YindjibarndiLiveCaseRun,
) -> YindjibarndiMachineReceipt {
    let treatments = yindjibarndi_primary_source_packet();
    let stop_reason = if !run
        .adversarial
        .summary
        .applicability_gap_refs
        .is_empty()
    {
        CaseRunStopReason::NeedsApplicability
    } else {
        match run.adversarial.route_status {
            crate::CandidateRouteStatus::ReachableCandidate => {
                CaseRunStopReason::RouteReachableCandidate
            }
            crate::CandidateRouteStatus::Defeated => CaseRunStopReason::RouteDefeated,
            crate::CandidateRouteStatus::Contested => CaseRunStopReason::Contested,
            crate::CandidateRouteStatus::AuthorityBlocked => CaseRunStopReason::NeedsAuthority,
            crate::CandidateRouteStatus::MissingAtoms => CaseRunStopReason::ExplicitlyUnresolved,
            crate::CandidateRouteStatus::ExplicitlyUnresolved => {
                CaseRunStopReason::ExplicitlyUnresolved
            }
        }
    };

    YindjibarndiMachineReceipt {
        consumer_ref: YINDJIBARNDI_CONSUMER.into(),
        route_ref: run.adversarial.route_ref.clone(),
        source_refs: treatments.iter().map(|item| item.source_ref.clone()).collect(),
        proposition_refs: treatments
            .iter()
            .map(|item| item.proposition_ref.clone())
            .collect(),
        shared_world_coordinate_refs: run
            .shared_world
            .coordinates
            .keys()
            .cloned()
            .collect(),
        reviewed_join_refs: run
            .shared_world
            .reviewed_joins
            .values()
            .map(|join| join.witness_ref.clone())
            .collect(),
        paid_atom_refs: run.adversarial.summary.paid_atom_refs.clone(),
        active_defeater_refs: run.adversarial.active_defeater_refs.clone(),
        counter_defeater_refs: run.adversarial.counter_defeater_refs.clone(),
        wrong_type_gap_refs: run.adversarial.summary.wrong_type_gap_refs.clone(),
        authority_gap_refs: run.adversarial.summary.authority_gap_refs.clone(),
        fact_gap_refs: run.adversarial.summary.fact_gap_refs.clone(),
        applicability_gap_refs: run
            .adversarial
            .summary
            .applicability_gap_refs
            .clone(),
        remaining_reopening_cut_refs: run
            .adversarial
            .summary
            .remaining_reopening_cut_refs
            .clone(),
        research_demand_refs: run
            .adversarial
            .next_demands
            .iter()
            .map(|demand| demand.demand_ref.clone())
            .collect(),
        stop_reason,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CandidateRouteStatus, TypedRerunGapKind};

    #[test]
    fn exact_yunupingu_and_mabo_coordinates_are_reused() {
        let run = run_yindjibarndi_live_case().unwrap();
        assert_eq!(run.reuse_decisions.len(), 2);
        assert!(run.reuse_decisions.iter().all(|decision| {
            decision.exact_dependency && decision.reviewed_join && decision.reusable
        }));
        assert!(run.generic_mabo_join_rejected);
    }

    #[test]
    fn mabo_counter_defeater_does_not_guess_away_yunupingu_scope_objections() {
        let run = run_yindjibarndi_live_case().unwrap();
        assert_eq!(run.adversarial.route_status, CandidateRouteStatus::Defeated);
        assert_eq!(run.adversarial.counter_defeater_refs.len(), 1);
        assert_eq!(run.adversarial.active_defeater_refs.len(), 2);
        assert!(run
            .adversarial
            .typed_gaps
            .iter()
            .any(|gap| gap.kind == TypedRerunGapKind::Applicability));
        assert!(!run.adversarial.next_demands.is_empty());
        assert!(!run.creates_claim_truth);
        let receipt = compile_yindjibarndi_machine_receipt(&run);
        assert_eq!(receipt.stop_reason, CaseRunStopReason::NeedsApplicability);
        assert_eq!(receipt.shared_world_coordinate_refs.len(), 2);
        assert_eq!(receipt.reviewed_join_refs.len(), 2);
        assert!(!receipt.research_demand_refs.is_empty());
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn live_run_uses_generic_treatment_compiler_not_case_specific_reducer() {
        let treatments = yindjibarndi_reviewed_treatments().unwrap();
        let direct = compile_reviewed_treatment_rerun(
            YINDJIBARNDI_CONSUMER,
            YINDJIBARNDI_ROUTE,
            YINDJIBARNDI_TARGET,
            &treatments,
        )
        .unwrap();
        let live = run_yindjibarndi_live_case().unwrap();
        assert_eq!(direct.route_status, live.adversarial.route_status);
        assert_eq!(
            direct.active_defeater_refs,
            live.adversarial.active_defeater_refs
        );
        assert_eq!(
            direct.counter_defeater_refs,
            live.adversarial.counter_defeater_refs
        );
    }
}
