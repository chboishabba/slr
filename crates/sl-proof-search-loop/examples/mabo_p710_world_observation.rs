use sensiblaw_proof_search_loop::world_observation::GetterBackend;
use sensiblaw_proof_search_loop::world_observation_adapters::wikidata_property_observation;
use sensiblaw_route_selector::{decode_route_candidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_entity_rdf_revision_receipt,
};

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
    let route = matched.ok_or_else(|| {
        std::io::Error::other("pinned Mabo P710 -> Q975866 route not present in provider output")
    })?;
    let observation = wikidata_property_observation(
        "query:mabo:P710:Q975866",
        &acquired,
        &route,
        GetterBackend::SlrNative,
    )
    .map_err(|error| std::io::Error::other(format!("observation adapter: {error:?}")))?;
    observation
        .validate()
        .map_err(|error| std::io::Error::other(format!("invalid observation: {error:?}")))?;

    println!("object_ref={}", observation.object_ref);
    println!("relation_ref={}", observation.relation_ref);
    println!("value_ref={}", observation.value_ref);
    println!("source_revision_ref={}", observation.source_revision_ref);
    println!("content_digest_ref={}", observation.content_digest_ref);
    println!("direct_property_candidates={}", provider_receipt.direct_property_candidates);
    println!("candidate_only={}", observation.candidate_only);
    println!("creates_semantic_authority={}", observation.creates_semantic_authority);
    println!("claim_truth_promoted={}", observation.claim_truth_promoted);
    Ok(())
}
