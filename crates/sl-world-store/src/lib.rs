use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use postgres::{Client, NoTls, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorldStoreError {
    #[error("DATABASE_URL is not configured")]
    MissingDatabaseUrl,
    #[error("env file does not exist: {0}")]
    MissingEnvFile(PathBuf),
    #[error("env file error: {0}")]
    EnvFile(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("invalid world record: {0}")]
    InvalidRecord(String),
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    database_url: String,
}

impl DatabaseConfig {
    pub fn database_url(&self) -> &str {
        &self.database_url
    }

    pub fn redacted_description(&self) -> &'static str {
        "DATABASE_URL configured"
    }
}

pub fn load_database_config(explicit_env_file: Option<&Path>) -> Result<DatabaseConfig, WorldStoreError> {
    if let Ok(value) = env::var("DATABASE_URL") {
        if !value.trim().is_empty() {
            return Ok(DatabaseConfig { database_url: value });
        }
    }
    if let Some(path) = explicit_env_file {
        if !path.exists() {
            return Err(WorldStoreError::MissingEnvFile(path.to_path_buf()));
        }
        dotenvy::from_path(path).map_err(|e| WorldStoreError::EnvFile(e.to_string()))?;
        let value = env::var("DATABASE_URL").map_err(|_| WorldStoreError::MissingDatabaseUrl)?;
        if value.trim().is_empty() {
            return Err(WorldStoreError::MissingDatabaseUrl);
        }
        return Ok(DatabaseConfig { database_url: value });
    }
    Err(WorldStoreError::MissingDatabaseUrl)
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
const MERGE_SOURCE: &str = "INSERT INTO slr_world_source_manifestation (source_manifestation_id,source_kind,qid,language,revision_ref,source_text_sha256,payload) SELECT record_id,COALESCE(payload->>'manifestation_kind',payload->>'source_kind',''),COALESCE(payload->>'qid',''),COALESCE(payload->>'language',''),COALESCE(payload->>'revision_id',payload->>'revision_ref',''),COALESCE(payload->>'source_text_sha256',''),payload FROM slr_world_stage_record WHERE kind='source_manifestation' ON CONFLICT (source_manifestation_id) DO NOTHING";
const MERGE_PNF: &str = "INSERT INTO slr_world_pnf_candidate (claim_candidate_id,source_manifestation_id,payload) SELECT record_id,COALESCE(aux1,''),payload FROM slr_world_stage_record WHERE kind='pnf_candidate' ON CONFLICT (claim_candidate_id) DO NOTHING";
const MERGE_ATOM: &str = "INSERT INTO slr_world_atom (atom_id,atom_kind,subject_qid,source_manifestation_id,payload) SELECT record_id,COALESCE(payload->>'kind',''),COALESCE(payload->>'subject_qid',''),COALESCE(aux1,payload->>'document_ref',''),payload FROM slr_world_stage_record WHERE kind='world_atom' ON CONFLICT (atom_id) DO NOTHING";
const MERGE_GAP: &str = "INSERT INTO slr_world_gap (gap_id,iteration_index,surface_id,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_stage_record WHERE kind='gap' AND iteration_index IS NOT NULL ON CONFLICT (gap_id,iteration_index) DO NOTHING";
const MERGE_OBLIGATION: &str = "INSERT INTO slr_world_obligation (obligation_id,iteration_index,obligation_kind,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_stage_record WHERE kind='obligation' AND iteration_index IS NOT NULL ON CONFLICT (obligation_id,iteration_index) DO NOTHING";
const MERGE_ROUTE: &str = "INSERT INTO slr_world_route_action (action_id,iteration_index,payload) SELECT record_id,iteration_index,payload FROM slr_world_stage_record WHERE kind='route_action' AND iteration_index IS NOT NULL ON CONFLICT (action_id,iteration_index) DO NOTHING";
const MERGE_ITERATION: &str = "INSERT INTO slr_world_iteration (iteration_index,parent_iteration_index,payload) SELECT iteration_index,CASE WHEN iteration_index>0 THEN iteration_index-1 ELSE NULL END,payload FROM slr_world_stage_record WHERE kind='iteration' AND iteration_index IS NOT NULL ON CONFLICT (iteration_index) DO NOTHING";
const MERGES: [&str; 7] = [
    MERGE_SOURCE,
    MERGE_PNF,
    MERGE_ATOM,
    MERGE_GAP,
    MERGE_OBLIGATION,
    MERGE_ROUTE,
    MERGE_ITERATION,
];

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
CREATE TABLE IF NOT EXISTS slr_world_source_manifestation (source_manifestation_id TEXT PRIMARY KEY, source_kind TEXT NOT NULL, qid TEXT NOT NULL DEFAULT '', language TEXT NOT NULL DEFAULT '', revision_ref TEXT NOT NULL DEFAULT '', source_text_sha256 TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_pnf_candidate (claim_candidate_id TEXT PRIMARY KEY, source_manifestation_id TEXT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_atom (atom_id TEXT PRIMARY KEY, atom_kind TEXT NOT NULL, subject_qid TEXT NOT NULL DEFAULT '', source_manifestation_id TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_gap (gap_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, surface_id TEXT NOT NULL DEFAULT '', payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (gap_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_obligation (obligation_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, obligation_kind TEXT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (obligation_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_route_action (action_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (action_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_iteration (iteration_index BIGINT PRIMARY KEY, parent_iteration_index BIGINT, payload JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
"#
}

fn stage_sql() -> &'static str {
    "CREATE TEMP TABLE slr_world_stage_record (kind TEXT NOT NULL, record_id TEXT NOT NULL, iteration_index BIGINT, aux1 TEXT, payload JSONB NOT NULL) ON COMMIT DROP"
}

pub fn latest_iteration_sql() -> &'static str {
    "SELECT MAX(iteration_index) FROM slr_world_iteration"
}

pub fn frontier_gap_sql() -> &'static str {
    "SELECT gap_id, iteration_index, payload::text FROM slr_world_gap WHERE iteration_index=$1"
}

pub fn frontier_obligation_sql() -> &'static str {
    "SELECT obligation_id, iteration_index, payload::text FROM slr_world_obligation WHERE iteration_index=$1"
}

pub fn parse_world_record_line(line: &str) -> Result<WorldRecord, WorldStoreError> {
    let record: WorldRecord = serde_json::from_str(line)?;
    if record.id.trim().is_empty() {
        return Err(WorldStoreError::InvalidRecord("empty id".into()));
    }
    Ok(record)
}

pub fn stream_ndjson_records<R, F>(reader: R, mut on_record: F) -> Result<u64, WorldStoreError>
where
    R: BufRead,
    F: FnMut(WorldRecord) -> Result<(), WorldStoreError>,
{
    let mut count = 0;
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record = parse_world_record_line(&line)?;
        on_record(record)?;
        count += 1;
    }
    Ok(count)
}

fn stable_id(prefix: &str, value: &Value) -> String {
    let digest = Sha256::digest(serde_json::to_vec(value).expect("json"));
    format!("{prefix}{digest:x}")
}

fn string_field(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

pub fn records_from_round_values(
    article: &Value,
    closure: &Value,
    plan: &Value,
    iteration: &Value,
) -> Result<Vec<WorldRecord>, WorldStoreError> {
    let idx = iteration.get("iteration_index").and_then(Value::as_i64).unwrap_or(0);
    let mut out = Vec::new();

    for manifestation in article.get("article_manifestations").and_then(Value::as_array).into_iter().flatten() {
        let qid = string_field(manifestation, "qid");
        let language = string_field(manifestation, "language");
        let revision = manifestation.get("revision_id").map(|v| v.to_string()).unwrap_or_default();
        let document_ref = string_field(manifestation, "document_ref");
        let id = if document_ref.is_empty() { format!("wiki:{qid}:{language}:{revision}") } else { document_ref };
        out.push(WorldRecord {
            kind: WorldRecordKind::SourceManifestation,
            id,
            iteration_index: Some(idx),
            source_manifestation_id: None,
            surface_id: None,
            obligation_kind: None,
            payload: manifestation.clone(),
        });
    }

    for candidate in article.get("pnf_candidates").and_then(Value::as_array).into_iter().flatten() {
        let id = string_field(candidate, "claim_candidate_id");
        if !id.is_empty() {
            out.push(WorldRecord {
                kind: WorldRecordKind::PnfCandidate,
                id,
                iteration_index: Some(idx),
                source_manifestation_id: Some(string_field(candidate, "document_ref")),
                surface_id: None,
                obligation_kind: None,
                payload: candidate.clone(),
            });
        }
    }

    for atom in closure.get("canonical_atoms").and_then(Value::as_array).into_iter().flatten() {
        let id = string_field(atom, "atom_id");
        if !id.is_empty() {
            out.push(WorldRecord {
                kind: WorldRecordKind::WorldAtom,
                id,
                iteration_index: Some(idx),
                source_manifestation_id: Some(string_field(atom, "document_ref")),
                surface_id: None,
                obligation_kind: None,
                payload: atom.clone(),
            });
        }
    }

    for gap in closure.get("gaps").and_then(Value::as_array).into_iter().flatten() {
        let surface = string_field(gap, "surface_id");
        if let Some(missing) = gap.get("missing_atom_ids").and_then(Value::as_array) {
            for atom_id in missing.iter().filter_map(Value::as_str) {
                let mut payload = gap.clone();
                if let Some(object) = payload.as_object_mut() {
                    object.insert("missing_atom_id".into(), Value::String(atom_id.into()));
                }
                out.push(WorldRecord {
                    kind: WorldRecordKind::Gap,
                    id: format!("gap:{surface}:{atom_id}"),
                    iteration_index: Some(idx),
                    source_manifestation_id: None,
                    surface_id: Some(surface.clone()),
                    obligation_kind: None,
                    payload,
                });
            }
        } else if string_field(gap, "gap_kind") == "missing-surface" {
            out.push(WorldRecord {
                kind: WorldRecordKind::Gap,
                id: format!("gap:{surface}:missing-surface"),
                iteration_index: Some(idx),
                source_manifestation_id: None,
                surface_id: Some(surface),
                obligation_kind: None,
                payload: gap.clone(),
            });
        }
    }

    for obligation in closure.get("acquisition_obligations").and_then(Value::as_array).into_iter().flatten() {
        let mut id = string_field(obligation, "obligation_id");
        if id.is_empty() {
            id = stable_id("obligation:", obligation);
        }
        out.push(WorldRecord {
            kind: WorldRecordKind::Obligation,
            id,
            iteration_index: Some(idx),
            source_manifestation_id: None,
            surface_id: None,
            obligation_kind: Some(string_field(obligation, "obligation_kind")),
            payload: obligation.clone(),
        });
    }

    for action in plan.get("selected_route_actions").and_then(Value::as_array).into_iter().flatten() {
        let id = string_field(action, "action_id");
        if !id.is_empty() {
            out.push(WorldRecord {
                kind: WorldRecordKind::RouteAction,
                id,
                iteration_index: Some(idx),
                source_manifestation_id: None,
                surface_id: None,
                obligation_kind: None,
                payload: action.clone(),
            });
        }
    }

    out.push(WorldRecord {
        kind: WorldRecordKind::Iteration,
        id: format!("iteration:{idx}"),
        iteration_index: Some(idx),
        source_manifestation_id: None,
        surface_id: None,
        obligation_kind: None,
        payload: iteration.clone(),
    });
    Ok(out)
}

pub fn records_from_round_files(
    article: &Path,
    closure: &Path,
    plan: &Path,
    iteration: &Path,
) -> Result<Vec<WorldRecord>, WorldStoreError> {
    fn read(path: &Path) -> Result<Value, WorldStoreError> {
        Ok(serde_json::from_reader(BufReader::new(File::open(path)?))?)
    }
    records_from_round_values(&read(article)?, &read(closure)?, &read(plan)?, &read(iteration)?)
}

fn copy_text(value: Option<&str>) -> String {
    match value {
        None => "\\N".into(),
        Some(value) => value
            .replace('\\', "\\\\")
            .replace('\t', "\\t")
            .replace('\n', "\\n")
            .replace('\r', "\\r"),
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

fn write_stage_record<W: Write>(writer: &mut W, record: &WorldRecord) -> Result<(), WorldStoreError> {
    let iteration = record.iteration_index.map(|value| value.to_string());
    let payload = serde_json::to_string(&record.payload)?;
    writeln!(
        writer,
        "{}\t{}\t{}\t{}\t{}",
        copy_text(Some(record.kind.as_str())),
        copy_text(Some(&record.id)),
        copy_text(iteration.as_deref()),
        copy_text(aux1(record)),
        copy_text(Some(&payload)),
    )?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct IngestReceipt {
    pub records: u64,
    pub copy_streams: u64,
    pub ndjson_streaming: bool,
    pub postgres_persistence_is_semantic_authority: bool,
    pub semantic_promotion: bool,
}

pub struct WorldStore {
    client: Client,
}

impl WorldStore {
    pub fn connect(config: &DatabaseConfig) -> Result<Self, WorldStoreError> {
        Ok(Self { client: Client::connect(config.database_url(), NoTls)? })
    }

    pub fn ensure_schema(&mut self) -> Result<(), WorldStoreError> {
        self.client.batch_execute(world_schema_sql())?;
        Ok(())
    }

    fn finish_stage_transaction(
        mut tx: postgres::Transaction<'_>,
        records: u64,
        ndjson_streaming: bool,
    ) -> Result<IngestReceipt, WorldStoreError> {
        for sql in MERGES {
            tx.execute(sql, &[])?;
        }
        tx.commit()?;
        Ok(IngestReceipt {
            records,
            copy_streams: 1,
            ndjson_streaming,
            postgres_persistence_is_semantic_authority: false,
            semantic_promotion: false,
        })
    }

    pub fn ingest_records<I>(&mut self, records: I) -> Result<IngestReceipt, WorldStoreError>
    where
        I: IntoIterator<Item = WorldRecord>,
    {
        self.ensure_schema()?;
        let mut tx = self.client.transaction()?;
        tx.batch_execute(stage_sql())?;
        let mut copy = tx.copy_in("COPY slr_world_stage_record (kind,record_id,iteration_index,aux1,payload) FROM STDIN WITH (FORMAT text)")?;
        let mut count = 0;
        for record in records {
            write_stage_record(&mut copy, &record)?;
            count += 1;
        }
        copy.finish()?;
        Self::finish_stage_transaction(tx, count, false)
    }

    pub fn ingest_ndjson<R: BufRead>(&mut self, reader: R) -> Result<IngestReceipt, WorldStoreError> {
        self.ensure_schema()?;
        let mut tx = self.client.transaction()?;
        tx.batch_execute(stage_sql())?;
        let mut copy = tx.copy_in("COPY slr_world_stage_record (kind,record_id,iteration_index,aux1,payload) FROM STDIN WITH (FORMAT text)")?;
        let count = stream_ndjson_records(reader, |record| write_stage_record(&mut copy, &record))?;
        copy.finish()?;
        Self::finish_stage_transaction(tx, count, true)
    }

    pub fn write_latest_frontier<W: Write>(&mut self, writer: &mut W) -> Result<u64, WorldStoreError> {
        self.ensure_schema()?;
        let latest: Option<i64> = self.client.query_one(latest_iteration_sql(), &[])?.get(0);
        let Some(iteration_index) = latest else {
            return Ok(0);
        };

        let mut count = 0;
        {
            let rows = self.client.query_raw(frontier_gap_sql(), [&iteration_index])?;
            for row in rows {
                write_frontier_row(writer, "gap", &row?)?;
                count += 1;
            }
        }
        {
            let rows = self.client.query_raw(frontier_obligation_sql(), [&iteration_index])?;
            for row in rows {
                write_frontier_row(writer, "obligation", &row?)?;
                count += 1;
            }
        }
        Ok(count)
    }
}

fn write_frontier_row<W: Write>(writer: &mut W, kind: &str, row: &Row) -> Result<(), WorldStoreError> {
    let id: String = row.get(0);
    let iteration_index: i64 = row.get(1);
    let payload: String = row.get(2);
    write!(
        writer,
        "{{\"kind\":{},\"id\":{},\"iteration_index\":{},\"payload\":{},\"candidate_only\":true,\"semantic_promotion\":false}}\n",
        serde_json::to_string(kind)?,
        serde_json::to_string(&id)?,
        iteration_index,
        payload,
    )?;
    writer.flush()?;
    Ok(())
}
