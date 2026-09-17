use sensiblaw_governed_legal_provider::OalcLookupReceipt;
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::{ResidualClass, ProducerLane};
use sensiblaw_proof_search_loop::world_expansion_adapters::{
    from_oalc_lookup, from_wikidata_route, AcquiredWikidataEntity, ExpansionAdapterError,
    ExpansionScoring,
};
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};

fn residual() -> ProofResidual {
    ProofResidual {
        residual_ref: "residual:mabo:identity".into(),
        proposition_ref: "mabo:proposition:identity".into(),
        producer_class_ref: "producer:world-expansion".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status: ResidualStatus::Open,
    }
}

fn scoring() -> ExpansionScoring {
    ExpansionScoring {
        expected_residual_contraction: 2,
        provenance_quality: 5,
        same_object_confidence: 5,
        expected_new_world_value: 4,
        acquisition_cost: 1,
    }
}

fn route(producer: ProducerFamily, source_ref: &str) -> RouteCandidate {
    RouteCandidate {
        candidate_id: "wikidata:Q1501525:P710:Q975866".into(),
        producer,
        route_family: RouteFamily::WikidataProperty,
        source_ref: source_ref.into(),
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
fn wikidata_revision_receipt_must_match_route_source_qid_and_producer() {
    let acquired = AcquiredWikidataEntity {
        qid: "Q1501525".into(),
        source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        content_digest_ref: "sha256:mabo-wikidata".into(),
        candidate_only: true,
        semantic_promotion: false,
    };
    let candidate = from_wikidata_route(
        &residual(), ResidualClass::Identity, &acquired,
        &route(ProducerFamily::IdentitySource, "Q1501525"), scoring(),
    ).unwrap();
    assert_eq!(candidate.producer_lane, ProducerLane::WikidataIdentity);

    assert_eq!(
        from_wikidata_route(
            &residual(), ResidualClass::Identity, &acquired,
            &route(ProducerFamily::IdentitySource, "Q1"), scoring(),
        ),
        Err(ExpansionAdapterError::WikidataSourceMismatch)
    );
    assert_eq!(
        from_wikidata_route(
            &residual(), ResidualClass::Identity, &acquired,
            &route(ProducerFamily::ArticleSemantic, "Q1501525"), scoring(),
        ),
        Err(ExpansionAdapterError::WrongWikidataProducer)
    );
}

#[test]
fn oalc_adapter_rejects_non_candidate_receipt_authority() {
    let receipt = OalcLookupReceipt {
        corpus_revision_ref: "oalc:rev".into(),
        citation: "[1992] HCA 23".into(),
        source_identity_ref: "case:[1992]-HCA-23".into(),
        source_revision_ref: "oalc:source:rev".into(),
        canonical_text_digest: "abc".into(),
        local_artifact_ref: "oalc://mabo".into(),
        network_requests: 0,
        receipt_authority: "semantic_authority",
    };
    assert_eq!(
        from_oalc_lookup(&residual(), ResidualClass::Legal, "Q1501525", &receipt, scoring()),
        Err(ExpansionAdapterError::OalcReceiptNotCandidateOnly)
    );
}
