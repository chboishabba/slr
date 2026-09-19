use sensiblaw_proof_search_loop::world_expansion::ProducerLane;
use sensiblaw_proof_search_loop::world_expansion_reentry::DiscoveryLineageReceipt;
use sensiblaw_world_expansion_runtime::discovery_lineage_input;

fn lineage() -> DiscoveryLineageReceipt {
    DiscoveryLineageReceipt {
        object_ref: "case:[1992]-HCA-23".into(),
        discovery_parent_ref: "Q1501525".into(),
        triggering_residual_ref: "residual:mabo:authority-source".into(),
        selected_candidate_ref: "oalc:case:[1992]-HCA-23".into(),
        producer_lane: ProducerLane::GovernedLegal,
        source_revision_ref: "oalc:snapshot:20260629:hca:1992:23".into(),
        pnf_world_disambiguation_ref: "pnf-world:mabo:brennan:page-39:radical-title".into(),
        expected_residual_contraction: 4,
        observed_residual_contraction: 2,
        new_residual_refs: vec!["residual:mabo:cited-case:milirrpum".into()],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "candidate_world_expansion_only",
    }
}

#[test]
fn projection_preserves_identity_and_semantic_lineage_coordinates() {
    let input = discovery_lineage_input(&lineage(), "world-object:mabo-case-1992-hca-23").unwrap();
    assert_eq!(input.object_ref, "case:[1992]-HCA-23");
    assert_eq!(input.identity_class_ref, "world-object:mabo-case-1992-hca-23");
    assert_eq!(input.discovery_parent_ref, "Q1501525");
    assert_eq!(input.triggering_residual_ref, "residual:mabo:authority-source");
    assert_eq!(input.selected_candidate_ref, "oalc:case:[1992]-HCA-23");
    assert_eq!(input.producer_lane_ref, "governed-legal");
    assert_eq!(input.source_revision_ref, "oalc:snapshot:20260629:hca:1992:23");
    assert_eq!(input.expected_residual_contraction, 4);
    assert_eq!(input.observed_residual_contraction, 2);
    assert_eq!(input.new_residual_refs, vec!["residual:mabo:cited-case:milirrpum"]);
    assert!(input.candidate_only);
    assert!(!input.creates_semantic_authority);
    assert!(!input.applicability_promoted);
    assert!(!input.claim_truth_promoted);
}

#[test]
fn projection_rejects_empty_identity_class_instead_of_falling_back_to_representation() {
    let error = discovery_lineage_input(&lineage(), "").unwrap_err();
    assert_eq!(error.to_string(), "reviewed identity class is required for durable recurrent lineage");
}
