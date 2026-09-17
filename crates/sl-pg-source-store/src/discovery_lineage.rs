#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_proof_search_loop::world_expansion::ProducerLane;
    use sensiblaw_proof_search_loop::world_expansion_reentry::DiscoveryLineageReceipt;

    fn lineage() -> DiscoveryLineageReceipt {
        DiscoveryLineageReceipt {
            object_ref: "case:[1992]-HCA-23".into(),
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            selected_candidate_ref: "oalc:case:[1992]-HCA-23".into(),
            producer_lane: ProducerLane::GovernedLegal,
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
            pnf_world_disambiguation_ref: "pnf-world:mabo:5".into(),
            expected_residual_contraction: 4,
            observed_residual_contraction: 2,
            new_residual_refs: vec!["residual:mabo:case-follow".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            receipt_authority: "candidate_world_expansion_only",
        }
    }

    #[test]
    fn lineage_row_preserves_discovery_and_observed_delta_coordinates() {
        let row = discovery_lineage_row(&lineage()).unwrap();
        assert_eq!(row.object_ref, "case:[1992]-HCA-23");
        assert_eq!(row.discovery_parent_ref, "Q1501525");
        assert_eq!(row.triggering_residual_ref, "residual:mabo:authority-source");
        assert_eq!(row.producer_lane_ref, "governed-legal");
        assert_eq!(row.source_revision_ref, "oalc:[1992]-HCA-23:sha256:abc");
        assert_eq!(row.expected_residual_contraction, 4);
        assert_eq!(row.observed_residual_contraction, 2);
        assert_eq!(row.new_residual_refs, vec!["residual:mabo:case-follow"]);
        assert!(row.candidate_only);
        assert!(!row.creates_semantic_authority);
        assert!(!row.applicability_promoted);
        assert!(!row.claim_truth_promoted);
        assert_eq!(row.receipt_sha256.len(), 64);
    }

    #[test]
    fn same_lineage_is_hash_stable_and_changed_delta_changes_receipt() {
        let first = discovery_lineage_row(&lineage()).unwrap();
        let second = discovery_lineage_row(&lineage()).unwrap();
        assert_eq!(first.receipt_sha256, second.receipt_sha256);

        let mut changed = lineage();
        changed.observed_residual_contraction = 3;
        let changed = discovery_lineage_row(&changed).unwrap();
        assert_ne!(first.receipt_sha256, changed.receipt_sha256);
    }

    #[test]
    fn promoted_or_non_candidate_lineage_fails_closed() {
        let mut bad = lineage();
        bad.candidate_only = false;
        assert_eq!(
            discovery_lineage_row(&bad),
            Err(DiscoveryLineageError::LineageMustRemainCandidateOnly)
        );
        bad.candidate_only = true;
        bad.creates_semantic_authority = true;
        assert_eq!(
            discovery_lineage_row(&bad),
            Err(DiscoveryLineageError::LineageMayNotPromote)
        );
    }
}
