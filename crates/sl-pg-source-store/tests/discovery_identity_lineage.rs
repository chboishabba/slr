use sensiblaw_pg_source_store::{discovery_lineage_row, DiscoveryLineageInput};

#[test]
fn discovery_lineage_persists_identity_class_separately_from_representation() {
    let input = DiscoveryLineageInput {
        object_ref: "https://en.wikipedia.org/wiki/Eddie_Mabo".into(),
        identity_class_ref: "world-object:eddie-mabo".into(),
        discovery_parent_ref: "Q1501525".into(),
        triggering_residual_ref: "residual:mabo:participant-identity".into(),
        selected_candidate_ref: "wikipedia:eddie-mabo".into(),
        producer_lane_ref: "wikipedia-context".into(),
        source_revision_ref: "etag:eddie".into(),
        pnf_world_disambiguation_ref: "identity-resolution:mabo:eddie".into(),
        expected_residual_contraction: 2,
        observed_residual_contraction: 1,
        new_residual_refs: vec![],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "candidate_world_expansion_only".into(),
    };
    let row = discovery_lineage_row(&input).unwrap();
    assert_eq!(row.object_ref, "https://en.wikipedia.org/wiki/Eddie_Mabo");
    assert_eq!(row.identity_class_ref, "world-object:eddie-mabo");
    assert_ne!(row.object_ref, row.identity_class_ref);
}
