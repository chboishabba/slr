use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;

use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

#[derive(Debug, Error)]
pub enum WorldStreamError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("invalid world record: {0}")]
    InvalidRecord(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldRecordKind {
    SourceManifestation,
    PnfCandidate,
    WorldAtom,
    Gap,
    Obligation,
    RouteAction,
    Iteration,
}

impl WorldRecordKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::SourceManifestation => "source_manifestation",
            Self::PnfCandidate => "pnf_candidate",
            Self::WorldAtom => "world_atom",
            Self::Gap => "gap",
            Self::Obligation => "obligation",
            Self::RouteAction => "route_action",
            Self::Iteration => "iteration",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldRecord {
    pub kind: WorldRecordKind,
    pub id: String,
    #[serde(default)]
    pub iteration_index: Option<i64>,
    #[serde(default)]
    pub source_manifestation_id: Option<String>,
    #[serde(default)]
    pub surface_id: Option<String>,
    #[serde(default)]
    pub obligation_kind: Option<String>,
    pub payload: Value,
}

pub struct CopyTarget {
    pub staging_table: &'static str,
    pub final_table: &'static str,
    pub merge_sql: &'static str,
}

const STAGE: &str = "slr_world_stage_record";

const MERGE_SOURCE: &str = r#"
INSERT INTO slr_world_source_manifestation
  (source_manifestation_id, source_kind, qid, language, revision_ref, source_text_sha256, payload)
SELECT record_id,
       COALESCE(payload->>'manifestation_kind', payload->>'source_kind', ''),
       COALESCE(payload->>'qid',''), COALESCE(payload->>'language',''),
       COALESCE(payload->>'revision_id', payload->>'revision_ref', ''),
       COALESCE(payload->>'source_text_sha256',''), payload
FROM slr_world_stage_record WHERE kind='source_manifestation'
ON CONFLICT (source_manifestation_id) DO NOTHING
"#;

const MERGE_PNF: &str = r#"
INSERT INTO slr_world_pnf_candidate (claim_candidate_id, source_manifestation_id, payload)
SELECT record_id, COALESCE(aux1,''), payload
FROM slr_world_stage_record WHERE kind='pnf_candidate'
ON CONFLICT (claim_candidate_id) DO NOTHING
"#;

const MERGE_ATOM: &str = r#"
INSERT INTO slr_world_atom (atom_id, atom_kind, subject_qid, source_manifestation_id, payload)
SELECT record_id, COALESCE(payload->>'kind',''), COALESCE(payload->>'subject_qid',''),
       COALESCE(aux1, payload->>'document_ref',''), payload
FROM slr_world_stage_record WHERE kind='world_atom'
ON CONFLICT (atom_id) DO NOTHING
"#;

const MERGE_GAP: &str = r#"
INSERT INTO slr_world_gap (gap_id, iteration_index, surface_id, payload)
SELECT record_id, iteration_index, COALESCE(aux1,''), payload
FROM slr_world_stage_record WHERE kind='gap' AND iteration_index IS NOT NULL
ON CONFLICT (gap_id, iteration_index) DO NOTHING
"#;

const MERGE_OBLIGATION: &str = r#"
INSERT INTO slr_world_obligation (obligation_id, iteration_index, obligation_kind, payload)
SELECT record_id, iteration_index, COALESCE(aux1,''), payload
FROM slr_world_stage_record WHERE kind='obligation' AND iteration_index IS NOT NULL
ON CONFLICT (obligation_id, iteration_index) DO NOTHING
"#;

const MERGE_ROUTE: &str = r#"
INSERT INTO slr_world_route_action (action_id, iteration_index, payload)
SELECT record_id, iteration_index, payload
FROM slr_world_stage_record WHERE kind='route_action' AND iteration_index IS NOT NULL
ON CONFLICT (action_id, iteration_index) DO NOTHING
"#;

const MERGE_ITERATION: &str = r#"
INSERT INTO slr_world_iteration (iteration_index, parent_iteration_index, payload)
SELECT iteration_index,
       CASE WHEN iteration_index > 0 THEN iteration_index - 1 ELSE NULL END,
       payload
FROM slr_world_stage_record WHERE kind='iteration' AND iteration_index IS NOT NULL
ON CONFLICT (iteration_index) DO NOTHING
"#;

pub fn copy_target_for_kind(kind: WorldRecordKind) -> CopyTarget {
    match kind {
        WorldRecordKind::SourceManifestation => CopyTarget { staging_table: STAGE, final_table: "slr_world_source_manifestation", merge_sql: MERGE_SOURCE },
        WorldRecordKind::PnfCandidate => CopyTarget { staging_table: STAGE, final_table: "slr_world_pnf_candidate", merge_sql: MERGE_PNF },
        WorldRecordKind::WorldAtom => CopyTarget { staging_table: STAGE, final_table: "slr_world_atom", merge_sql: MERGE_ATOM },
        WorldRecordKind::Gap => CopyTarget { staging_table: STAGE, final_table: "slr_world_gap", merge_sql: MERGE_GAP },
        WorldRecordKind::Obligation => CopyTarget { staging_table: STAGE, final_table: "slr_world_obligation", merge_sql: MERGE_OBLIGATION },
        WorldRecordKind::RouteAction => CopyTarget { staging_table: STAGE, final_table: "slr_world_route_action", merge_sql: MERGE_ROUTE },
        WorldRecordKind::Iteration => CopyTarget { staging_table: STAGE, final_table: "slr_world_iteration", merge_sql: MERGE_ITERATION },
    }
}

pub fn world_schema_sql() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS slr_world_source_manifestation (
 source_manifestation_id TEXT PRIMARY KEY, source_kind TEXT NOT NULL, qid TEXT NOT NULL DEFAULT '',
 language TEXT NOT NULL DEFAULT '', revision_ref TEXT NOT NULL DEFAULT '', source_text_sha256 TEXT NOT NULL DEFAULT '',
 payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_pnf_candidate (
 claim_candidate_id TEXT PRIMARY KEY, source_manifestation_id TEXT NOT NULL, payload JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_atom (
 atom_id TEXT PRIMARY KEY, atom_kind TEXT NOT NULL, subject_qid TEXT NOT NULL DEFAULT '',
 source_manifestation_id TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_gap (
 gap_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, surface_id TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (gap_id, iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_obligation (
 obligation_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, obligation_kind TEXT NOT NULL, payload JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (obligation_id, iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_route_action (
 action_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, payload JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (action_id, iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_iteration (
 iteration_index BIGINT PRIMARY KEY, parent_iteration_index BIGINT, payload JSONB NOT NULL,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now());
"#
}

fn stage_sql() -> &'static str {
    r#"CREATE TEMP TABLE slr_world_stage_record (
kind TEXT NOT NULL,
record_id TEXT NOT NULL,
iteration_index BIGINT,
aux1 TEXT,
payload JSONB NOT NULL
) ON COMMIT DROP"#
}

pub fn latest_frontier_sql() -> &'static str {
    r#"
WITH latest AS (SELECT MAX(iteration_index) AS iteration_index FROM slr_world_iteration)
SELECT 'gap'::text AS kind, g.gap_id AS id, g.iteration_index, g.payload
FROM slr_world_gap g, latest l WHERE g.iteration_index=l.iteration_index
UNION ALL
SELECT 'obligation'::text AS kind, o.obligation_id AS id, o.iteration_index, o.payload
FROM slr_world_obligation o, latest l WHERE o.iteration_index=l.iteration_index
ORDER BY kind, id
"#
}

pub fn parse_world_record_line(line: &str) -> Result<WorldRecord, WorldStreamError> {
    let record: WorldRecord = serde_json::from_str(line)?;
    if record.id.trim().is_empty() {
        return Err(WorldStreamError::InvalidRecord("empty id".to_string()));
    }
    Ok(record)
}

fn stable_id(prefix: &str, value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serializable JSON value");
    let digest = Sha256::digest(bytes);
    format!("{prefix}{digest:x}")
}

fn string_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

fn i64_field(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

pub fn records_from_round_values(
    article: &Value,
    closure: &Value,
    route_plan: &Value,
    iteration: &Value,
) -> Result<Vec<WorldRecord>, WorldStreamError> {
    let idx = i64_field(iteration, "iteration_index").unwrap_or(0);
    let mut out = Vec::new();

    for m in article.get("article_manifestations").and_then(Value::as_array).into_iter().flatten() {
        let qid = string_field(m, "qid");
        let language = string_field(m, "language");
        let revision = m.get("revision_id").map(|x| x.to_string()).unwrap_or_default().trim_matches('"').to_string();
        let id = if !string_field(m, "document_ref").is_empty() {
            string_field(m, "document_ref")
        } else {
            format!("wiki:{qid}:{language}:{revision}")
        };
        out.push(WorldRecord { kind: WorldRecordKind::SourceManifestation, id, iteration_index: Some(idx), source_manifestation_id: None, surface_id: None, obligation_kind: None, payload: m.clone() });
    }
    for c in article.get("pnf_candidates").and_then(Value::as_array).into_iter().flatten() {
        let id = string_field(c, "claim_candidate_id");
        if id.is_empty() { continue; }
        out.push(WorldRecord { kind: WorldRecordKind::PnfCandidate, id, iteration_index: Some(idx), source_manifestation_id: Some(string_field(c, "document_ref")), surface_id: None, obligation_kind: None, payload: c.clone() });
    }
    for atom in closure.get("canonical_atoms").and_then(Value::as_array).into_iter().flatten() {
        let id = string_field(atom, "atom_id");
        if id.is_empty() { continue; }
        out.push(WorldRecord { kind: WorldRecordKind::WorldAtom, id, iteration_index: Some(idx), source_manifestation_id: Some(string_field(atom, "document_ref")), surface_id: None, obligation_kind: None, payload: atom.clone() });
    }
    for gap in closure.get("gaps").and_then(Value::as_array).into_iter().flatten() {
        let surface = string_field(gap, "surface_id");
        if let Some(missing) = gap.get("missing_atom_ids").and_then(Value::as_array) {
            for atom in missing.iter().filter_map(Value::as_str) {
                let payload = if let Some(obj) = gap.as_object() {
                    let mut m = obj.clone(); m.insert("missing_atom_id".to_string(), Value::String(atom.to_string())); Value::Object(m)
                } else { gap.clone() };
                out.push(WorldRecord { kind: WorldRecordKind::Gap, id: format!("gap:{surface}:{atom}"), iteration_index: Some(idx), source_manifestation_id: None, surface_id: Some(surface.clone()), obligation_kind: None, payload });
            }
        } else if string_field(gap, "gap_kind") == "missing-surface" {
            out.push(WorldRecord { kind: WorldRecordKind::Gap, id: format!("gap:{surface}:missing-surface"), iteration_index: Some(idx), source_manifestation_id: None, surface_id: Some(surface), obligation_kind: None, payload: gap.clone() });
        }
    }
    for obligation in closure.get("acquisition_obligations").and_then(Value::as_array).into_iter().flatten() {
        let mut id = string_field(obligation, "obligation_id");
        if id.is_empty() { id = stable_id("obligation:", obligation); }
        out.push(WorldRecord { kind: WorldRecordKind::Obligation, id, iteration_index: Some(idx), source_manifestation_id: None, surface_id: None, obligation_kind: Some(string_field(obligation, "obligation_kind")), payload: obligation.clone() });
    }
    for action in route_plan.get("selected_route_actions").and_then(Value::as_array).into_iter().flatten() {
        let id = string_field(action, "action_id");
        if id.is_empty() { continue; }
        out.push(WorldRecord { kind: WorldRecordKind::RouteAction, id, iteration_index: Some(idx), source_manifestation_id: None, surface_id: None, obligation_kind: None, payload: action.clone() });
    }
    out.push(WorldRecord { kind: WorldRecordKind::Iteration, id: format!("iteration:{idx}"), iteration_index: Some(idx), source_manifestation_id: None, surface_id: None, obligation_kind: None, payload: iteration.clone() });
    Ok(out)
}

pub fn records_from_round_files(
    article_path: &Path,
    closure_path: &Path,
    route_plan_path: &Path,
    iteration_path: &Path,
) -> Result<Vec<WorldRecord>, WorldStreamError> {
    fn read(path: &Path) -> Result<Value, WorldStreamError> {
        let file = File::open(path)?;
        Ok(serde_json::from_reader(BufReader::new(file))?)
    }
    records_from_round_values(&read(article_path)?, &read(closure_path)?, &read(route_plan_path)?, &read(iteration_path)?)
}

fn copy_text(value: Option<&str>) -> String {
    match value {
        None => "\\N".to_string(),
        Some(v) => v.replace('\\', "\\\\").replace('\t', "\\t").replace('\n', "\\n").replace('\r', "\\r"),
    }
}

fn aux1(record: &WorldRecord) -> Option<&str> {
    match record.kind {
        WorldRecordKind::PnfCandidate | WorldRecordKind::WorldAtom => record.source_manifestation_id.as_deref(),
        WorldRecordKind::Gap => record.surface_id.as_deref(),
        WorldRecordKind::Obligation => record.obligation_kind.as_deref(),
        _ => None,
    }
}

pub struct IngestReceipt {
    pub records: u64,
    pub copy_streams: u64,
}

pub struct WorldStreamStore {
    client: Client,
}

impl WorldStreamStore {
    pub fn connect(config: &DatabaseConfig) -> Result<Self, WorldStreamError> {
        Ok(Self { client: Client::connect(config.database_url(), NoTls)? })
    }

    pub fn ensure_schema(&mut self) -> Result<(), WorldStreamError> {
        self.client.batch_execute(world_schema_sql())?;
        Ok(())
    }

    pub fn ingest_records<I>(&mut self, records: I) -> Result<IngestReceipt, WorldStreamError>
    where I: IntoIterator<Item = WorldRecord> {
        self.ensure_schema()?;
        let mut tx = self.client.transaction()?;
        tx.batch_execute(stage_sql())?;
        let mut writer = tx.copy_in("COPY slr_world_stage_record (kind,record_id,iteration_index,aux1,payload) FROM STDIN WITH (FORMAT text)")?;
        let mut count = 0u64;
        for record in records {
            let iteration = record.iteration_index.map(|x| x.to_string());
            let payload = serde_json::to_string(&record.payload)?;
            writeln!(writer, "{}\t{}\t{}\t{}\t{}",
                copy_text(Some(record.kind.as_str())),
                copy_text(Some(&record.id)),
                copy_text(iteration.as_deref()),
                copy_text(aux1(&record)),
                copy_text(Some(&payload)))?;
            count += 1;
        }
        writer.finish()?;
        for sql in [MERGE_SOURCE, MERGE_PNF, MERGE_ATOM, MERGE_GAP, MERGE_OBLIGATION, MERGE_ROUTE, MERGE_ITERATION] {
            tx.execute(sql, &[])?;
        }
        tx.commit()?;
        Ok(IngestReceipt { records: count, copy_streams: 1 })
    }

    pub fn ingest_ndjson<R: BufRead>(&mut self, reader: R) -> Result<IngestReceipt, WorldStreamError> {
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            records.push(parse_world_record_line(&line)?);
        }
        self.ingest_records(records)
    }

    pub fn latest_frontier(&mut self) -> Result<Vec<Value>, WorldStreamError> {
        self.ensure_schema()?;
        let rows = self.client.query(latest_frontier_sql(), &[])?;
        Ok(rows.into_iter().map(|row| {
            let kind: String = row.get(0);
            let id: String = row.get(1);
            let iteration_index: i64 = row.get(2);
            let payload: Value = row.get(3);
            json!({"kind":kind,"id":id,"iteration_index":iteration_index,"payload":payload,
                   "candidate_only":true,"semantic_promotion":false})
        }).collect())
    }
}

pub fn read_ndjson_file(path: &Path) -> Result<BufReader<File>, WorldStreamError> {
    Ok(BufReader::new(File::open(path)?))
}
