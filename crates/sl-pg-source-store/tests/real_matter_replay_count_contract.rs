use sensiblaw_pg_source_store::{
    checked_replay_count, replay_count_satisfies, RealMatterReplayAcceptanceError,
};

#[test]
fn a_stronger_stored_replay_count_satisfies_a_weaker_rerun_request() {
    assert!(replay_count_satisfies(3, 2));
    assert!(replay_count_satisfies(2, 2));
    assert!(!replay_count_satisfies(2, 3));
}

#[test]
fn replay_count_numeric_overflow_is_not_reported_as_insufficient_replay() {
    assert!(matches!(
        checked_replay_count(usize::MAX),
        Err(RealMatterReplayAcceptanceError::ReplayCountOverflow)
    ));
}
