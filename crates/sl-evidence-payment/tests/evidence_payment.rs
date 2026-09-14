use sensiblaw_evidence_payment::{
    apply_review_to_active_frontier, decode_review_spec, encode_review_spec, EvidenceCoordinateKind,
    EvidenceReviewSpec, ReviewDisposition, REVIEW_MAGIC, REVIEW_VERSION,
};
use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use std::io::Cursor;

fn write_text(buf: &mut Vec<u8>, value: &str) {
    buf.extend_from_slice(&(value.len() as u32).to_le_bytes());
    buf.extend_from_slice(value.as_bytes());
}

fn obligation(consumer: &str, requirement: &str, coordinate: EvidenceCoordinateKind, iteration: i64) -> WireRecord {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"OBL2");
    payload.push(coordinate as u8);
    payload.push(1);
    payload.push(0);
    write_text(&mut payload, consumer);
    write_text(&mut payload, requirement);
    payload.push(0);
    WireRecord {
        kind: WorldRecordKind::Obligation,
        id: format!("obligation:{consumer}:{requirement}"),
        iteration_index: Some(iteration),
        aux1: Some("need-evidence-coordinate".into()),
        payload,
    }
}

fn review(disposition: ReviewDisposition) -> EvidenceReviewSpec {
    EvidenceReviewSpec {
        consumer_id: "ABC730-2026-09-09-C029".into(),
        requirement_id: "C029:unintendedConsequenceMechanism".into(),
        coordinate: EvidenceCoordinateKind::Mechanism,
        evidence_reference: "evidence:source:mechanism:1".into(),
        reviewer_reference: "review:operator:1".into(),
        disposition,
    }
}

#[test]
fn review_spec_round_trip_is_binary() {
    let spec = review(ReviewDisposition::PaysObligation);
    let mut bytes = Vec::new();
    encode_review_spec(&mut bytes, &spec).unwrap();
    assert_eq!(&bytes[..4], &REVIEW_MAGIC);
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), REVIEW_VERSION);
    assert_eq!(decode_review_spec(&mut Cursor::new(bytes)).unwrap(), spec);
}

#[test]
fn exact_reviewed_payment_emits_review_and_two_pay2_receipts() {
    let spec = review(ReviewDisposition::PaysObligation);
    let mut frontier = Vec::new();
    encode_record(
        &mut frontier,
        &obligation(&spec.consumer_id, &spec.requirement_id, spec.coordinate, 12),
    )
    .unwrap();
    let mut out = Vec::new();
    let receipt = apply_review_to_active_frontier(&mut Cursor::new(frontier), &spec, &mut out, 13).unwrap();
    assert!(receipt.target_obligation_found);
    assert_eq!(receipt.reviews_emitted, 1);
    assert_eq!(receipt.payments_emitted, 2);
    assert!(!receipt.claim_truth_promoted);

    let mut cursor = Cursor::new(out);
    let review = decode_record(&mut cursor).unwrap().unwrap();
    let gap_payment = decode_record(&mut cursor).unwrap().unwrap();
    let obligation_payment = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(review.kind, WorldRecordKind::Review);
    assert_eq!(&review.payload[..4], b"RVW1");
    assert_eq!(gap_payment.kind, WorldRecordKind::Payment);
    assert_eq!(obligation_payment.kind, WorldRecordKind::Payment);
    assert_eq!(&gap_payment.payload[..4], b"PAY2");
    assert_eq!(gap_payment.aux1.as_deref(), Some("gap:ABC730-2026-09-09-C029:C029:unintendedConsequenceMechanism"));
    assert_eq!(obligation_payment.aux1.as_deref(), Some("obligation:ABC730-2026-09-09-C029:C029:unintendedConsequenceMechanism"));
}

#[test]
fn partial_review_is_retained_but_does_not_contract_frontier() {
    let spec = review(ReviewDisposition::PartialEvidenceOnly);
    let mut frontier = Vec::new();
    encode_record(&mut frontier, &obligation(&spec.consumer_id, &spec.requirement_id, spec.coordinate, 12)).unwrap();
    let mut out = Vec::new();
    let receipt = apply_review_to_active_frontier(&mut Cursor::new(frontier), &spec, &mut out, 13).unwrap();
    assert!(receipt.target_obligation_found);
    assert_eq!(receipt.reviews_emitted, 1);
    assert_eq!(receipt.payments_emitted, 0);
    let record = decode_record(&mut Cursor::new(out)).unwrap().unwrap();
    assert_eq!(record.kind, WorldRecordKind::Review);
    assert_eq!(&record.payload[..4], b"RVW1");
}

#[test]
fn coordinate_mismatch_cannot_pay_different_obligation() {
    let spec = review(ReviewDisposition::PaysObligation);
    let mut frontier = Vec::new();
    encode_record(
        &mut frontier,
        &obligation(&spec.consumer_id, &spec.requirement_id, EvidenceCoordinateKind::Quantification, 12),
    )
    .unwrap();
    let mut out = Vec::new();
    let error = apply_review_to_active_frontier(&mut Cursor::new(frontier), &spec, &mut out, 13).unwrap_err();
    assert!(error.to_string().contains("active obligation"));
    assert!(out.is_empty());
}

#[test]
fn reviewed_evidence_has_no_json_regex_or_automatic_truth_contract() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
    let spec = review(ReviewDisposition::RejectedWrongType);
    let mut frontier = Vec::new();
    encode_record(&mut frontier, &obligation(&spec.consumer_id, &spec.requirement_id, spec.coordinate, 12)).unwrap();
    let mut out = Vec::new();
    let receipt = apply_review_to_active_frontier(&mut Cursor::new(frontier), &spec, &mut out, 13).unwrap();
    assert_eq!(receipt.payments_emitted, 0);
    assert!(!receipt.claim_truth_promoted);
    assert!(!receipt.semantic_authority_created);
}
