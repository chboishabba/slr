use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_proof_search_loop::world_observation::GetterBackend;
use sensiblaw_proof_search_loop::world_observation_adapters::wikidata_property_observation;
use sensiblaw_reviewed_evidence_payment::compile_consumer_residual_stream_review_aware;
use sensiblaw_route_selector::{decode_route_candidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_entity_rdf_revision_receipt,
};
use sensiblaw_world_expansion_runtime::reviewed_observation_payment_stream;

const MABO_QID: &str = "Q1501525";
const MABO_REVISION: u64 = 2_333_409_615;
const PARTICIPANT_PROPERTY: &str = "P710";
const EDDIE_MABO_QID: &str = "Q975866";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let acquired = fetch_entity_rdf_revision_receipt(MABO_QID, MABO_REVISION)?;
    let mut encoded = Vec::new();
    let provider_receipt = emit_candidates_from_rdf(
        MABO_QID,
        std::io::Cursor::new(acquired.rdf_bytes.as_slice()),
        &mut encoded,
    )?;

    let mut cursor = std::io::Cursor::new(encoded);
    let mut matched = None;
    while let Some(route) = decode_route_candidate(&mut cursor)? {
        if route.route_family == RouteFamily::WikidataProperty
            && route.source_ref == MABO_QID
            && route.property_ref == PARTICIPANT_PROPERTY
            && route.target_ref == EDDIE_MABO_QID
        {
            matched = Some(route);
            break;
        }
    }
    let route = matched.ok_or("pinned Mabo P710 -> Q975866 route not present")?;
    let observation = wikidata_property_observation(
        "query:mabo:P710:Q975866",
        &acquired,
        &route,
        GetterBackend::SlrNative,
    )?;

    let spec = ConsumerSpec {
        consumer_id: "consumer:mabo-100-identity-classes".into(),
        surface_id: "surface:mabo".into(),
        requirements: vec![ConsumerRequirement {
            requirement_id: "participant-identity".into(),
            need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject),
            scope: RequirementScope::SourceManifestation(observation.source_revision_ref.clone()),
        }],
    };

    let payment_stream = reviewed_observation_payment_stream(
        &spec,
        "participant-identity",
        EvidenceCoordinateKind::SameObject,
        "review:mabo:P710:Q975866",
        &observation,
        7,
    )?;
    let mut next_world = Vec::new();
    let residual_receipt = compile_consumer_residual_stream_review_aware(
        &mut std::io::Cursor::new(payment_stream),
        &spec,
        &mut next_world,
        8,
    )?;

    println!("object_ref={}", observation.object_ref);
    println!("relation_ref={}", observation.relation_ref);
    println!("value_ref={}", observation.value_ref);
    println!("source_revision_ref={}", observation.source_revision_ref);
    println!("content_digest_ref={}", observation.content_digest_ref);
    println!("direct_property_candidates={}", provider_receipt.direct_property_candidates);
    println!("reviewed_evidence_coordinate=SameObject");
    println!("requirements_total={}", residual_receipt.requirements_total);
    println!("requirements_paid={}", residual_receipt.requirements_paid);
    println!("requirements_unpaid={}", residual_receipt.requirements_unpaid);
    println!("gaps_emitted={}", residual_receipt.gaps_emitted);
    println!("obligations_emitted={}", residual_receipt.obligations_emitted);
    println!("candidate_only={}", residual_receipt.candidate_only);
    println!("semantic_promotion={}", residual_receipt.semantic_promotion);
    Ok(())
}
