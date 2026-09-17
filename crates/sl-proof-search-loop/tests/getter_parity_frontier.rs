use sensiblaw_proof_search_loop::frontier::ResidualStatus;
use sensiblaw_proof_search_loop::world_observation::{
    compare_observations, getter_parity_to_proof_residual, FreshnessStatus, GetterBackend,
    ProvenanceClass, RetrievalStatus, WorldObservation,
};

fn observation(backend: GetterBackend, value_ref: &str) -> WorldObservation {
    WorldObservation {
        request_ref: "query:mabo:P710".into(),
        object_ref: "Q1501525".into(),
        relation_ref: "P710".into(),
        source_ref: "wikidata".into(),
        source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        content_digest_ref: "sha256:mabo-wikidata".into(),
        value_ref: value_ref.into(),
        retrieval_status: RetrievalStatus::Retrieved,
        freshness_status: FreshnessStatus::Current,
        provenance_class: ProvenanceClass::RevisionPinnedExternalSource,
        backend,
        candidate_only: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn mismatch_becomes_open_existing_frontier_residual_shape() {
    let parity = compare_observations(
        &observation(GetterBackend::SlrNative, "Q975866"),
        &observation(GetterBackend::LeanInterop, "Q36074"),
    );
    let residual = getter_parity_to_proof_residual(
        &parity,
        "mabo:proposition:participant-identity",
        Some("AU"),
        80,
    ).unwrap();
    assert_eq!(residual.status, ResidualStatus::Open);
    assert_eq!(residual.producer_class_ref, "producer:getter-parity");
    assert!(residual.residual_ref.starts_with("getter-parity:"));
}

#[test]
fn agreement_creates_no_residual() {
    let parity = compare_observations(
        &observation(GetterBackend::SlrNative, "Q975866"),
        &observation(GetterBackend::LeanInterop, "Q975866"),
    );
    assert!(getter_parity_to_proof_residual(
        &parity,
        "mabo:proposition:participant-identity",
        Some("AU"),
        80,
    ).is_none());
}
