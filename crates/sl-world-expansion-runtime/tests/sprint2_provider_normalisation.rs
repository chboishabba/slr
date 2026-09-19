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
    normalize_cache_first_resolution, normalize_cached_legal_provider, normalize_oalc_provider,
    normalize_wikidata_provider, normalize_wikipedia_provider, reduce_reviewed_provider_evidence,
    CacheFirstProviderPath,
};
use sensiblaw_reviewed_evidence_payment::ReviewedEvidenceCoordinate;
use sensiblaw_pg_source_store::{
    CacheFirstResolution, CachedResolvedDocument, ResolutionPath, TemporalCoverage,
};

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


#[test]
fn cache_first_legal_source_enters_the_same_canonical_review_reducer_path() {
    let cached = CachedResolvedDocument {
        document_ref: "document:cache:mabo".into(),
        external_source_revision_ref: "legal:revision:mabo".into(),
        source_resolution_ref: "resolution:mabo".into(),
        provider_ref: "oalc".into(),
        dataset_ref: "isaacus/open-australian-legal-corpus".into(),
        dataset_revision_ref: "dataset:fixture".into(),
        external_version_ref: "version:fixture".into(),
        citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
        source_ref: "case:[1992]-HCA-23".into(),
        jurisdiction_ref: "AU".into(),
        document_type_ref: "primary_case".into(),
        temporal_coverage: TemporalCoverage::HistoricallyVerified,
        resolution_path: ResolutionPath::OfflineJsonlReplay,
        source_url: None,
        canonical_text: "fixture canonical legal text".into(),
    };
    let evidence = normalize_cached_legal_provider(
        "request:m2.4:cached-legal",
        &cached,
        "receipt:revision:cached-legal",
    )
    .unwrap();
    evidence.validate().unwrap();

    let review = review_for(&evidence.observation.observation_ref);
    let reduced = reduce_reviewed_provider_evidence(
        &evidence,
        &review,
        "payment:m2.4:cached-legal",
        None,
        None,
        None,
    )
    .unwrap();

    assert_eq!(reduced.observation_ref, evidence.observation.observation_ref);
    assert_eq!(reduced.source_revision_ref, cached.external_source_revision_ref);
    assert!(!reduced.creates_semantic_authority);
    assert!(!reduced.applicability_promoted);
    assert!(!reduced.claim_truth_promoted);
}


#[test]
fn pg_hit_preserves_zero_network_when_lowered_to_canonical_evidence() {
    let cached = CachedResolvedDocument {
        document_ref: "document:cache:hit".into(),
        external_source_revision_ref: "legal:revision:hit".into(),
        source_resolution_ref: "resolution:hit".into(),
        provider_ref: "oalc".into(),
        dataset_ref: "isaacus/open-australian-legal-corpus".into(),
        dataset_revision_ref: "dataset:fixture".into(),
        external_version_ref: "version:fixture".into(),
        citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
        source_ref: "case:[1992]-HCA-23".into(),
        jurisdiction_ref: "AU".into(),
        document_type_ref: "primary_case".into(),
        temporal_coverage: TemporalCoverage::HistoricallyVerified,
        resolution_path: ResolutionPath::OfflineJsonlReplay,
        source_url: None,
        canonical_text: "fixture canonical legal text".into(),
    };
    let normalized = normalize_cache_first_resolution(
        "request:m2.4:pg-hit",
        &CacheFirstResolution::PgHit {
            source: cached,
            network_requests: 0,
        },
        "receipt:revision:pg-hit",
    )
    .unwrap();

    assert_eq!(normalized.path, CacheFirstProviderPath::PgHit);
    assert_eq!(normalized.acquisition_network_requests, 0);
    assert_eq!(normalized.verification_network_requests, 0);
    normalized.validate().unwrap();
}

#[test]
fn acquired_persisted_path_retains_acquisition_count_and_zero_network_verification() {
    let cached = CachedResolvedDocument {
        document_ref: "document:cache:miss".into(),
        external_source_revision_ref: "legal:revision:miss".into(),
        source_resolution_ref: "resolution:miss".into(),
        provider_ref: "official-court".into(),
        dataset_ref: "official-source".into(),
        dataset_revision_ref: "dataset:fixture".into(),
        external_version_ref: "version:fixture".into(),
        citation: "[2026] HCA 19".into(),
        source_ref: "case:[2026]-HCA-19".into(),
        jurisdiction_ref: "AU".into(),
        document_type_ref: "primary_case".into(),
        temporal_coverage: TemporalCoverage::HistoricallyVerified,
        resolution_path: ResolutionPath::RevisionPinnedStreamingLegacy,
        source_url: Some("https://example.invalid/hca/19".into()),
        canonical_text: "fixture official legal text".into(),
    };
    let normalized = normalize_cache_first_resolution(
        "request:m2.4:pg-miss",
        &CacheFirstResolution::AcquiredPersisted {
            source: cached,
            acquisition_network_requests: 1,
            verification_network_requests: 0,
        },
        "receipt:revision:pg-miss",
    )
    .unwrap();

    assert_eq!(normalized.path, CacheFirstProviderPath::AcquiredPersisted);
    assert_eq!(normalized.acquisition_network_requests, 1);
    assert_eq!(normalized.verification_network_requests, 0);
    normalized.validate().unwrap();
}
