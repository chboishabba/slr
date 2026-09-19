use sensiblaw_world_store::{
    active_frontier_gap_sql, active_frontier_obligation_sql, decode_record, encode_record,
    latest_iteration_sql, world_schema_sql, WireRecord, WorldRecordKind, WIRE_MAGIC, WIRE_VERSION,
};
use std::io::Cursor;

#[test]
fn binary_wire_round_trip_preserves_core_coordinates() {
    let record = WireRecord {
        kind: WorldRecordKind::PnfCandidate,
        id: "pnf-candidate:abc".into(),
        iteration_index: Some(4),
        aux1: Some("wiki:Q207:en:456".into()),
        payload: vec![1, 2, 3, 4, 5],
    };
    let mut bytes = Vec::new();
    encode_record(&mut bytes, &record).expect("encode");
    assert_eq!(&bytes[..4], &WIRE_MAGIC);
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), WIRE_VERSION);
    let decoded = decode_record(&mut Cursor::new(bytes)).expect("decode").expect("record");
    assert_eq!(decoded, record);
}

#[test]
fn payment_and_review_are_first_class_binary_world_records() {
    for (kind, expected_tag, id, aux, magic) in [
        (WorldRecordKind::Payment, 8u8, "payment:test", "gap:test", b"PAY2".as_slice()),
        (WorldRecordKind::Review, 9u8, "review:test", "evidence:test", b"RVW1".as_slice()),
    ] {
        let record = WireRecord {
            kind,
            id: id.into(),
            iteration_index: Some(5),
            aux1: Some(aux.into()),
            payload: magic.to_vec(),
        };
        let mut bytes = Vec::new();
        encode_record(&mut bytes, &record).unwrap();
        let decoded = decode_record(&mut Cursor::new(bytes)).unwrap().unwrap();
        assert_eq!(decoded, record);
        assert_eq!(decoded.kind as u8, expected_tag);
    }
}

#[test]
fn binary_wire_is_incremental_not_whole_stream_buffered() {
    let first = WireRecord { kind: WorldRecordKind::WorldAtom, id: "atom:1".into(), iteration_index: Some(4), aux1: None, payload: vec![7,8,9] };
    let second = WireRecord { kind: WorldRecordKind::Gap, id: "gap:1".into(), iteration_index: Some(4), aux1: Some("Q207:fr".into()), payload: vec![10,11] };
    let mut bytes = Vec::new();
    encode_record(&mut bytes, &first).unwrap();
    encode_record(&mut bytes, &second).unwrap();
    let mut cursor = Cursor::new(bytes);
    assert_eq!(decode_record(&mut cursor).unwrap().unwrap(), first);
    assert_eq!(decode_record(&mut cursor).unwrap().unwrap(), second);
    assert!(decode_record(&mut cursor).unwrap().is_none());
}

#[test]
fn v2_schema_is_append_only_binary_storage() {
    let sql = world_schema_sql();
    for table in [
        "slr_world_v2_source_manifestation",
        "slr_world_v2_pnf_candidate",
        "slr_world_v2_atom",
        "slr_world_v2_gap",
        "slr_world_v2_obligation",
        "slr_world_v2_route_action",
        "slr_world_v2_iteration",
        "slr_world_v2_payment",
        "slr_world_v2_review",
    ] {
        assert!(sql.contains(table), "missing {table}");
    }
    assert!(sql.contains("target_residual_id"));
    assert!(sql.contains("evidence_reference"));
    let upper = sql.to_ascii_uppercase();
    assert!(upper.contains("BYTEA"));
    assert!(!upper.contains("JSON"));
    assert!(!upper.contains("UPDATE "));
    assert!(!upper.contains("DELETE "));
}

#[test]
fn active_frontier_is_derived_without_deleting_historical_residuals() {
    assert!(latest_iteration_sql().contains("MAX(iteration_index)"));
    for sql in [active_frontier_gap_sql(), active_frontier_obligation_sql()] {
        assert!(sql.contains("NOT EXISTS"));
        assert!(sql.contains("slr_world_v2_payment"));
        assert!(sql.contains("target_residual_id"));
        assert!(sql.contains("p.iteration_index >="));
        assert!(sql.contains("newer.iteration_index >"));
        let upper = sql.to_ascii_uppercase();
        assert!(!upper.contains("UPDATE "));
        assert!(!upper.contains("DELETE "));
        assert!(!upper.contains("UNION"));
        assert!(!upper.contains("JSON"));
    }
}

#[test]
fn storage_transport_has_no_json_or_regex_contract() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("world-store Cargo.toml");
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
