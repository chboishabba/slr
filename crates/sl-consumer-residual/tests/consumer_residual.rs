use sensiblaw_consumer_residual::{
    compile_consumer_residual_stream, decode_consumer_spec, encode_consumer_spec,
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, FragmentKind, RequirementNeed,
    RequirementScope, CONSUMER_MAGIC, CONSUMER_VERSION,
};
use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use std::io::Cursor;

fn pnf_payload(fragment: FragmentKind) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"PNF1");
    body.push(fragment as u8);
    body.push(1);
    body.push(1);
    body.push(0);
    body
}

fn world_with_pnf(fragment: FragmentKind, source: &str, iteration: i64) -> Vec<u8> {
    let mut out = Vec::new();
    encode_record(
        &mut out,
        &WireRecord {
            kind: WorldRecordKind::PnfCandidate,
            id: "pnf:test".into(),
            iteration_index: Some(iteration),
            aux1: Some(source.into()),
            payload: pnf_payload(fragment),
        },
    )
    .unwrap();
    out
}

fn fragment_requirement(id: &str, fragment: FragmentKind, scope: RequirementScope) -> ConsumerRequirement {
    ConsumerRequirement {
        requirement_id: id.into(),
        need: RequirementNeed::PnfFragment(fragment),
        scope,
    }
}

fn evidence_requirement(id: &str, kind: EvidenceCoordinateKind) -> ConsumerRequirement {
    ConsumerRequirement {
        requirement_id: id.into(),
        need: RequirementNeed::EvidenceCoordinate(kind),
        scope: RequirementScope::AnySource,
    }
}

fn spec(requirement: ConsumerRequirement) -> ConsumerSpec {
    ConsumerSpec {
        consumer_id: "consumer:test".into(),
        surface_id: "surface:test".into(),
        requirements: vec![requirement],
    }
}

#[test]
fn consumer_spec_round_trip_is_versioned_binary_v2() {
    let s = ConsumerSpec {
        consumer_id: "consumer:test".into(),
        surface_id: "surface:test".into(),
        requirements: vec![
            fragment_requirement("need-actor", FragmentKind::Actor, RequirementScope::AnySource),
            evidence_requirement("need-mechanism", EvidenceCoordinateKind::Mechanism),
        ],
    };
    let mut bytes = Vec::new();
    encode_consumer_spec(&mut bytes, &s).unwrap();
    assert_eq!(&bytes[..4], &CONSUMER_MAGIC);
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), CONSUMER_VERSION);
    assert_eq!(CONSUMER_VERSION, 2);
    assert_eq!(decode_consumer_spec(&mut Cursor::new(bytes)).unwrap(), s);
}

#[test]
fn observed_required_fragment_emits_append_only_payment_receipts() {
    let world = world_with_pnf(FragmentKind::Actor, "wiki:Q207:en:456", 5);
    let spec = spec(fragment_requirement("need-actor", FragmentKind::Actor, RequirementScope::AnySource));
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(&mut Cursor::new(world), &spec, &mut out, 5).unwrap();
    assert_eq!(receipt.requirements_paid, 1);
    assert_eq!(receipt.requirements_unpaid, 0);
    assert_eq!(receipt.payments_emitted, 2);
    assert!(!receipt.semantic_promotion);

    let mut cursor = Cursor::new(out);
    let _original = decode_record(&mut cursor).unwrap().unwrap();
    let gap_payment = decode_record(&mut cursor).unwrap().unwrap();
    let obligation_payment = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(gap_payment.kind, WorldRecordKind::Payment);
    assert_eq!(obligation_payment.kind, WorldRecordKind::Payment);
    assert_eq!(gap_payment.aux1.as_deref(), Some("gap:consumer:test:need-actor"));
    assert_eq!(obligation_payment.aux1.as_deref(), Some("obligation:consumer:test:need-actor"));
    assert_eq!(&gap_payment.payload[..4], b"PAY1");
}

#[test]
fn unpaid_fragment_emits_gap1_and_obl1() {
    let world = world_with_pnf(FragmentKind::Actor, "wiki:Q207:en:456", 7);
    let spec = spec(fragment_requirement("need-patient", FragmentKind::Patient, RequirementScope::AnySource));
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(&mut Cursor::new(world), &spec, &mut out, 7).unwrap();
    assert_eq!(receipt.requirements_paid, 0);
    assert_eq!(receipt.requirements_unpaid, 1);
    assert_eq!(receipt.gaps_emitted, 1);
    assert_eq!(receipt.obligations_emitted, 1);

    let mut cursor = Cursor::new(out);
    let _original = decode_record(&mut cursor).unwrap().unwrap();
    let gap = decode_record(&mut cursor).unwrap().unwrap();
    let obligation = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(&gap.payload[..4], b"GAP1");
    assert_eq!(&obligation.payload[..4], b"OBL1");
    assert_eq!(gap.payload[4], FragmentKind::Patient as u8);
}

#[test]
fn substantive_evidence_coordinate_is_not_paid_by_matching_pnf_shape() {
    let world = world_with_pnf(FragmentKind::Relation, "abc730:primary", 10);
    let spec = spec(evidence_requirement("C029:mechanism", EvidenceCoordinateKind::Mechanism));
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(&mut Cursor::new(world), &spec, &mut out, 10).unwrap();
    assert_eq!(receipt.requirements_paid, 0);
    assert_eq!(receipt.requirements_unpaid, 1);

    let mut cursor = Cursor::new(out);
    let _original = decode_record(&mut cursor).unwrap().unwrap();
    let gap = decode_record(&mut cursor).unwrap().unwrap();
    let obligation = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(&gap.payload[..4], b"GAP2");
    assert_eq!(&obligation.payload[..4], b"OBL2");
    assert_eq!(gap.payload[4], EvidenceCoordinateKind::Mechanism as u8);
}

#[test]
fn real_c029_consumer_coordinates_round_trip_without_claim_promotion() {
    let spec = ConsumerSpec {
        consumer_id: "ABC730-2026-09-09-C029".into(),
        surface_id: "ABC730:C029:consumer-adequacy".into(),
        requirements: vec![
            evidence_requirement("C029:settlementSubcountryClassifier", EvidenceCoordinateKind::Classification),
            evidence_requirement("C029:palestinianNetIncidence", EvidenceCoordinateKind::Incidence),
            evidence_requirement("C029:settlementTradeMagnitude", EvidenceCoordinateKind::Quantification),
            evidence_requirement("C029:unintendedConsequenceMechanism", EvidenceCoordinateKind::Mechanism),
            evidence_requirement("C029:effectProbability", EvidenceCoordinateKind::Probability),
            evidence_requirement("C029:counterfactual", EvidenceCoordinateKind::Counterfactual),
            evidence_requirement("C029:instrumentComparison", EvidenceCoordinateKind::InstrumentComparison),
        ],
    };
    let mut bytes = Vec::new();
    encode_consumer_spec(&mut bytes, &spec).unwrap();
    let decoded = decode_consumer_spec(&mut Cursor::new(bytes)).unwrap();
    assert_eq!(decoded, spec);
    assert_eq!(decoded.requirements.len(), 7);
}

#[test]
fn source_scoped_fragment_requirement_is_not_paid_by_other_source() {
    let world = world_with_pnf(FragmentKind::Actor, "wiki:QOTHER:en:1", 9);
    let spec = spec(fragment_requirement(
        "need-actor-q207",
        FragmentKind::Actor,
        RequirementScope::SourceManifestation("wiki:Q207:en:456".into()),
    ));
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(&mut Cursor::new(world), &spec, &mut out, 9).unwrap();
    assert_eq!(receipt.requirements_paid, 0);
    assert_eq!(receipt.requirements_unpaid, 1);
    assert_eq!(receipt.payments_emitted, 0);
}

#[test]
fn consumer_residual_source_has_no_json_or_regex_dependency() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let cargo = std::fs::read_to_string(manifest).unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
