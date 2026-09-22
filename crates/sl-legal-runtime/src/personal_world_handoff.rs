//! M9 / S22-S23 personal world -> professional consumer runtime.
//!
//! This is a consumer projection over the existing shared-world carrier, not a
//! second personal-data store.  Scope is an admission gate: a coordinate may be
//! present in the personal world while being unavailable to one or more
//! professional consumers.
//!
//! The first bounded fixture uses synthetic coordinate identities matching the
//! ITIR Smart-Journal contract classes:
//! - reviewed event
//! - reviewed document
//! - reviewed factual proposition
//! - private hypothesis
//! - not-ready journal material
//!
//! Private/not-ready material remains usable by the personal consumer while
//! professional consumers receive only coordinates admitted by both their
//! dependency slice and share scope.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    affected_consumer_recompute_plan, ConsumerDependencySlice, SharedCoordinateKind,
    SharedWorld, SharedWorldCoordinate,
};

pub const PERSONAL_CONSUMER: &str = "consumer:personal-journal";
pub const LAWYER_CONSUMER: &str = "consumer:professional:lawyer";
pub const DOCTOR_CONSUMER: &str = "consumer:professional:doctor";
pub const ADVOCATE_CONSUMER: &str = "consumer:professional:advocate";
pub const REGULATOR_CONSUMER: &str = "consumer:professional:regulator";

pub const EVENT_COORDINATE: &str = "coordinate:personal:event:reviewed";
pub const DOCUMENT_COORDINATE: &str = "coordinate:personal:document:reviewed";
pub const FACT_COORDINATE: &str = "coordinate:personal:fact:reviewed";
pub const PRIVATE_HYPOTHESIS_COORDINATE: &str =
    "coordinate:personal:hypothesis:private";
pub const NOT_READY_COORDINATE: &str = "coordinate:personal:journal:not-ready";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ShareClass {
    PersonalOnly,
    SelectedProfessional,
    Lawyer,
    Doctor,
    Advocate,
    Regulator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopedPersonalCoordinate {
    pub coordinate: SharedWorldCoordinate,
    pub share_classes: BTreeSet<ShareClass>,
    pub reviewed: bool,
    pub explicitly_not_ready: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfessionalProjectionReceipt {
    pub consumer_ref: String,
    pub included_coordinate_refs: BTreeSet<String>,
    pub excluded_coordinate_refs: BTreeSet<String>,
    pub excluded_reason_refs: BTreeMap<String, String>,
    pub unresolved_coordinate_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalWorldHandoffExperiment {
    pub world: SharedWorld,
    pub scoped_coordinates: BTreeMap<String, ScopedPersonalCoordinate>,
    pub projections: BTreeMap<String, ProfessionalProjectionReceipt>,
    pub affected_consumers_by_coordinate: BTreeMap<String, BTreeSet<String>>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn coordinate(
    coordinate_ref: &str,
    kind: SharedCoordinateKind,
    semantic_ref: &str,
) -> SharedWorldCoordinate {
    SharedWorldCoordinate {
        coordinate_ref: coordinate_ref.into(),
        kind,
        semantic_ref: semantic_ref.into(),
        source_revision_refs: BTreeSet::from(["revision:personal-world:v1".into()]),
        provenance_refs: BTreeSet::from(["provenance:personal-world:reviewed-fixture".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn scoped(
    coordinate: SharedWorldCoordinate,
    share_classes: impl IntoIterator<Item = ShareClass>,
    reviewed: bool,
    explicitly_not_ready: bool,
) -> ScopedPersonalCoordinate {
    ScopedPersonalCoordinate {
        coordinate,
        share_classes: share_classes.into_iter().collect(),
        reviewed,
        explicitly_not_ready,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn slice(
    consumer_ref: &str,
    required: impl IntoIterator<Item = &'static str>,
    optional: impl IntoIterator<Item = &'static str>,
) -> ConsumerDependencySlice {
    ConsumerDependencySlice {
        consumer_ref: consumer_ref.into(),
        required_coordinate_refs: required.into_iter().map(str::to_owned).collect(),
        optional_coordinate_refs: optional.into_iter().map(str::to_owned).collect(),
        dependency_slice_ref: format!("slice:{consumer_ref}:m9"),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn consumer_share_class(consumer_ref: &str) -> Option<ShareClass> {
    match consumer_ref {
        LAWYER_CONSUMER => Some(ShareClass::Lawyer),
        DOCTOR_CONSUMER => Some(ShareClass::Doctor),
        ADVOCATE_CONSUMER => Some(ShareClass::Advocate),
        REGULATOR_CONSUMER => Some(ShareClass::Regulator),
        _ => None,
    }
}

pub fn build_personal_world() -> Result<(SharedWorld, BTreeMap<String, ScopedPersonalCoordinate>), String> {
    let mut world = SharedWorld::new();

    let scoped_coordinates = [
        scoped(
            coordinate(
                EVENT_COORDINATE,
                SharedCoordinateKind::Event,
                "semantic:personal:event:reviewed",
            ),
            [
                ShareClass::SelectedProfessional,
                ShareClass::Lawyer,
                ShareClass::Doctor,
                ShareClass::Advocate,
            ],
            true,
            false,
        ),
        scoped(
            coordinate(
                DOCUMENT_COORDINATE,
                SharedCoordinateKind::Evidence,
                "semantic:personal:document:reviewed",
            ),
            [
                ShareClass::SelectedProfessional,
                ShareClass::Lawyer,
                ShareClass::Doctor,
                ShareClass::Regulator,
            ],
            true,
            false,
        ),
        scoped(
            coordinate(
                FACT_COORDINATE,
                SharedCoordinateKind::Proposition,
                "semantic:personal:fact:reviewed",
            ),
            [
                ShareClass::SelectedProfessional,
                ShareClass::Lawyer,
                ShareClass::Doctor,
                ShareClass::Advocate,
                ShareClass::Regulator,
            ],
            true,
            false,
        ),
        scoped(
            coordinate(
                PRIVATE_HYPOTHESIS_COORDINATE,
                SharedCoordinateKind::Claim,
                "semantic:personal:hypothesis:private",
            ),
            [ShareClass::PersonalOnly],
            false,
            false,
        ),
        scoped(
            coordinate(
                NOT_READY_COORDINATE,
                SharedCoordinateKind::Context,
                "semantic:personal:journal:not-ready",
            ),
            [ShareClass::PersonalOnly],
            false,
            true,
        ),
    ]
    .into_iter()
    .map(|item| (item.coordinate.coordinate_ref.clone(), item))
    .collect::<BTreeMap<_, _>>();

    for item in scoped_coordinates.values() {
        world.admit_coordinate(item.coordinate.clone())?;
    }

    world.set_dependency_slice(slice(
        PERSONAL_CONSUMER,
        [
            EVENT_COORDINATE,
            DOCUMENT_COORDINATE,
            FACT_COORDINATE,
            PRIVATE_HYPOTHESIS_COORDINATE,
            NOT_READY_COORDINATE,
        ],
        [],
    ))?;

    world.set_dependency_slice(slice(
        LAWYER_CONSUMER,
        [EVENT_COORDINATE, DOCUMENT_COORDINATE, FACT_COORDINATE],
        [],
    ))?;
    world.set_dependency_slice(slice(
        DOCTOR_CONSUMER,
        [EVENT_COORDINATE, DOCUMENT_COORDINATE, FACT_COORDINATE],
        [],
    ))?;
    world.set_dependency_slice(slice(
        ADVOCATE_CONSUMER,
        [EVENT_COORDINATE, FACT_COORDINATE],
        [],
    ))?;
    world.set_dependency_slice(slice(
        REGULATOR_CONSUMER,
        [DOCUMENT_COORDINATE, FACT_COORDINATE],
        [],
    ))?;

    Ok((world, scoped_coordinates))
}

pub fn project_personal_world_for_consumer(
    world: &SharedWorld,
    scoped_coordinates: &BTreeMap<String, ScopedPersonalCoordinate>,
    consumer_ref: &str,
) -> Result<ProfessionalProjectionReceipt, String> {
    let slice = world
        .dependency_slices
        .get(consumer_ref)
        .ok_or_else(|| format!("consumer {consumer_ref} has no dependency slice"))?;

    let role = consumer_share_class(consumer_ref);
    let mut included = BTreeSet::new();
    let mut excluded = BTreeSet::new();
    let mut reasons = BTreeMap::new();
    let mut unresolved = BTreeSet::new();

    for (coordinate_ref, item) in scoped_coordinates {
        if consumer_ref == PERSONAL_CONSUMER {
            included.insert(coordinate_ref.clone());
            if !item.reviewed || item.explicitly_not_ready {
                unresolved.insert(coordinate_ref.clone());
            }
            continue;
        }

        if item.explicitly_not_ready {
            excluded.insert(coordinate_ref.clone());
            reasons.insert(coordinate_ref.clone(), "explicitly-not-ready".into());
            continue;
        }

        if item.share_classes.contains(&ShareClass::PersonalOnly) {
            excluded.insert(coordinate_ref.clone());
            reasons.insert(coordinate_ref.clone(), "personal-only-scope".into());
            continue;
        }

        if !item.reviewed {
            excluded.insert(coordinate_ref.clone());
            reasons.insert(coordinate_ref.clone(), "unreviewed-personal-material".into());
            continue;
        }

        if !slice.mentions(coordinate_ref) {
            excluded.insert(coordinate_ref.clone());
            reasons.insert(coordinate_ref.clone(), "outside-consumer-dependency-slice".into());
            continue;
        }

        let share_allowed = role.is_some_and(|role| {
            item.share_classes.contains(&ShareClass::SelectedProfessional)
                && item.share_classes.contains(&role)
        });

        if !share_allowed {
            excluded.insert(coordinate_ref.clone());
            reasons.insert(coordinate_ref.clone(), "scope-not-admitted-for-role".into());
            continue;
        }

        included.insert(coordinate_ref.clone());
    }

    Ok(ProfessionalProjectionReceipt {
        consumer_ref: consumer_ref.into(),
        included_coordinate_refs: included,
        excluded_coordinate_refs: excluded,
        excluded_reason_refs: reasons,
        unresolved_coordinate_refs: unresolved,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn run_personal_world_handoff_experiment(
) -> Result<PersonalWorldHandoffExperiment, String> {
    let (world, scoped_coordinates) = build_personal_world()?;
    let mut projections = BTreeMap::new();
    for consumer in [
        PERSONAL_CONSUMER,
        LAWYER_CONSUMER,
        DOCTOR_CONSUMER,
        ADVOCATE_CONSUMER,
        REGULATOR_CONSUMER,
    ] {
        projections.insert(
            consumer.into(),
            project_personal_world_for_consumer(&world, &scoped_coordinates, consumer)?,
        );
    }

    let mut affected_consumers_by_coordinate = BTreeMap::new();
    for coordinate_ref in [EVENT_COORDINATE, DOCUMENT_COORDINATE, FACT_COORDINATE] {
        let plan =
            affected_consumer_recompute_plan(&world, [coordinate_ref.to_owned()])?;
        affected_consumers_by_coordinate
            .insert(coordinate_ref.to_owned(), plan.consumer_refs);
    }

    Ok(PersonalWorldHandoffExperiment {
        world,
        scoped_coordinates,
        projections,
        affected_consumers_by_coordinate,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_consumer_retains_private_and_not_ready_material() {
        let run = run_personal_world_handoff_experiment().unwrap();
        let personal = run.projections.get(PERSONAL_CONSUMER).unwrap();
        assert!(personal
            .included_coordinate_refs
            .contains(PRIVATE_HYPOTHESIS_COORDINATE));
        assert!(personal
            .included_coordinate_refs
            .contains(NOT_READY_COORDINATE));
        assert!(personal
            .unresolved_coordinate_refs
            .contains(PRIVATE_HYPOTHESIS_COORDINATE));
        assert!(personal
            .unresolved_coordinate_refs
            .contains(NOT_READY_COORDINATE));
    }

    #[test]
    fn professional_consumers_do_not_inherit_private_hypothesis_or_not_ready_material() {
        let run = run_personal_world_handoff_experiment().unwrap();
        for consumer in [
            LAWYER_CONSUMER,
            DOCTOR_CONSUMER,
            ADVOCATE_CONSUMER,
            REGULATOR_CONSUMER,
        ] {
            let projection = run.projections.get(consumer).unwrap();
            assert!(!projection
                .included_coordinate_refs
                .contains(PRIVATE_HYPOTHESIS_COORDINATE));
            assert!(!projection
                .included_coordinate_refs
                .contains(NOT_READY_COORDINATE));
            assert!(projection
                .excluded_coordinate_refs
                .contains(PRIVATE_HYPOTHESIS_COORDINATE));
            assert!(projection
                .excluded_coordinate_refs
                .contains(NOT_READY_COORDINATE));
            assert_eq!(
                projection
                    .excluded_reason_refs
                    .get(PRIVATE_HYPOTHESIS_COORDINATE)
                    .map(String::as_str),
                Some("personal-only-scope")
            );
            assert_eq!(
                projection
                    .excluded_reason_refs
                    .get(NOT_READY_COORDINATE)
                    .map(String::as_str),
                Some("explicitly-not-ready")
            );
        }
    }

    #[test]
    fn role_slices_are_distinct() {
        let run = run_personal_world_handoff_experiment().unwrap();
        let lawyer = run.projections.get(LAWYER_CONSUMER).unwrap();
        let advocate = run.projections.get(ADVOCATE_CONSUMER).unwrap();
        let regulator = run.projections.get(REGULATOR_CONSUMER).unwrap();

        assert!(lawyer.included_coordinate_refs.contains(EVENT_COORDINATE));
        assert!(lawyer.included_coordinate_refs.contains(DOCUMENT_COORDINATE));

        assert!(advocate.included_coordinate_refs.contains(EVENT_COORDINATE));
        assert!(!advocate
            .included_coordinate_refs
            .contains(DOCUMENT_COORDINATE));

        assert!(!regulator.included_coordinate_refs.contains(EVENT_COORDINATE));
        assert!(regulator
            .included_coordinate_refs
            .contains(DOCUMENT_COORDINATE));
    }

    #[test]
    fn reviewed_fact_delta_recomputes_only_declared_dependents() {
        let run = run_personal_world_handoff_experiment().unwrap();
        assert_eq!(
            run.affected_consumers_by_coordinate
                .get(FACT_COORDINATE)
                .unwrap(),
            &BTreeSet::from([
                PERSONAL_CONSUMER.to_owned(),
                LAWYER_CONSUMER.to_owned(),
                DOCTOR_CONSUMER.to_owned(),
                ADVOCATE_CONSUMER.to_owned(),
                REGULATOR_CONSUMER.to_owned(),
            ])
        );
        assert!(!run.creates_claim_truth);
    }

    #[test]
    fn event_delta_does_not_recompute_regulator_without_dependency() {
        let run = run_personal_world_handoff_experiment().unwrap();
        let affected = run
            .affected_consumers_by_coordinate
            .get(EVENT_COORDINATE)
            .unwrap();
        assert!(affected.contains(PERSONAL_CONSUMER));
        assert!(affected.contains(LAWYER_CONSUMER));
        assert!(affected.contains(DOCTOR_CONSUMER));
        assert!(affected.contains(ADVOCATE_CONSUMER));
        assert!(!affected.contains(REGULATOR_CONSUMER));
    }
}
