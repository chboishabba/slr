#[path = "../src/reviewed_campaign.rs"]
mod reviewed_campaign;

use reviewed_campaign::{parse_wikidata_revision_ref, prepare_reviewed_mabo_identity_cycle};
use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::ResidualClass;
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::entity_revision_receipt_from_rdf;
use sensiblaw_world_expansion_runtime::{
    MaboConsumerDiagnosis, MaboIdentityDiagnosisRow, MaboIdentityReviewAssignment,
    MaboPlannedIdentityReview,
};

fn diagnosis_row() -> MaboIdentityDiagnosisRow {
    MaboIdentityDiagnosisRow {
        representation_ref: "Q975866".into(),
        relation_type_refs: vec!["context:wikidata:P710".into()],
        source_revision_refs: vec!["wikidata:Q1501525:oldid:2333409615".into()],
        requirement_id: "world-identity:Q975866".into(),
        residual_ref: "residual:mabo:world-identity:Q975866".into(),
        residual_class: ResidualClass::Identity,
        discovery_route_ref: "residual-observation",
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn diagnosis() -> MaboConsumerDiagnosis {
    let row = diagnosis_row();
    MaboConsumerDiagnosis {
        consumer_spec: ConsumerSpec {
            consumer_id: "consumer:mabo-context-world-identity".into(),
            surface_id: "surface:mabo:reviewed-context-world".into(),
            requirements: vec![ConsumerRequirement {
                requirement_id: row.requirement_id.clone(),
                need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject),
                scope: RequirementScope::SourceManifestation(
                    "wikidata:Q1501525:oldid:2333409615".into(),
                ),
            }],
        },
        residuals: vec![ProofResidual {
            residual_ref: row.residual_ref.clone(),
            proposition_ref: "mabo:world-identity:Q975866".into(),
            producer_class_ref: "producer:world-expansion".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: 100,
            dependency_refs: vec![],
            status: ResidualStatus::Open,
        }],
        rows: vec![row],
        reviewed_context_edges_considered: 1,
        known_identity_representations: 0,
        duplicate_target_edges: 0,
        out_of_scope_or_wrong_type_edges: 0,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn planned() -> MaboPlannedIdentityReview {
    MaboPlannedIdentityReview {
        row: diagnosis_row(),
        assignment: MaboIdentityReviewAssignment {
            representation_ref: "Q975866".into(),
            identity_class_ref: "world-object:eddie-mabo".into(),
            review_ref: "review:mabo:identity:eddie".into(),
        },
    }
}

fn route() -> RouteCandidate {
    RouteCandidate {
        candidate_id: "wikidata:Q1501525:P710:Q975866".into(),
        producer: ProducerFamily::IdentitySource,
        route_family: RouteFamily::WikidataProperty,
        source_ref: "Q1501525".into(),
        target_ref: "Q975866".into(),
        property_ref: "P710".into(),
        cross_language_gap_coverage: 0,
        source_surface_support: 0,
        root_qid_support: 1,
        typed_property_support: 1,
        route_specificity: 3,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    }
}

#[test]
fn exact_wikidata_revision_ref_is_parsed_fail_closed() {
    assert_eq!(
        parse_wikidata_revision_ref("wikidata:Q1501525:oldid:2333409615").unwrap(),
        ("Q1501525".to_owned(), 2_333_409_615)
    );
    assert!(parse_wikidata_revision_ref("wikidata:Q1501525:latest").is_err());
    assert!(parse_wikidata_revision_ref("wikidata:not-a-qid:oldid:2333409615").is_err());
    assert!(parse_wikidata_revision_ref("wikidata:Q1501525:oldid:0").is_err());
}

#[test]
fn reviewed_identity_assignment_prepares_real_pinned_cycle_and_payment() {
    let acquired = entity_revision_receipt_from_rdf(
        "Q1501525",
        2_333_409_615,
        b"<rdf:RDF>fixture</rdf:RDF>".to_vec(),
    )
    .unwrap();
    let prepared = prepare_reviewed_mabo_identity_cycle(
        &diagnosis(),
        &planned(),
        &acquired,
        &route(),
        7,
    )
    .unwrap();

    assert_eq!(prepared.prepared.routing.residual_ref, "residual:mabo:world-identity:Q975866");
    assert_eq!(prepared.prepared.routing.residual_class, ResidualClass::Identity);
    assert_eq!(prepared.prepared.candidates.len(), 1);
    assert_eq!(prepared.prepared.candidates[0].object_ref, "Q975866");
    assert_eq!(
        prepared.prepared.candidates[0].source_revision_ref.as_deref(),
        Some("wikidata:Q1501525:oldid:2333409615")
    );
    assert_eq!(
        prepared.prepared.identity_resolution.identity.identity_class_ref,
        "world-object:eddie-mabo"
    );
    assert!(prepared.prepared.identity_resolution.identity.contains_representation("Q975866"));
    assert_eq!(
        prepared.prepared.observation.source_revision_ref,
        "wikidata:Q1501525:oldid:2333409615"
    );
    assert!(!prepared.reviewed_payment_wire.is_empty());
}

#[test]
fn preparation_rejects_route_or_revision_not_owned_by_diagnosis_row() {
    let acquired = entity_revision_receipt_from_rdf(
        "Q1501525",
        2_333_409_615,
        b"<rdf:RDF>fixture</rdf:RDF>".to_vec(),
    )
    .unwrap();
    let mut wrong_target = route();
    wrong_target.target_ref = "Q36074".into();
    assert!(prepare_reviewed_mabo_identity_cycle(
        &diagnosis(),
        &planned(),
        &acquired,
        &wrong_target,
        7,
    )
    .is_err());

    let wrong_revision = entity_revision_receipt_from_rdf(
        "Q1501525",
        2_333_409_616,
        b"<rdf:RDF>fixture</rdf:RDF>".to_vec(),
    )
    .unwrap();
    assert!(prepare_reviewed_mabo_identity_cycle(
        &diagnosis(),
        &planned(),
        &wrong_revision,
        &route(),
        7,
    )
    .is_err());
}
