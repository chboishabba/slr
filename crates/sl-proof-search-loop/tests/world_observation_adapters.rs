use sensiblaw_proof_search_loop::world_observation::{GetterBackend, ObservationParity};
use sensiblaw_proof_search_loop::world_observation_adapters::wikidata_property_observation;
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::entity_revision_receipt_from_rdf;

fn route_with(
    producer: ProducerFamily,
    property_ref: &str,
    target_ref: &str,
) -> RouteCandidate {
    RouteCandidate {
        candidate_id: format!("wikidata:Q1501525:{property_ref}:{target_ref}"),
        producer,
        route_family: RouteFamily::WikidataProperty,
        source_ref: "Q1501525".into(),
        target_ref: target_ref.into(),
        property_ref: property_ref.into(),
        cross_language_gap_coverage: 0,
        source_surface_support: 1,
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

fn route() -> RouteCandidate {
    route_with(ProducerFamily::IdentitySource, "P710", "Q975866")
}

fn acquired() -> sensiblaw_wikimedia_candidate_provider::WikidataEntityRevisionReceipt {
    entity_revision_receipt_from_rdf(
        "Q1501525",
        2333409615,
        b"<rdf:RDF>mabo fixture</rdf:RDF>".to_vec(),
    )
    .unwrap()
}

#[test]
fn mabo_native_wikidata_route_normalizes_to_golden_observation_shape() {
    let acquired = acquired();
    let observation = wikidata_property_observation(
        "query:mabo:P710",
        &acquired,
        &route(),
        GetterBackend::SlrNative,
    )
    .unwrap();
    assert_eq!(observation.object_ref, "Q1501525");
    assert_eq!(observation.relation_ref, "P710");
    assert_eq!(observation.value_ref, "Q975866");
    assert_eq!(
        observation.source_revision_ref,
        "wikidata:Q1501525:oldid:2333409615"
    );
    assert!(observation.content_digest_ref.starts_with("sha256:"));
    assert!(observation.validate().is_ok());
    let interop = sensiblaw_proof_search_loop::world_observation::WorldObservation {
        backend: GetterBackend::LeanInterop,
        ..observation.clone()
    };
    assert_eq!(
        sensiblaw_proof_search_loop::world_observation::compare_observations(
            &observation,
            &interop
        ),
        ObservationParity::Agreement
    );
}

#[test]
fn mabo_overrules_authority_source_is_observable_without_becoming_authority() {
    let acquired = acquired();
    let observation = wikidata_property_observation(
        "query:mabo:P4006",
        &acquired,
        &route_with(ProducerFamily::AuthoritySource, "P4006", "Q6851910"),
        GetterBackend::SlrNative,
    )
    .unwrap();
    assert_eq!(observation.relation_ref, "P4006");
    assert_eq!(observation.value_ref, "Q6851910");
    assert!(observation.candidate_only);
    assert!(!observation.creates_semantic_authority);
    assert!(!observation.claim_truth_promoted);
}

#[test]
fn wikidata_observation_accepts_classification_source_but_rejects_unrelated_producer() {
    let acquired = acquired();
    assert!(wikidata_property_observation(
        "query:mabo:P31",
        &acquired,
        &route_with(ProducerFamily::ClassificationEvidence, "P31", "Q2334719"),
        GetterBackend::SlrNative,
    )
    .is_ok());

    let error = wikidata_property_observation(
        "query:mabo:bad-producer",
        &acquired,
        &route_with(ProducerFamily::ArticleSemantic, "P710", "Q975866"),
        GetterBackend::SlrNative,
    )
    .unwrap_err();
    assert_eq!(
        error,
        sensiblaw_proof_search_loop::world_observation_adapters::WorldObservationAdapterError::WrongWikidataProducer
    );
}
