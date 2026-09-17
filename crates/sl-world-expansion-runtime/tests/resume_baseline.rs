use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_pg_source_store::DiscoveryIdentityBaseline;
use sensiblaw_world_expansion_runtime::{
    identity_coherence_baseline, mabo_remaining_world_expansion_policy,
    durable_mabo_campaign_total,
};

fn baseline(count: usize) -> DiscoveryIdentityBaseline {
    let mut identity_class_refs = BTreeSet::new();
    let mut representation_identity_class_refs = BTreeMap::new();
    for index in 0..count {
        let class_ref = format!("world-object:{index}");
        let object_ref = format!("Q{index}");
        identity_class_refs.insert(class_ref.clone());
        representation_identity_class_refs.insert(object_ref, class_ref);
    }
    DiscoveryIdentityBaseline {
        identity_class_refs,
        representation_identity_class_refs,
    }
}

#[test]
fn remaining_target_is_100_minus_durable_identity_classes() {
    let policy = mabo_remaining_world_expansion_policy(&baseline(12));
    assert_eq!(policy.target_novel_objects, 88);
    assert_eq!(policy.minimum_expected_residual_contraction, 1);
}

#[test]
fn baseline_at_or_above_target_requires_no_more_novel_admissions() {
    assert_eq!(mabo_remaining_world_expansion_policy(&baseline(100)).target_novel_objects, 0);
    assert_eq!(mabo_remaining_world_expansion_policy(&baseline(101)).target_novel_objects, 0);
}

#[test]
fn campaign_total_adds_new_run_novelty_to_unique_durable_baseline() {
    assert_eq!(durable_mabo_campaign_total(&baseline(12), 7), 19);
}

#[test]
fn pg_baseline_projects_without_changing_identity_coordinates() {
    let pg = baseline(2);
    let guard = identity_coherence_baseline(&pg);
    assert_eq!(guard.identity_class_refs, pg.identity_class_refs);
    assert_eq!(
        guard.representation_identity_class_refs,
        pg.representation_identity_class_refs
    );
}
