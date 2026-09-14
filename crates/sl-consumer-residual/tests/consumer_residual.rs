use sensiblaw_consumer_residual::{
    compile_consumer_residual_stream, decode_consumer_spec, encode_consumer_spec,
    ConsumerRequirement, ConsumerSpec, FragmentKind, RequirementScope, CONSUMER_MAGIC,
    CONSUMER_VERSION,
};
use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use std::io::Cursor;

fn pnf_payload(fragment: FragmentKind) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(b"PNF1");
    body.push(fragment as u8);
    body.push(1); // dependency shape tag is opaque to this compiler
    body.push(1); // candidate-only
    body.push(0); // semantic promotion false
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

fn spec(requirement: ConsumerRequirement) -> ConsumerSpec {
    ConsumerSpec {
        consumer_id: "consumer:test".into(),
        surface_id: "surface:test".into(),
        requirements: vec![requirement],
    }
}

#[test]
fn consumer_spec_round_trip_is_versioned_binary() {
    let s = spec(ConsumerRequirement {
        requirement_id: "need-actor".into(),
        fragment: FragmentKind::Actor,
        scope: RequirementScope::AnySource,
    });
    let mut bytes = Vec::new();
    encode_consumer_spec(&mut bytes, &s).unwrap();
    assert_eq!(&bytes[..4], &CONSUMER_MAGIC);
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), CONSUMER_VERSION);
    assert_eq!(decode_consumer_spec(&mut Cursor::new(bytes)).unwrap(), s);
}

#[test]
fn observed_required_fragment_pays_consumer_requirement() {
    let world = world_with_pnf(FragmentKind::Actor, "wiki:Q207:en:456", 5);
    let spec = spec(ConsumerRequirement {
        requirement_id: "need-actor".into(),
        fragment: FragmentKind::Actor,
        scope: RequirementScope::AnySource,
    });
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(
        &mut Cursor::new(world.clone()),
        &spec,
        &mut out,
        5,
    )
    .unwrap();
    assert_eq!(receipt.requirements_paid, 1);
    assert_eq!(receipt.requirements_unpaid, 0);
    assert_eq!(receipt.gaps_emitted, 0);
    assert_eq!(receipt.obligations_emitted, 0);
    assert_eq!(out, world);
    assert!(!receipt.semantic_promotion);
}

#[test]
fn unpaid_fragment_emits_gap_and_acquisition_obligation() {
    let world = world_with_pnf(FragmentKind::Actor, "wiki:Q207:en:456", 7);
    let spec = spec(ConsumerRequirement {
        requirement_id: "need-patient".into(),
        fragment: FragmentKind::Patient,
        scope: RequirementScope::AnySource,
    });
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(
        &mut Cursor::new(world),
        &spec,
        &mut out,
        7,
    )
    .unwrap();
    assert_eq!(receipt.requirements_paid, 0);
    assert_eq!(receipt.requirements_unpaid, 1);
    assert_eq!(receipt.gaps_emitted, 1);
    assert_eq!(receipt.obligations_emitted, 1);

    let mut cursor = Cursor::new(out);
    let _original = decode_record(&mut cursor).unwrap().unwrap();
    let gap = decode_record(&mut cursor).unwrap().unwrap();
    let obligation = decode_record(&mut cursor).unwrap().unwrap();
    assert_eq!(gap.kind, WorldRecordKind::Gap);
    assert_eq!(obligation.kind, WorldRecordKind::Obligation);
    assert_eq!(gap.iteration_index, Some(7));
    assert_eq!(obligation.iteration_index, Some(7));
    assert_eq!(gap.aux1.as_deref(), Some("surface:test"));
    assert_eq!(obligation.aux1.as_deref(), Some("need-fragment-kind"));
    assert_eq!(&gap.payload[..4], b"GAP1");
    assert_eq!(&obligation.payload[..4], b"OBL1");
    assert_eq!(gap.payload[4], FragmentKind::Patient as u8);
    assert_eq!(obligation.payload[4], FragmentKind::Patient as u8);
}

#[test]
fn source_scoped_requirement_is_not_paid_by_other_source() {
    let world = world_with_pnf(FragmentKind::Actor, "wiki:QOTHER:en:1", 9);
    let spec = spec(ConsumerRequirement {
        requirement_id: "need-actor-q207".into(),
        fragment: FragmentKind::Actor,
        scope: RequirementScope::SourceManifestation("wiki:Q207:en:456".into()),
    });
    let mut out = Vec::new();
    let receipt = compile_consumer_residual_stream(
        &mut Cursor::new(world),
        &spec,
        &mut out,
        9,
    )
    .unwrap();
    assert_eq!(receipt.requirements_paid, 0);
    assert_eq!(receipt.requirements_unpaid, 1);
}

#[test]
fn consumer_residual_source_has_no_json_or_regex_dependency() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let cargo = std::fs::read_to_string(manifest).unwrap();
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
