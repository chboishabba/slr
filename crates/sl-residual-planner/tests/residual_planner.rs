use sensiblaw_residual_planner::{plan_active_frontier_stream, ProducerFamily};
use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use std::io::Cursor;

fn write_text(buf: &mut Vec<u8>, value: &str) {
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value.as_bytes());
}

fn obligation(magic: &[u8; 4], need_tag: u8, id: &str, iteration: i64) -> WireRecord {
    let mut payload = Vec::new();
    payload.extend_from_slice(magic);
    payload.push(need_tag);
    payload.push(1);
    payload.push(0);
    write_text(&mut payload, "consumer:test");
    write_text(&mut payload, id);
    payload.push(0);
    WireRecord {
        kind: WorldRecordKind::Obligation,
        id: format!("obligation:consumer:test:{id}"),
        iteration_index: Some(iteration),
        aux1: Some(if magic == b"OBL1" { "need-fragment-kind" } else { "need-evidence-coordinate" }.into()),
        payload,
    }
}

fn route_for(magic: &[u8; 4], need_tag: u8) -> (sensiblaw_residual_planner::PlanReceipt, WireRecord) {
    let mut input = Vec::new();
    encode_record(&mut input, &obligation(magic, need_tag, "need-x", 4)).unwrap();
    let mut output = Vec::new();
    let receipt = plan_active_frontier_stream(&mut Cursor::new(input), &mut output).unwrap();
    let route = decode_record(&mut Cursor::new(output)).unwrap().unwrap();
    (receipt, route)
}

#[test]
fn semantic_fragment_obligation_selects_article_semantic_producer() {
    let (receipt, route) = route_for(b"OBL1", 1);
    assert_eq!(receipt.obligations_seen, 1);
    assert_eq!(receipt.route_intents_emitted, 1);
    assert_eq!(route.kind, WorldRecordKind::RouteAction);
    assert_eq!(route.payload[4], ProducerFamily::ArticleSemantic as u8);
    assert_eq!(route.payload[5], 1);
    assert_eq!(route.payload[6], 1);
    assert_eq!(route.payload[7], 0);
}

#[test]
fn temporal_obligation_selects_revision_temporal_producer() {
    let (_, route) = route_for(b"OBL1", 9);
    assert_eq!(route.payload[4], ProducerFamily::RevisionTemporal as u8);
}

#[test]
fn unresolved_obligation_selects_parser_repair_not_external_research() {
    let (_, route) = route_for(b"OBL1", 12);
    assert_eq!(route.payload[4], ProducerFamily::ParserRepair as u8);
}

#[test]
fn substantive_evidence_coordinates_route_to_typed_producer_families() {
    let cases = [
        (1, ProducerFamily::IdentitySource),
        (2, ProducerFamily::IdentitySource),
        (3, ProducerFamily::AuthoritySource),
        (4, ProducerFamily::MechanismEvidence),
        (5, ProducerFamily::MeasurementEvidence),
        (6, ProducerFamily::MeasurementEvidence),
        (7, ProducerFamily::ComparatorEvidence),
        (8, ProducerFamily::ComparatorEvidence),
        (9, ProducerFamily::MeasurementEvidence),
        (10, ProducerFamily::ClassificationEvidence),
    ];
    for (tag, expected) in cases {
        let (_, route) = route_for(b"OBL2", tag);
        assert_eq!(route.payload[4], expected as u8, "evidence tag {tag}");
        assert_eq!(route.payload[5], tag);
    }
}

#[test]
fn gap_rows_do_not_independently_create_route_intents() {
    let mut gap = obligation(b"OBL1", 2, "need-patient", 4);
    gap.kind = WorldRecordKind::Gap;
    gap.id = "gap:consumer:test:need-patient".into();
    gap.payload[..4].copy_from_slice(b"GAP1");
    let mut input = Vec::new();
    encode_record(&mut input, &gap).unwrap();
    let mut output = Vec::new();
    let receipt = plan_active_frontier_stream(&mut Cursor::new(input), &mut output).unwrap();
    assert_eq!(receipt.obligations_seen, 0);
    assert_eq!(receipt.route_intents_emitted, 0);
    assert!(output.is_empty());
}

#[test]
fn planner_has_no_json_regex_or_truth_promotion_contract() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
    let (receipt, _) = route_for(b"OBL1", 1);
    assert!(receipt.candidate_only);
    assert!(!receipt.semantic_promotion);
    assert!(!receipt.route_intent_is_claim_truth);
}
