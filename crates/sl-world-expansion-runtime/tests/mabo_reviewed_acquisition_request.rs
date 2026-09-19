use sensiblaw_world_expansion_runtime::reviewed_campaign::{
    reviewed_acquisition_request, MaboReviewedCyclePreparationError,
};
use sensiblaw_proof_search_loop::world_expansion::ResidualClass;
use sensiblaw_world_expansion_runtime::{
    MaboIdentityDiagnosisRow, MaboIdentityReviewAssignment, MaboPlannedIdentityReview,
};

fn planned() -> MaboPlannedIdentityReview {
    MaboPlannedIdentityReview {
        row: MaboIdentityDiagnosisRow {
            representation_ref: "Q975866".into(),
            relation_type_refs: vec!["context:wikidata:participant".into()],
            source_revision_refs: vec!["wikidata:Q1501525:oldid:2333409615".into()],
            requirement_id: "world-identity:Q975866".into(),
            residual_ref: "residual:mabo:world-identity:Q975866".into(),
            residual_class: ResidualClass::Identity,
            discovery_route_ref: "residual-observation",
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        assignment: MaboIdentityReviewAssignment {
            representation_ref: "Q975866".into(),
            identity_class_ref: "world-object:eddie-mabo".into(),
            review_ref: "review:mabo:identity:eddie".into(),
        },
    }
}

#[test]
fn reviewed_row_derives_exact_pinned_provider_request() {
    let request = reviewed_acquisition_request(&planned()).unwrap();
    assert_eq!(request.source_qid, "Q1501525");
    assert_eq!(request.revision_id, 2_333_409_615);
    assert_eq!(request.property_ref, "P710");
    assert_eq!(request.target_ref, "Q975866");
    assert_eq!(request.source_revision_ref, "wikidata:Q1501525:oldid:2333409615");
}

#[test]
fn ambiguous_revision_fails_closed_but_duplicate_reviewed_relations_choose_a_deterministic_witness() {
    let mut multiple_revisions = planned();
    multiple_revisions
        .row
        .source_revision_refs
        .push("wikidata:Q1501525:oldid:2333409616".into());
    assert_eq!(
        reviewed_acquisition_request(&multiple_revisions),
        Err(MaboReviewedCyclePreparationError::AmbiguousSourceRevision)
    );

    let mut multiple_relations = planned();
    multiple_relations
        .row
        .relation_type_refs
        .push("context:wikidata:judge".into());
    let request = reviewed_acquisition_request(&multiple_relations).unwrap();
    assert_eq!(request.source_revision_ref, "wikidata:Q1501525:oldid:2333409615");
    assert_eq!(request.target_ref, "Q975866");
    assert_eq!(request.property_ref, "P1594");
}

#[test]
fn arbitrary_semantic_label_cannot_be_replayed_as_a_provider_property() {
    let mut value = planned();
    value.row.relation_type_refs = vec!["context:wikidata:made-up".into()];
    assert_eq!(
        reviewed_acquisition_request(&value),
        Err(MaboReviewedCyclePreparationError::MissingDiagnosedRelation)
    );
}

#[test]
fn same_object_review_does_not_change_parent_relation_semantics() {
    let request = reviewed_acquisition_request(&planned()).unwrap();
    assert_eq!(request.target_ref, planned().assignment.representation_ref);
    assert_eq!(request.property_ref, "P710");
}
