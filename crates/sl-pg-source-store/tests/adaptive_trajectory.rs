use sensiblaw_pg_source_store::{
    adaptive_trajectory_row, AdaptiveTrajectoryInput, AdaptiveTrajectoryError,
};

fn input(cycle_index: usize, prior: Option<&str>, commit: &str) -> AdaptiveTrajectoryInput {
    AdaptiveTrajectoryInput {
        campaign_ref: "campaign:mabo-adaptive".into(),
        cycle_index,
        world_digest: format!("sha256:world{cycle_index}"),
        frontier_digest: format!("sha256:frontier{cycle_index}"),
        selected_residual_ref: format!("residual:{cycle_index}"),
        selected_move_ref: format!("move:{cycle_index}"),
        selected_producer_lane_ref: "wikidata-context-expansion".into(),
        prior_commit_ref: prior.map(str::to_owned),
        commit_ref: commit.into(),
        review_or_payment_ref: format!("review:{cycle_index}"),
        world_delta_ref: format!("delta:{cycle_index}"),
        selection_origin: "post-commit-current-world-diagnosis".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn trajectory_row_is_deterministic_and_non_promoting() {
    let row = adaptive_trajectory_row(&input(1, Some("commit:0"), "commit:1")).unwrap();
    assert_eq!(row.cycle_index, 1);
    assert_eq!(row.prior_commit_ref.as_deref(), Some("commit:0"));
    assert_eq!(row.commit_ref, "commit:1");
    assert_eq!(row.receipt_sha256.len(), 64);
    assert!(row.candidate_only);
    assert!(!row.creates_semantic_authority);
    assert!(!row.applicability_promoted);
    assert!(!row.claim_truth_promoted);
    assert_eq!(
        row.receipt_sha256,
        adaptive_trajectory_row(&input(1, Some("commit:0"), "commit:1"))
            .unwrap()
            .receipt_sha256
    );
}

#[test]
fn cycle_zero_has_no_prior_and_later_cycles_require_one() {
    assert!(adaptive_trajectory_row(&input(0, None, "commit:0")).is_ok());

    assert_eq!(
        adaptive_trajectory_row(&input(0, Some("commit:old"), "commit:0")),
        Err(AdaptiveTrajectoryError::UnexpectedPriorCommitAtCycleZero)
    );
    assert_eq!(
        adaptive_trajectory_row(&input(1, None, "commit:1")),
        Err(AdaptiveTrajectoryError::MissingPriorCommit)
    );
}

#[test]
fn trajectory_receipt_cannot_promote_or_claim_queued_selection() {
    let mut bad = input(1, Some("commit:0"), "commit:1");
    bad.selection_origin = "precomputed-queue".into();
    assert_eq!(
        adaptive_trajectory_row(&bad),
        Err(AdaptiveTrajectoryError::InvalidSelectionOrigin)
    );

    let mut bad = input(1, Some("commit:0"), "commit:1");
    bad.claim_truth_promoted = true;
    assert_eq!(
        adaptive_trajectory_row(&bad),
        Err(AdaptiveTrajectoryError::TrajectoryMayNotPromote)
    );
}
