//! M11 / S26.5 same-world personal/professional fibre comparison.
//!
//! The canonical personal world is unchanged. Only the consumer projection
//! differs. This receipt explains the visibility delta without claiming that
//! either projection is the uniquely "true" world.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    run_personal_world_handoff_experiment, ProfessionalProjectionReceipt,
    ADVOCATE_CONSUMER, DOCTOR_CONSUMER, LAWYER_CONSUMER, NOT_READY_COORDINATE,
    PERSONAL_CONSUMER, PRIVATE_HYPOTHESIS_COORDINATE, REGULATOR_CONSUMER,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerFibreComparison {
    pub world_ref: String,
    pub left_consumer_ref: String,
    pub right_consumer_ref: String,
    pub shared_visible_coordinate_refs: BTreeSet<String>,
    pub left_only_visible_coordinate_refs: BTreeSet<String>,
    pub right_only_visible_coordinate_refs: BTreeSet<String>,
    pub right_scope_blocked_coordinate_refs: BTreeSet<String>,
    pub right_not_ready_coordinate_refs: BTreeSet<String>,
    pub right_dependency_irrelevant_coordinate_refs: BTreeSet<String>,
    pub right_unreviewed_coordinate_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn comparison(
    world_ref: &str,
    left: &ProfessionalProjectionReceipt,
    right: &ProfessionalProjectionReceipt,
) -> ConsumerFibreComparison {
    let shared_visible_coordinate_refs = left
        .included_coordinate_refs
        .intersection(&right.included_coordinate_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let left_only_visible_coordinate_refs = left
        .included_coordinate_refs
        .difference(&right.included_coordinate_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let right_only_visible_coordinate_refs = right
        .included_coordinate_refs
        .difference(&left.included_coordinate_refs)
        .cloned()
        .collect::<BTreeSet<_>>();

    let by_reason = |reason: &str| {
        right
            .excluded_reason_refs
            .iter()
            .filter_map(|(coordinate_ref, candidate_reason)| {
                (candidate_reason == reason).then_some(coordinate_ref.clone())
            })
            .collect::<BTreeSet<_>>()
    };

    ConsumerFibreComparison {
        world_ref: world_ref.into(),
        left_consumer_ref: left.consumer_ref.clone(),
        right_consumer_ref: right.consumer_ref.clone(),
        shared_visible_coordinate_refs,
        left_only_visible_coordinate_refs,
        right_only_visible_coordinate_refs,
        right_scope_blocked_coordinate_refs: by_reason("personal-only-scope")
            .union(&by_reason("scope-not-admitted-for-role"))
            .cloned()
            .collect(),
        right_not_ready_coordinate_refs: by_reason("explicitly-not-ready"),
        right_dependency_irrelevant_coordinate_refs: by_reason(
            "outside-consumer-dependency-slice",
        ),
        right_unreviewed_coordinate_refs: by_reason("unreviewed-personal-material"),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalProfessionalComparativeReceipt {
    pub world_ref: String,
    pub personal_to_lawyer: ConsumerFibreComparison,
    pub personal_to_doctor: ConsumerFibreComparison,
    pub personal_to_advocate: ConsumerFibreComparison,
    pub personal_to_regulator: ConsumerFibreComparison,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn run_personal_professional_comparison(
) -> Result<PersonalProfessionalComparativeReceipt, String> {
    let run = run_personal_world_handoff_experiment()?;
    let world_ref = "world:personal-professional:m9".to_owned();
    let personal = run
        .projections
        .get(PERSONAL_CONSUMER)
        .ok_or_else(|| "personal projection missing".to_string())?;

    let projection = |consumer_ref: &str| -> Result<&ProfessionalProjectionReceipt, String> {
        run.projections
            .get(consumer_ref)
            .ok_or_else(|| format!("projection missing for {consumer_ref}"))
    };

    Ok(PersonalProfessionalComparativeReceipt {
        world_ref: world_ref.clone(),
        personal_to_lawyer: comparison(&world_ref, personal, projection(LAWYER_CONSUMER)?),
        personal_to_doctor: comparison(&world_ref, personal, projection(DOCTOR_CONSUMER)?),
        personal_to_advocate: comparison(&world_ref, personal, projection(ADVOCATE_CONSUMER)?),
        personal_to_regulator: comparison(&world_ref, personal, projection(REGULATOR_CONSUMER)?),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn fibre_matrix(
    receipt: &PersonalProfessionalComparativeReceipt,
) -> BTreeMap<String, BTreeSet<String>> {
    BTreeMap::from([
        (
            receipt.personal_to_lawyer.right_consumer_ref.clone(),
            receipt
                .personal_to_lawyer
                .shared_visible_coordinate_refs
                .clone(),
        ),
        (
            receipt.personal_to_doctor.right_consumer_ref.clone(),
            receipt
                .personal_to_doctor
                .shared_visible_coordinate_refs
                .clone(),
        ),
        (
            receipt.personal_to_advocate.right_consumer_ref.clone(),
            receipt
                .personal_to_advocate
                .shared_visible_coordinate_refs
                .clone(),
        ),
        (
            receipt.personal_to_regulator.right_consumer_ref.clone(),
            receipt
                .personal_to_regulator
                .shared_visible_coordinate_refs
                .clone(),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DOCUMENT_COORDINATE, EVENT_COORDINATE, FACT_COORDINATE};

    #[test]
    fn same_world_yields_distinct_legitimate_professional_fibres() {
        let receipt = run_personal_professional_comparison().unwrap();
        assert_eq!(
            receipt.personal_to_lawyer.world_ref,
            receipt.personal_to_regulator.world_ref
        );
        assert!(receipt
            .personal_to_lawyer
            .shared_visible_coordinate_refs
            .contains(EVENT_COORDINATE));
        assert!(receipt
            .personal_to_lawyer
            .shared_visible_coordinate_refs
            .contains(DOCUMENT_COORDINATE));
        assert!(!receipt
            .personal_to_regulator
            .shared_visible_coordinate_refs
            .contains(EVENT_COORDINATE));
        assert!(receipt
            .personal_to_regulator
            .shared_visible_coordinate_refs
            .contains(DOCUMENT_COORDINATE));
        assert!(receipt
            .personal_to_regulator
            .shared_visible_coordinate_refs
            .contains(FACT_COORDINATE));
    }

    #[test]
    fn private_and_not_ready_material_are_explained_not_erased() {
        let receipt = run_personal_professional_comparison().unwrap();
        for fibre in [
            &receipt.personal_to_lawyer,
            &receipt.personal_to_doctor,
            &receipt.personal_to_advocate,
            &receipt.personal_to_regulator,
        ] {
            assert!(fibre
                .left_only_visible_coordinate_refs
                .contains(PRIVATE_HYPOTHESIS_COORDINATE));
            assert!(fibre
                .left_only_visible_coordinate_refs
                .contains(NOT_READY_COORDINATE));
            assert!(fibre
                .right_scope_blocked_coordinate_refs
                .contains(PRIVATE_HYPOTHESIS_COORDINATE));
            assert!(fibre
                .right_not_ready_coordinate_refs
                .contains(NOT_READY_COORDINATE));
        }
        assert!(!receipt.creates_claim_truth);
    }
}
