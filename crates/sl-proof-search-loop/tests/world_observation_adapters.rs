use sensiblaw_proof_search_loop::world_expansion_adapters::AcquiredWikidataEntity;
use sensiblaw_proof_search_loop::world_observation::{GetterBackend, ObservationParity};
use sensiblaw_proof_search_loop::world_observation_adapters::wikidata_property_observation;
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};

fn route() -> RouteCandidate {
    RouteCandidate {
        candidate_id: "wikidata:Q1501525:P710:Q975866".into(),
        producer: ProducerFamily::IdentitySource,
        route_family: RouteFamily::WikidataProperty,
        source_ref: "Q1501525".into(),
        target_ref: "Q975866".into(),
        property_ref: "P710".into(),
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

#[test]
fn mabo_native_wikidata_route_normalizes_to_golden_observation_shape() {
    let acquired = AcquiredWikidataEntity {
        qid: "Q1501525".into(),
        source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        content_digest_ref: "sha256:mabo-wikidata".into(),
        candidate_only: true,
        semantic_promotion: false,
    };
    let observation = wikidata_property_observation(
        "query:mabo:P710",
        &acquired,
        &route(),
        GetterBackend::SlrNative,
    ).unwrap();
    assert_eq!(observation.object_ref, "Q1501525");
    assert_eq!(observation.relation_ref, "P710");
    assert_eq!(observation.value_ref, "Q975866");
    assert_eq!(observation.source_revision_ref, "wikidata:Q1501525:oldid:2333409615");
    assert!(observation.validate().is_ok());
    let interop = sensiblaw_proof_search_loop::world_observation::WorldObservation {
        backend: GetterBackend::LeanInterop,
        ..observation.clone()
    };
    assert_eq!(
        sensiblaw_proof_search_loop::world_observation::compare_observations(&observation, &interop),
        ObservationParity::Agreement
    );
}
