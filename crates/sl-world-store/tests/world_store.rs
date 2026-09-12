use sensiblaw_world_store::{
    copy_target_for_kind, decode_record, encode_record, frontier_gap_sql, frontier_obligation_sql,
    latest_iteration_sql, WireRecord, WorldRecordKind, WIRE_MAGIC, WIRE_VERSION,
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
fn maps_record_kinds_to_v2_append_only_binary_targets() {
    let cases = [
        (WorldRecordKind::SourceManifestation, "slr_world_v2_source_manifestation"),
        (WorldRecordKind::PnfCandidate, "slr_world_v2_pnf_candidate"),
        (WorldRecordKind::WorldAtom, "slr_world_v2_atom"),
        (WorldRecordKind::Gap, "slr_world_v2_gap"),
        (WorldRecordKind::Obligation, "slr_world_v2_obligation"),
        (WorldRecordKind::RouteAction, "slr_world_v2_route_action"),
        (WorldRecordKind::Iteration, "slr_world_v2_iteration"),
    ];
    for (kind, table) in cases {
        let target = copy_target_for_kind(kind);
        assert_eq!(target.final_table, table);
        assert_eq!(target.staging_table, "slr_world_v2_stage_record");
        assert!(target.merge_sql.contains("ON CONFLICT"));
        assert!(target.merge_sql.contains("DO NOTHING"));
        assert!(!target.merge_sql.contains("DO UPDATE"));
        assert!(!target.merge_sql.contains("DELETE"));
        assert!(!target.merge_sql.contains("JSON"));
    }
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
fn storage_transport_has_no_json_contract() {
    let cargo = std::fs::read_to_string("crates/sl-world-store/Cargo.toml").expect("Cargo.toml");
    assert!(!cargo.contains("serde_json"));
    assert!(!cargo.contains("regex"));
}
