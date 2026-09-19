use std::collections::BTreeSet;

use sensiblaw_pg_source_store::{
    collapse_discovery_campaign_identity_classes, DiscoveryCampaignIdentityRow,
};

fn row(object_ref: &str, identity_class_ref: &str, discovery_parent_ref: &str) -> DiscoveryCampaignIdentityRow {
    DiscoveryCampaignIdentityRow {
        object_ref: object_ref.into(),
        identity_class_ref: identity_class_ref.into(),
        discovery_parent_ref: discovery_parent_ref.into(),
    }
}

#[test]
fn campaign_identity_classes_are_transitively_scoped_to_seed_lineage() {
    let classes = collapse_discovery_campaign_identity_classes(
        "Q1501525",
        &[
            row("Q1", "world-object:q1", "Q1501525"),
            row("Q2", "world-object:q2", "Q1"),
            row("case:[1992]-HCA-23", "world-object:mabo-case", "Q1501525"),
            row("Q999", "world-object:unrelated", "Q888"),
        ],
    )
    .unwrap();

    assert_eq!(
        classes,
        BTreeSet::from([
            "world-object:mabo-case".to_string(),
            "world-object:q1".to_string(),
            "world-object:q2".to_string(),
        ])
    );
}

#[test]
fn campaign_identity_scope_does_not_count_same_class_twice_or_follow_unreachable_cycles() {
    let classes = collapse_discovery_campaign_identity_classes(
        "Q1501525",
        &[
            row("Q1", "world-object:shared", "Q1501525"),
            row("https://example.invalid/q1", "world-object:shared", "Q1"),
            row("Q77", "world-object:cycle-a", "Q78"),
            row("Q78", "world-object:cycle-b", "Q77"),
        ],
    )
    .unwrap();

    assert_eq!(classes, BTreeSet::from(["world-object:shared".to_string()]));
}
