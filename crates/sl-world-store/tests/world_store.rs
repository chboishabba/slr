use sensiblaw_world_store::{
    decode_record, encode_record, frontier_gap_sql, frontier_obligation_sql,
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
    ] {
        assert!(sql.contains(table), "missing {table}");
    }
    let upper = sql.to_ascii_uppercase();
    assert!(upper.contains("BYTEA"));
    assert!(!upper.contains("JSON"));
    assert!(!upper.contains("UPDATE "));
    assert!(!upper.contains("DELETE "));
}

#[test]
fn frontier_queries_are_iteration_scoped_streamable_binary_and_read_only() {
    assert!(latest_iteration_sql().contains("MAX(iteration_index)"));
    for sql in [frontier_gap_sql(), frontier_obligation_sql()] {
        assert!(sql.contains("iteration_index=$1"));
        let upper = sql.to_ascii_uppercase();
        assert!(!upper.contains("UPDATE "));
        assert!(!upper.contains("DELETE "));
        assert!(!upper.contains("ORDER BY"));
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
