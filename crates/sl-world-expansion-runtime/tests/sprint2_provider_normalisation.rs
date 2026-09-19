// M2.4 provider normalisation regression surface.
//
// These tests deliberately exercise existing producer artifacts.  They do not
// perform acquisition and they do not create review authority.

use sensiblaw_consumer_residual::EvidenceCoordinateKind;
use sensiblaw_governed_legal_provider::OalcLookupReceipt;
use sensiblaw_route_executor::{AcquiredSource, AcquiredSourceKind};
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::entity_revision_receipt_from_rdf;
use sensiblaw_world_expansion_runtime::sprint2_provider_normalisation::{
    normalize_oalc_provider, normalize_wikidata_provider, normalize_wikipedia_provider,
    reduce_reviewed_provider_evidence,
};
use sensiblaw_reviewed_evidence_payment::ReviewedEvidenceCoordinate;

fn review_for(observation_ref: &str) -> ReviewedEvidenceCoordinate {
    ReviewedEvidenceCoordinate {
        review_ref: format!("review:{observation_ref}"),
        consumer_id: "consumer:m2.4-regression".into(),
        requirement_id: "requirement:canonical-provider-evidence".into(),
        coordinate: EvidenceCoordinateKind::Mechanism,
        source_ref: None,
        evidence_ref: observation_ref.into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn existing_wikidata_wikipedia_and_oalc_artifacts_lower_to_one_canonical_carrier() {
    let wikidata = entity_revision_receipt_from_rdf(
        "Q1501525",
        2333409615,
        b"<rdf:RDF>fixture</rdf:RDF>".to_vec(),
    )
    .unwrap();
    let route = RouteCandidate {
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
    };
    let wd = normalize_wikidata_provider(
        "request:m2.4:wikidata",
        &wikidata,
        &route,
        "receipt:acquire:wikidata",
        "receipt:revision:wikidata",
    )
    .unwrap();

    let wikipedia = AcquiredSource {
        kind: AcquiredSourceKind::WikipediaRenderedHtml,
        document_ref: "document:wikipedia:mabo".into(),
        source_ref: "Q1501525".into(),
        language: "en".into(),
        revision_ref: "wikipedia:en:mabo:oldid:1".into(),
        canonical_url: "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)".into(),
        source_sha256: [7_u8; 32],
        text: "Mabo v Queensland (No 2)".into(),
        candidate_only: true,
        semantic_promotion: false,
    };
    let wp = normalize_wikipedia_provider(
        "request:m2.4:wikipedia",
        &wikipedia,
        "receipt:acquire:wikipedia",
        "receipt:revision:wikipedia",
    )
    .unwrap();

    let oalc = OalcLookupReceipt {
        corpus_revision_ref: "oalc:corpus:fixture".into(),
        citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
        source_identity_ref: "case:[1992]-HCA-23".into(),
        source_revision_ref: "oalc:revision:mabo".into(),
        canonical_text_digest: "sha256:oalc-fixture".into(),
        local_artifact_ref: "oalc://mabo".into(),
        network_requests: 0,
        receipt_authority: "experimental_candidate_only",
    };
    let legal = normalize_oalc_provider(
        "request:m2.4:oalc",
        &oalc,
        "receipt:acquire:oalc",
        "receipt:revision:oalc",
    )
    .unwrap();

    for evidence in [&wd, &wp, &legal] {
        evidence.validate().unwrap();
        let review = review_for(&evidence.observation.observation_ref);
        let reduced = reduce_reviewed_provider_evidence(
            evidence,
            &review,
            "payment:m2.4",
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(reduced.observation_ref, evidence.observation.observation_ref);
        assert_eq!(reduced.source_revision_ref, evidence.revision.source_revision_ref);
        assert_eq!(reduced.span_ref, evidence.observation.span.span_ref);
        assert!(reduced.candidate_only);
        assert!(!reduced.creates_semantic_authority);
        assert!(!reduced.applicability_promoted);
        assert!(!reduced.claim_truth_promoted);
    }
}
