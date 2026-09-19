use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    mabo_remaining_adaptive_world_expansion_policy, mabo_target_complete,
};

#[test]
fn adaptive_remaining_target_uses_seed_scoped_campaign_count_not_global_identity_count() {
    let policy = mabo_remaining_adaptive_world_expansion_policy(12);
    assert_eq!(policy.target_novel_objects, 88);
    assert_eq!(policy.minimum_expected_residual_contraction, 1);
    assert!(!mabo_target_complete(12));
}

#[test]
fn adaptive_target_is_complete_only_at_one_hundred_seed_scoped_identity_classes() {
    assert!(!mabo_target_complete(99));
    assert!(mabo_target_complete(100));
    assert!(mabo_target_complete(101));
    assert_eq!(
        mabo_remaining_adaptive_world_expansion_policy(101).target_novel_objects,
        0
    );
}
