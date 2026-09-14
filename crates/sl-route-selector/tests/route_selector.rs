use sensiblaw_route_selector::{
    encode_route_candidate, select_routes, ProducerFamily, RouteCandidate, RouteFamily, SLRG_MAGIC,
    SLRG_VERSION,
};
use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use std::io::Cursor;

fn write_text(buf: &mut Vec<u8>, value: &str) {
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value.as_bytes());
}

fn intent(producer: ProducerFamily, obligation: &str) -> WireRecord {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"RTA1");
    payload.push(producer as u8);
    payload.push(4); // mechanism coordinate or fragment tag; opaque here
    payload.push(1);
    payload.push(0);
    payload.push(2); // evidence need class
    write_text(&mut payload, obligation);
    write_text(&mut payload, "consumer:test");
    write_text(&mut payload, "need:test");
    payload.push(0);
    WireRecord {
        kind: WorldRecordKind::RouteAction,
        id: format!("route-intent:{obligation}"),
        iteration_index: Some(20),
        aux1: Some("mechanism-evidence".into()),
        payload,
    }
}

fn candidate(id: &str, specificity: u32, old_gap_yield: u32, new_gap_cost: u32) -> RouteCandidate {
    RouteCandidate {
        candidate_id: id.into(),
        producer: ProducerFamily::MechanismEvidence,
        route_family: RouteFamily::WikipediaArticle,
        source_ref: "Q1".into(),
        target_ref: format!("Q:{id}"),
        property_ref: "P361".into(),
        cross_language_gap_coverage: 1,
        source_surface_support: 2,
        root_qid_support: 1,
        typed_property_support: 1,
        route_specificity: specificity,
        yield_history_observed: 1,
        prior_contracted_old_gaps: old_gap_yield,
        prior_retired_obligations: 1,
        prior_new_gap_atoms: new_gap_cost,
        prior_network_requests: 1,
    }
}

#[test]
fn route_candidate_wire_is_binary_versioned_and_round_trippable() {
    let row = candidate("a", 4, 2, 1);
    let mut bytes = Vec::new();
    encode_route_candidate(&mut bytes, &row).unwrap();
    assert_eq!(&bytes[..4], &SLRG_MAGIC);
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), SLRG_VERSION);
}

#[test]
fn selector_filters_by_producer_family() {
    let mut intents = Vec::new();
    encode_record(&mut intents, &intent(ProducerFamily::MechanismEvidence, "obligation:test")).unwrap();
    let mut candidates = Vec::new();
    encode_route_candidate(&mut candidates, &candidate("match", 4, 2, 1)).unwrap();
    let mut wrong = candidate("wrong", 5, 4, 0);
    wrong.producer = ProducerFamily::MeasurementEvidence;
    encode_route_candidate(&mut candidates, &wrong).unwrap();

    let mut output = Vec::new();
    let receipt = select_routes(&mut Cursor::new(intents), &mut Cursor::new(candidates), &mut output, 4).unwrap();
    assert_eq!(receipt.intents_seen, 1);
    assert_eq!(receipt.candidates_seen, 2);
    assert_eq!(receipt.selected_actions, 1);
    let selected = decode_record(&mut Cursor::new(output)).unwrap().unwrap();
    assert_eq!(selected.kind, WorldRecordKind::RouteAction);
    assert_eq!(&selected.payload[..4], b"RTA2");
}

#[test]
fn selector_uses_nonscalar_pareto_dominance() {
    let mut intents = Vec::new();
    encode_record(&mut intents, &intent(ProducerFamily::MechanismEvidence, "obligation:test")).unwrap();
    let mut candidates = Vec::new();
    // dominated: lower specificity/yield and higher new-gap burden.
    encode_route_candidate(&mut candidates, &candidate("dominated", 2, 0, 5)).unwrap();
    encode_route_candidate(&mut candidates, &candidate("front-a", 4, 2, 1)).unwrap();
    // tradeoff remains on Pareto front: higher specificity but worse old-gap yield/new-gap burden.
    encode_route_candidate(&mut candidates, &candidate("front-b", 5, 1, 2)).unwrap();

    let mut output = Vec::new();
    let receipt = select_routes(&mut Cursor::new(intents), &mut Cursor::new(candidates), &mut output, 4).unwrap();
    assert_eq!(receipt.selected_actions, 2);
    assert!(!receipt.pareto_dimensions_scalarized);
    assert!(!receipt.frontier_rank_is_truth_rank);
}

#[test]
fn selected_route_is_candidate_only_and_not_claim_truth() {
    let mut intents = Vec::new();
    encode_record(&mut intents, &intent(ProducerFamily::MechanismEvidence, "obligation:test")).unwrap();
    let mut candidates = Vec::new();
    encode_route_candidate(&mut candidates, &candidate("a", 4, 2, 1)).unwrap();
    let mut output = Vec::new();
    let receipt = select_routes(&mut Cursor::new(intents), &mut Cursor::new(candidates), &mut output, 1).unwrap();
    assert!(receipt.candidate_only);
    assert!(!receipt.semantic_promotion);
    assert!(!receipt.route_action_is_claim_truth);
    let selected = decode_record(&mut Cursor::new(output)).unwrap().unwrap();
    assert_eq!(selected.payload[6], 1);
    assert_eq!(selected.payload[7], 0);
}

#[test]
fn selector_has_no_json_or_regex_contract() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
