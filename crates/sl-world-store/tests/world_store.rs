use sensiblaw_world_store::{
    copy_target_for_kind, frontier_gap_sql, frontier_obligation_sql, latest_iteration_sql,
    parse_world_record_line, records_from_round_values, stream_ndjson_records, WorldRecordKind,
};
use serde_json::json;
use std::io::{BufReader, Cursor};

#[test]
fn parses_compact_pnf_record_without_promoting_truth() {
    let line = r#"{"kind":"pnf_candidate","id":"pnf-candidate:abc","source_manifestation_id":"wiki:Q207:en:456","payload":{"claim_truth_promoted":false,"candidate_only":true}}"#;
    let record = parse_world_record_line(line).expect("compact record");
    assert_eq!(record.kind, WorldRecordKind::PnfCandidate);
    assert_eq!(record.id, "pnf-candidate:abc");
    assert_eq!(record.source_manifestation_id.as_deref(), Some("wiki:Q207:en:456"));
    assert_eq!(record.payload["claim_truth_promoted"], false);
    assert_eq!(record.payload["candidate_only"], true);
}

#[test]
fn maps_record_kinds_to_append_only_staging_targets() {
    let cases = [
        (WorldRecordKind::SourceManifestation, "slr_world_source_manifestation"),
        (WorldRecordKind::PnfCandidate, "slr_world_pnf_candidate"),
        (WorldRecordKind::WorldAtom, "slr_world_atom"),
        (WorldRecordKind::Gap, "slr_world_gap"),
        (WorldRecordKind::Obligation, "slr_world_obligation"),
        (WorldRecordKind::RouteAction, "slr_world_route_action"),
        (WorldRecordKind::Iteration, "slr_world_iteration"),
    ];
    for (kind, table) in cases {
        let target = copy_target_for_kind(kind);
        assert_eq!(target.final_table, table);
        assert_eq!(target.staging_table, "slr_world_stage_record");
        assert!(target.merge_sql.contains("ON CONFLICT"));
        assert!(target.merge_sql.contains("DO NOTHING"));
        assert!(!target.merge_sql.contains("DO UPDATE"));
        assert!(!target.merge_sql.contains("DELETE"));
    }
}

#[test]
fn frontier_queries_are_iteration_scoped_streamable_and_read_only() {
    assert!(latest_iteration_sql().contains("MAX(iteration_index)"));
    for sql in [frontier_gap_sql(), frontier_obligation_sql()] {
        assert!(sql.contains("iteration_index=$1"));
        let upper = sql.to_ascii_uppercase();
        assert!(!upper.contains("UPDATE "));
        assert!(!upper.contains("DELETE "));
        assert!(!upper.contains("ORDER BY"));
        assert!(!upper.contains("UNION"));
    }
}

#[test]
fn ndjson_projection_is_record_streaming_not_whole_input_buffering() {
    let first = r#"{"kind":"world_atom","id":"atom:1","payload":{"candidate_only":true,"semantic_promotion":false}}"#;
    let bad = "{not-json}";
    let input = format!("{first}\n{bad}\n");
    let reader = BufReader::new(Cursor::new(input.into_bytes()));
    let mut seen = Vec::new();
    let result = stream_ndjson_records(reader, |record| {
        seen.push(record.id);
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(seen, vec!["atom:1"]);
}

#[test]
fn projects_round_artifacts_in_rust() {
    let article = json!({
        "article_manifestations": [{"qid":"Q207","language":"en","revision_id":456,"manifestation_kind":"wikipedia-revision-text","source_text_sha256":"abc"}],
        "pnf_candidates": [{"claim_candidate_id":"pnf-candidate:1","document_ref":"wiki:Q207:en:456","candidate_only":true,"semantic_promotion":false}]
    });
    let closure = json!({
        "canonical_atoms": [{"atom_id":"atom:1","kind":"pnf-candidate","document_ref":"wiki:Q207:en:456"}],
        "gaps": [{"surface_id":"Q207:fr","missing_atom_ids":["atom:1"]}],
        "acquisition_obligations": [{"obligation_id":"obl:1","obligation_kind":"follow-related-qid"}]
    });
    let plan = json!({"selected_route_actions":[{"action_id":"Q207:P279:Q5"}]});
    let iteration = json!({"iteration_index":4,"candidate_only":true,"semantic_promotion":false});
    let records = records_from_round_values(&article, &closure, &plan, &iteration).expect("round records");
    for kind in [
        WorldRecordKind::SourceManifestation, WorldRecordKind::PnfCandidate,
        WorldRecordKind::WorldAtom, WorldRecordKind::Gap, WorldRecordKind::Obligation,
        WorldRecordKind::RouteAction, WorldRecordKind::Iteration,
    ] {
        assert_eq!(records.iter().filter(|r| r.kind == kind).count(), 1, "{kind:?}");
    }
}

#[test]
fn storage_never_promotes_semantic_authority() {
    let line = r#"{"kind":"world_atom","id":"atom:1","payload":{"postgres_persistence_is_semantic_authority":false,"semantic_promotion":false}}"#;
    let record = parse_world_record_line(line).expect("world atom");
    assert_eq!(record.payload["postgres_persistence_is_semantic_authority"], false);
    assert_eq!(record.payload["semantic_promotion"], false);
}
