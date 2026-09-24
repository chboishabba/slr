//! S19.10 empirical affected-consumer propagation for Yindjibarndi.
//!
//! This is the last M8 live systems property: a reviewed change to an exact
//! shared coordinate invalidates/reopens only consumers whose dependency slice
//! mentions that exact coordinate.  Related consumers are not fanned out by
//! citation, ontology, or doctrinal adjacency alone.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    affected_consumer_recompute_plan, build_yindjibarndi_shared_world,
    ConsumerDependencySlice, AffectedConsumerRecomputePlan,
    MABO_ACQUISITION_DISTINCTION_COORDINATE,
    YUNUPINGU_ACQUISITION_COORDINATE,
};

pub const SHARED_NATIVE_TITLE_AUTHORITY_CONSUMER: &str =
    "consumer:shared-native-title-authorities";
pub const MABO_ACQUISITION_CONSUMER: &str = "consumer:mabo-acquisition-distinction";
pub const PABAI_CONTROL_CONSUMER: &str = "consumer:pabai-climate-duty";
pub const MUNKARA_CONTROL_CONSUMER: &str = "consumer:munkara-tipakalippa";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YindjibarndiAffectedConsumerReceipt {
    pub yunupingu_plan: AffectedConsumerRecomputePlan,
    pub mabo_plan: AffectedConsumerRecomputePlan,
    pub unaffected_control_consumers: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn slice(
    consumer_ref: &str,
    required: impl IntoIterator<Item = &'static str>,
) -> ConsumerDependencySlice {
    ConsumerDependencySlice {
        consumer_ref: consumer_ref.into(),
        required_coordinate_refs: required.into_iter().map(str::to_owned).collect(),
        optional_coordinate_refs: BTreeSet::new(),
        dependency_slice_ref: format!("slice:{consumer_ref}:m8-affected-consumer"),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn run_yindjibarndi_affected_consumer_experiment(
) -> Result<YindjibarndiAffectedConsumerReceipt, String> {
    let mut world = build_yindjibarndi_shared_world()?;

    world.set_dependency_slice(slice(
        SHARED_NATIVE_TITLE_AUTHORITY_CONSUMER,
        [YUNUPINGU_ACQUISITION_COORDINATE],
    ))?;
    world.set_dependency_slice(slice(
        MABO_ACQUISITION_CONSUMER,
        [MABO_ACQUISITION_DISTINCTION_COORDINATE],
    ))?;

    // These are deliberately nearby/related consumers whose current slices do
    // not mention either changed coordinate.
    world.set_dependency_slice(slice(PABAI_CONTROL_CONSUMER, []))?;
    world.set_dependency_slice(slice(MUNKARA_CONTROL_CONSUMER, []))?;

    let yunupingu_plan = affected_consumer_recompute_plan(
        &world,
        [YUNUPINGU_ACQUISITION_COORDINATE.to_owned()],
    )?;
    let mabo_plan = affected_consumer_recompute_plan(
        &world,
        [MABO_ACQUISITION_DISTINCTION_COORDINATE.to_owned()],
    )?;

    let unaffected_control_consumers = BTreeSet::from([
        PABAI_CONTROL_CONSUMER.to_owned(),
        MUNKARA_CONTROL_CONSUMER.to_owned(),
    ]);

    for control in &unaffected_control_consumers {
        if yunupingu_plan.consumer_refs.contains(control)
            || mabo_plan.consumer_refs.contains(control)
        {
            return Err(format!(
                "non-dependent control consumer {control} was incorrectly invalidated"
            ));
        }
    }

    Ok(YindjibarndiAffectedConsumerReceipt {
        yunupingu_plan,
        mabo_plan,
        unaffected_control_consumers,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn exact_recompute_causes(
    receipt: &YindjibarndiAffectedConsumerReceipt,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = receipt.yunupingu_plan.cause_refs_by_consumer.clone();
    for (consumer, causes) in &receipt.mabo_plan.cause_refs_by_consumer {
        out.entry(consumer.clone())
            .or_default()
            .extend(causes.iter().cloned());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::YINDJIBARNDI_CONSUMER;

    #[test]
    fn yunupingu_delta_recomputes_only_exact_dependents() {
        let receipt = run_yindjibarndi_affected_consumer_experiment().unwrap();
        assert_eq!(
            receipt.yunupingu_plan.consumer_refs,
            BTreeSet::from([
                YINDJIBARNDI_CONSUMER.to_owned(),
                SHARED_NATIVE_TITLE_AUTHORITY_CONSUMER.to_owned(),
            ])
        );
        assert!(!receipt
            .yunupingu_plan
            .consumer_refs
            .contains(PABAI_CONTROL_CONSUMER));
        assert!(!receipt
            .yunupingu_plan
            .consumer_refs
            .contains(MUNKARA_CONTROL_CONSUMER));
    }

    #[test]
    fn mabo_delta_recomputes_only_exact_dependents() {
        let receipt = run_yindjibarndi_affected_consumer_experiment().unwrap();
        assert_eq!(
            receipt.mabo_plan.consumer_refs,
            BTreeSet::from([
                YINDJIBARNDI_CONSUMER.to_owned(),
                MABO_ACQUISITION_CONSUMER.to_owned(),
            ])
        );
        assert!(!receipt
            .mabo_plan
            .consumer_refs
            .contains(PABAI_CONTROL_CONSUMER));
        assert!(!receipt
            .mabo_plan
            .consumer_refs
            .contains(MUNKARA_CONTROL_CONSUMER));
    }

    #[test]
    fn combined_receipt_preserves_exact_cause_per_consumer() {
        let receipt = run_yindjibarndi_affected_consumer_experiment().unwrap();
        let causes = exact_recompute_causes(&receipt);
        assert_eq!(
            causes.get(YINDJIBARNDI_CONSUMER).unwrap(),
            &BTreeSet::from([
                YUNUPINGU_ACQUISITION_COORDINATE.to_owned(),
                MABO_ACQUISITION_DISTINCTION_COORDINATE.to_owned(),
            ])
        );
        assert_eq!(
            causes.get(SHARED_NATIVE_TITLE_AUTHORITY_CONSUMER).unwrap(),
            &BTreeSet::from([YUNUPINGU_ACQUISITION_COORDINATE.to_owned()])
        );
        assert_eq!(
            causes.get(MABO_ACQUISITION_CONSUMER).unwrap(),
            &BTreeSet::from([MABO_ACQUISITION_DISTINCTION_COORDINATE.to_owned()])
        );
        assert!(!receipt.creates_claim_truth);
    }
}
