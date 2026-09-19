use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_proof_search_loop::world_observation::{
    FreshnessStatus, GetterBackend, ProvenanceClass, RetrievalStatus, WorldObservation,
};
use sensiblaw_reviewed_evidence_payment::compile_consumer_residual_stream_review_aware;
use sensiblaw_world_expansion_runtime::{
    reviewed_evidence_from_world_observation, reviewed_observation_payment_stream,
};

fn observation() -> WorldObservation {
    WorldObservation {
        request_ref: "query:mabo:P710:Q975866".into(),
        object_ref: "Q1501525".into(),
        relation_ref: "P710".into(),
        source_ref: "wikidata".into(),
        source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        content_digest_ref: "sha256:43681681a832e9d0edf09f745c7d3e71fd4cdb9fd23d4670f25e5b94827b5eba".into(),
        value_ref: "Q975866".into(),
        retrieval_status: RetrievalStatus::Retrieved,
        freshness_status: FreshnessStatus::Current,
        provenance_class: ProvenanceClass::RevisionPinnedExternalSource,
        backend: GetterBackend::SlrNative,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    }
}

fn spec() -> ConsumerSpec {
    ConsumerSpec {
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        surface_id: "surface:mabo".into(),
        requirements: vec![ConsumerRequirement {
            requirement_id: "participant-identity".into(),
            need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject),
            scope: RequirementScope::SourceManifestation(
                "wikidata:Q1501525:oldid:2333409615".into(),
            ),
        }],
    }
}

#[test]
fn mabo_p710_observation_requires_explicit_same_object_review_then_pays_identity_gap() {
    let reviewed = reviewed_evidence_from_world_observation(
        &spec(),
        "participant-identity",
        EvidenceCoordinateKind::SameObject,
        "review:mabo:P710:Q975866",
        &observation(),
    )
    .unwrap();
    assert_eq!(reviewed.coordinate, EvidenceCoordinateKind::SameObject);
    assert_eq!(reviewed.source_ref.as_deref(), Some("wikidata:Q1501525:oldid:2333409615"));
    assert_eq!(reviewed.evidence_ref, "query:mabo:P710:Q975866");
    assert!(reviewed.candidate_only);
    assert!(!reviewed.creates_semantic_authority);
    assert!(!reviewed.claim_truth_promoted);

    let payment_stream = reviewed_observation_payment_stream(
        &spec(),
        "participant-identity",
        EvidenceCoordinateKind::SameObject,
        "review:mabo:P710:Q975866",
        &observation(),
        7,
    )
    .unwrap();

    let mut next = Vec::new();
    let residual_receipt = compile_consumer_residual_stream_review_aware(
        &mut std::io::Cursor::new(payment_stream),
        &spec(),
        &mut next,
        8,
    )
    .unwrap();
    assert_eq!(residual_receipt.requirements_paid, 1);
    assert_eq!(residual_receipt.requirements_unpaid, 0);
    assert_eq!(residual_receipt.gaps_emitted, 0);
    assert_eq!(residual_receipt.obligations_emitted, 0);
}

#[test]
fn observation_does_not_infer_evidence_coordinate_or_bypass_review_scope() {
    let wrong_coordinate = reviewed_evidence_from_world_observation(
        &spec(),
        "participant-identity",
        EvidenceCoordinateKind::Authority,
        "review:mabo:P710:Q975866",
        &observation(),
    );
    assert!(wrong_coordinate.is_err());

    let mut wrong_revision = observation();
    wrong_revision.source_revision_ref = "wikidata:Q1501525:oldid:1".into();
    let wrong_scope = reviewed_evidence_from_world_observation(
        &spec(),
        "participant-identity",
        EvidenceCoordinateKind::SameObject,
        "review:mabo:P710:Q975866",
        &wrong_revision,
    );
    assert!(wrong_scope.is_err());
}
