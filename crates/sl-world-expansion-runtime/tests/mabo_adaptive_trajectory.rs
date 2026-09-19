use sensiblaw_pg_source_store::{LatentWorldEdgeRow, LatentWorldRows};
use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_world_expansion_runtime::adaptive_campaign::{
    MaboAdaptiveDecision, MaboTypedProducerSelection,
};
use sensiblaw_world_expansion_runtime::adaptive_trajectory::{
    complete_adaptive_selection_receipt, latent_world_digest,
    proof_frontier_digest, render_adaptive_selection_receipt,
    selection_receipt_from_decision, AdaptiveSelectionReceipt,
    AdaptiveTrajectoryLinkError, link_adaptive_cycles,
};

fn world(edges: Vec<LatentWorldEdgeRow>) -> LatentWorldRows {
    LatentWorldRows {
        seed_ref: "Q1501525".into(),
        max_hops: 100,
        requested_max_hops: 100,
        visited_refs: vec!["Q1501525".into(), "Q975866".into()],
        deepest_observed_hop: 1,
        frontier_exhausted: true,
        frontier_refs: vec![],
        residual_refs: vec![],
        edges,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn frontier(residuals: Vec<ProofResidual>) -> ProofFrontier {
    ProofFrontier {
        consumer_ref: "consumer:mabo-adaptive-world-expansion".into(),
        frontier_ref: "frontier:mabo:adaptive:1".into(),
        residuals,
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

fn residual(id: &str) -> ProofResidual {
    ProofResidual {
        residual_ref: id.into(),
        proposition_ref: format!("prop:{id}"),
        producer_class_ref: "producer:test".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 1,
        dependency_refs: vec!["dep:b".into(), "dep:a".into()],
        status: ResidualStatus::Open,
    }
}

#[test]
fn trajectory_digests_are_order_invariant_over_set_like_runtime_surfaces() {
    let a = LatentWorldEdgeRow {
        from_ref: "Q1501525".into(),
        to_ref: "Q975866".into(),
        relation_ref: "context:wikidata:participant".into(),
        provenance_refs: vec!["p:b".into(), "p:a".into()],
    };
    let b = LatentWorldEdgeRow {
        from_ref: "Q1501525".into(),
        to_ref: "Q16533".into(),
        relation_ref: "context:wikidata:court".into(),
        provenance_refs: vec!["p:c".into()],
    };

    assert_eq!(
        latent_world_digest(&world(vec![a.clone(), b.clone()])),
        latent_world_digest(&world(vec![b, a]))
    );
    assert_eq!(
        proof_frontier_digest(&frontier(vec![residual("r:b"), residual("r:a")])),
        proof_frontier_digest(&frontier(vec![residual("r:a"), residual("r:b")]))
    );
}

#[test]
fn next_selection_must_link_to_the_previous_commit_and_post_commit_frontier() {
    let first = AdaptiveSelectionReceipt {
        schema_version: "mabo-adaptive-selection:v1".into(),
        cycle_index: 0,
        world_digest: "sha256:world0".into(),
        frontier_digest: "sha256:frontier0".into(),
        selected_residual_ref: "r:0".into(),
        selected_move_ref: "move:0".into(),
        selected_producer_lane_ref: "wikidata-context".into(),
        prior_commit_ref: None,
        commit_ref: Some("commit:cycle0".into()),
        review_or_payment_ref: Some("review:0".into()),
        world_delta_ref: Some("delta:0".into()),
        selection_origin: "post-commit-current-world-diagnosis".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let second = AdaptiveSelectionReceipt {
        schema_version: "mabo-adaptive-selection:v1".into(),
        cycle_index: 1,
        world_digest: "sha256:world1".into(),
        frontier_digest: "sha256:frontier1".into(),
        selected_residual_ref: "r:1".into(),
        selected_move_ref: "move:1".into(),
        selected_producer_lane_ref: "governed-legal".into(),
        prior_commit_ref: Some("commit:cycle0".into()),
        commit_ref: None,
        review_or_payment_ref: None,
        world_delta_ref: None,
        selection_origin: "post-commit-current-world-diagnosis".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let link = link_adaptive_cycles(&first, &second).unwrap();
    assert_eq!(link.previous_commit_ref, "commit:cycle0");
    assert_eq!(link.next_frontier_digest, "sha256:frontier1");
    assert!(link.fresh_post_commit_selection);
    assert!(!link.precomputed_execution_authority);
}

#[test]
fn stale_or_unlinked_next_selection_fails_closed() {
    let previous = AdaptiveSelectionReceipt {
        schema_version: "mabo-adaptive-selection:v1".into(),
        cycle_index: 0,
        world_digest: "w0".into(),
        frontier_digest: "f0".into(),
        selected_residual_ref: "r0".into(),
        selected_move_ref: "m0".into(),
        selected_producer_lane_ref: "wikidata-context".into(),
        prior_commit_ref: None,
        commit_ref: Some("commit:0".into()),
        review_or_payment_ref: None,
        world_delta_ref: None,
        selection_origin: "post-commit-current-world-diagnosis".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let mut stale = previous.clone();
    stale.cycle_index = 1;
    stale.prior_commit_ref = Some("commit:other".into());
    stale.commit_ref = None;

    assert!(matches!(
        link_adaptive_cycles(&previous, &stale),
        Err(AdaptiveTrajectoryLinkError::PriorCommitMismatch { .. })
    ));
}


#[test]
fn selection_receipt_binds_exact_world_frontier_and_typed_lane_before_commit() {
    let w = world(vec![]);
    let f = frontier(vec![residual("r:legal")]);
    let decision = MaboAdaptiveDecision::TypedProducer(MaboTypedProducerSelection {
        residual_ref: "r:legal".into(),
        residual_class: sensiblaw_proof_search_loop::world_expansion::ResidualClass::Legal,
        producer_lane: sensiblaw_proof_search_loop::world_expansion::ProducerLane::GovernedLegal,
        move_ref: "move:oalc".into(),
        source_ref: Some("source:oalc".into()),
        provider_operation_ref: "oalc:exact".into(),
        shared_dependency_gain: 3,
        diagnosis_reference: "diagnosis:legal".into(),
    });

    let pending = selection_receipt_from_decision(
        7,
        Some("commit:6".into()),
        &w,
        &f,
        &decision,
    );
    assert_eq!(pending.world_digest, latent_world_digest(&w));
    assert_eq!(pending.frontier_digest, proof_frontier_digest(&f));
    assert_eq!(pending.selected_residual_ref, "r:legal");
    assert_eq!(pending.selected_move_ref, "move:oalc");
    assert_eq!(pending.selected_producer_lane_ref, "governed-legal");
    assert_eq!(pending.prior_commit_ref.as_deref(), Some("commit:6"));
    assert!(pending.commit_ref.is_none());

    let completed = complete_adaptive_selection_receipt(
        &pending,
        "commit:7",
        "review:7",
        "delta:7",
    );
    assert_eq!(completed.commit_ref.as_deref(), Some("commit:7"));
    assert_eq!(completed.review_or_payment_ref.as_deref(), Some("review:7"));
    assert_eq!(completed.world_delta_ref.as_deref(), Some("delta:7"));

    let rendered = render_adaptive_selection_receipt(&completed);
    assert!(rendered.contains("cycle_index\t7"));
    assert!(rendered.contains("frontier_digest\tsha256:"));
    assert!(rendered.contains("selected_producer_lane\tgoverned-legal"));
    assert!(rendered.contains("commit_ref\tcommit:7"));
}
