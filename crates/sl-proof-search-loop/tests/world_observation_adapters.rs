use sensiblaw_proof_search_loop::world_observation::{GetterBackend, ObservationParity};
use sensiblaw_proof_search_loop::world_observation_adapters::{
    oalc_evidence_manifestation, wikidata_evidence_manifestation,
    wikidata_property_observation, wikipedia_evidence_manifestation,
};
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

fn acquired() -> sensiblaw_wikimedia_candidate_provider::AcquiredEntityRdf {
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


#[test]
fn canonical_manifestation_envelope_is_shared_across_existing_producers() {
    use sensiblaw_core::canonical_evidence::EvidenceManifestationFamily;
    use sensiblaw_governed_legal_provider::OalcLookupReceipt;
    use sensiblaw_route_executor::{AcquiredSource, AcquiredSourceKind};

    let wikidata = acquired();
    let wikidata_manifestation =
        wikidata_evidence_manifestation(&wikidata, "receipt:wikidata:fixture").unwrap();
    assert_eq!(
        wikidata_manifestation.family,
        EvidenceManifestationFamily::Wikidata
    );
    assert!(wikidata_manifestation.validate().is_ok());

    let wikipedia = AcquiredSource {
        kind: AcquiredSourceKind::WikipediaRenderedHtml,
        document_ref: "document:wikipedia:fixture".into(),
        source_ref: "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)".into(),
        language: "en".into(),
        revision_ref: "wikipedia:en:fixture:oldid:1".into(),
        canonical_url: "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)".into(),
        source_sha256: [7_u8; 32],
        text: "fixture".into(),
        candidate_only: true,
        semantic_promotion: false,
    };
    let wikipedia_manifestation =
        wikipedia_evidence_manifestation(&wikipedia, "receipt:wikipedia:fixture").unwrap();
    assert_eq!(
        wikipedia_manifestation.family,
        EvidenceManifestationFamily::Wikipedia
    );
    assert!(wikipedia_manifestation.validate().is_ok());

    let oalc = OalcLookupReceipt {
        corpus_revision_ref: "oalc:corpus:fixture".into(),
        citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
        source_identity_ref: "case:[1992]-HCA-23".into(),
        source_revision_ref: "oalc:revision:fixture".into(),
        canonical_text_digest: "sha256:fixture".into(),
        local_artifact_ref: "artifact:oalc:fixture".into(),
        network_requests: 0,
        receipt_authority: "experimental_candidate_only",
    };
    let oalc_manifestation =
        oalc_evidence_manifestation(&oalc, "receipt:oalc:fixture").unwrap();
    assert_eq!(oalc_manifestation.family, EvidenceManifestationFamily::Oalc);
    assert!(oalc_manifestation.validate().is_ok());

    assert!(!wikidata_manifestation.creates_semantic_authority);
    assert!(!wikipedia_manifestation.applicability_promoted);
    assert!(!oalc_manifestation.claim_truth_promoted);
}
