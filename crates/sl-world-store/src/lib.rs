use std::env;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use postgres::binary_copy::BinaryCopyInWriter;
use postgres::fallible_iterator::FallibleIterator;
use postgres::types::Type;
use postgres::{Client, NoTls, Row};
use thiserror::Error;

pub const WIRE_MAGIC: [u8; 4] = *b"SLRW";
pub const WIRE_VERSION: u16 = 1;
const FLAG_ITERATION: u8 = 1 << 0;
const FLAG_AUX1: u8 = 1 << 1;
const MAX_ID_BYTES: usize = 1 << 20;
const MAX_AUX_BYTES: usize = 4 << 20;
const MAX_PAYLOAD_BYTES: usize = 64 << 20;

#[derive(Debug, Error)]
pub enum WorldStoreError {
    #[error("DATABASE_URL is not configured")]
    MissingDatabaseUrl,
    #[error("env file does not exist: {0}")]
    MissingEnvFile(PathBuf),
    #[error("env file error: {0}")]
    EnvFile(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("invalid wire record: {0}")]
    InvalidRecord(String),
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    database_url: String,
}
impl DatabaseConfig {
    pub fn database_url(&self) -> &str { &self.database_url }
    pub fn redacted_description(&self) -> &'static str { "DATABASE_URL configured" }
}

pub fn load_database_config(explicit_env_file: Option<&Path>) -> Result<DatabaseConfig, WorldStoreError> {
    if let Ok(value) = env::var("DATABASE_URL") {
        if !value.trim().is_empty() { return Ok(DatabaseConfig { database_url: value }); }
    }
    if let Some(path) = explicit_env_file {
        if !path.exists() { return Err(WorldStoreError::MissingEnvFile(path.to_path_buf())); }
        dotenvy::from_path(path).map_err(|e| WorldStoreError::EnvFile(e.to_string()))?;
        let value = env::var("DATABASE_URL").map_err(|_| WorldStoreError::MissingDatabaseUrl)?;
        if value.trim().is_empty() { return Err(WorldStoreError::MissingDatabaseUrl); }
        return Ok(DatabaseConfig { database_url: value });
    }
    Err(WorldStoreError::MissingDatabaseUrl)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldRecordKind {
    SourceManifestation = 1,
    PnfCandidate = 2,
    WorldAtom = 3,
    Gap = 4,
    Obligation = 5,
    RouteAction = 6,
    Iteration = 7,
    Payment = 8,
    Review = 9,
}
impl WorldRecordKind {
    pub fn from_u8(value: u8) -> Result<Self, WorldStoreError> {
        match value {
            1 => Ok(Self::SourceManifestation),
            2 => Ok(Self::PnfCandidate),
            3 => Ok(Self::WorldAtom),
            4 => Ok(Self::Gap),
            5 => Ok(Self::Obligation),
            6 => Ok(Self::RouteAction),
            7 => Ok(Self::Iteration),
            8 => Ok(Self::Payment),
            9 => Ok(Self::Review),
            _ => Err(WorldStoreError::InvalidRecord(format!("unknown kind {value}"))),
        }
    }
    pub fn as_i16(self) -> i16 { self as i16 }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireRecord {
    pub kind: WorldRecordKind,
    pub id: String,
    pub iteration_index: Option<i64>,
    pub aux1: Option<String>,
    pub payload: Vec<u8>,
}

pub fn encode_record<W: Write>(writer: &mut W, record: &WireRecord) -> Result<(), WorldStoreError> {
    let id = record.id.as_bytes();
    if id.len() > MAX_ID_BYTES { return Err(WorldStoreError::InvalidRecord("id too large".into())); }
    let aux = record.aux1.as_deref().unwrap_or("").as_bytes();
    if aux.len() > MAX_AUX_BYTES { return Err(WorldStoreError::InvalidRecord("aux too large".into())); }
    if record.payload.len() > MAX_PAYLOAD_BYTES { return Err(WorldStoreError::InvalidRecord("payload too large".into())); }
    let mut flags = 0u8;
    if record.iteration_index.is_some() { flags |= FLAG_ITERATION; }
    if record.aux1.is_some() { flags |= FLAG_AUX1; }
    writer.write_all(&WIRE_MAGIC)?;
    writer.write_all(&WIRE_VERSION.to_le_bytes())?;
    writer.write_all(&[record.kind as u8, flags])?;
    writer.write_all(&(id.len() as u32).to_le_bytes())?;
    writer.write_all(&(aux.len() as u32).to_le_bytes())?;
    writer.write_all(&(record.payload.len() as u32).to_le_bytes())?;
    writer.write_all(&record.iteration_index.unwrap_or(0).to_le_bytes())?;
    writer.write_all(id)?;
    writer.write_all(aux)?;
    writer.write_all(&record.payload)?;
    Ok(())
}

fn read_exact_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<bool, WorldStoreError> {
    let mut offset = 0;
    while offset < buf.len() {
        match reader.read(&mut buf[offset..]) {
            Ok(0) if offset == 0 => return Ok(false),
            Ok(0) => return Err(WorldStoreError::InvalidRecord("truncated frame".into())),
            Ok(n) => offset += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

pub fn decode_record<R: Read>(reader: &mut R) -> Result<Option<WireRecord>, WorldStoreError> {
    let mut magic = [0u8; 4];
    if !read_exact_or_eof(reader, &mut magic)? { return Ok(None); }
    if magic != WIRE_MAGIC { return Err(WorldStoreError::InvalidRecord("bad magic".into())); }
    let mut v = [0u8; 2]; reader.read_exact(&mut v)?;
    if u16::from_le_bytes(v) != WIRE_VERSION { return Err(WorldStoreError::InvalidRecord("unsupported version".into())); }
    let mut kf = [0u8; 2]; reader.read_exact(&mut kf)?;
    let kind = WorldRecordKind::from_u8(kf[0])?;
    let flags = kf[1];
    let mut b4 = [0u8; 4];
    reader.read_exact(&mut b4)?; let id_len = u32::from_le_bytes(b4) as usize;
    reader.read_exact(&mut b4)?; let aux_len = u32::from_le_bytes(b4) as usize;
    reader.read_exact(&mut b4)?; let payload_len = u32::from_le_bytes(b4) as usize;
    if id_len > MAX_ID_BYTES || aux_len > MAX_AUX_BYTES || payload_len > MAX_PAYLOAD_BYTES {
        return Err(WorldStoreError::InvalidRecord("declared field length exceeds limit".into()));
    }
    let mut b8 = [0u8; 8]; reader.read_exact(&mut b8)?;
    let raw_iteration = i64::from_le_bytes(b8);
    let mut id = vec![0u8; id_len]; reader.read_exact(&mut id)?;
    let mut aux = vec![0u8; aux_len]; reader.read_exact(&mut aux)?;
    let mut payload = vec![0u8; payload_len]; reader.read_exact(&mut payload)?;
    let id = String::from_utf8(id).map_err(|_| WorldStoreError::InvalidRecord("id is not UTF-8".into()))?;
    if id.is_empty() { return Err(WorldStoreError::InvalidRecord("empty id".into())); }
    let aux1 = if flags & FLAG_AUX1 != 0 {
        Some(String::from_utf8(aux).map_err(|_| WorldStoreError::InvalidRecord("aux is not UTF-8".into()))?)
    } else { None };
    let iteration_index = if flags & FLAG_ITERATION != 0 { Some(raw_iteration) } else { None };
    Ok(Some(WireRecord { kind, id, iteration_index, aux1, payload }))
}

const MERGE_SOURCE: &str = "INSERT INTO slr_world_v2_source_manifestation (source_manifestation_id,payload) SELECT record_id,payload FROM slr_world_v2_stage_record WHERE kind=1 ON CONFLICT (source_manifestation_id) DO NOTHING";
const MERGE_PNF: &str = "INSERT INTO slr_world_v2_pnf_candidate (claim_candidate_id,source_manifestation_id,payload) SELECT record_id,COALESCE(aux1,''),payload FROM slr_world_v2_stage_record WHERE kind=2 ON CONFLICT (claim_candidate_id) DO NOTHING";
const MERGE_ATOM: &str = "INSERT INTO slr_world_v2_atom (atom_id,source_manifestation_id,payload) SELECT record_id,COALESCE(aux1,''),payload FROM slr_world_v2_stage_record WHERE kind=3 ON CONFLICT (atom_id) DO NOTHING";
const MERGE_GAP: &str = "INSERT INTO slr_world_v2_gap (gap_id,iteration_index,surface_id,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_v2_stage_record WHERE kind=4 AND iteration_index IS NOT NULL ON CONFLICT (gap_id,iteration_index) DO NOTHING";
const MERGE_OBLIGATION: &str = "INSERT INTO slr_world_v2_obligation (obligation_id,iteration_index,obligation_kind,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_v2_stage_record WHERE kind=5 AND iteration_index IS NOT NULL ON CONFLICT (obligation_id,iteration_index) DO NOTHING";
const MERGE_ROUTE: &str = "INSERT INTO slr_world_v2_route_action (action_id,iteration_index,payload) SELECT record_id,iteration_index,payload FROM slr_world_v2_stage_record WHERE kind=6 AND iteration_index IS NOT NULL ON CONFLICT (action_id,iteration_index) DO NOTHING";
const MERGE_ITERATION: &str = "INSERT INTO slr_world_v2_iteration (iteration_index,payload) SELECT iteration_index,payload FROM slr_world_v2_stage_record WHERE kind=7 AND iteration_index IS NOT NULL ON CONFLICT (iteration_index) DO NOTHING";
const MERGE_PAYMENT: &str = "INSERT INTO slr_world_v2_payment (payment_id,iteration_index,target_residual_id,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_v2_stage_record WHERE kind=8 AND iteration_index IS NOT NULL ON CONFLICT (payment_id,iteration_index) DO NOTHING";
const MERGE_REVIEW: &str = "INSERT INTO slr_world_v2_review (review_id,iteration_index,evidence_reference,payload) SELECT record_id,iteration_index,COALESCE(aux1,''),payload FROM slr_world_v2_stage_record WHERE kind=9 AND iteration_index IS NOT NULL ON CONFLICT (review_id,iteration_index) DO NOTHING";
const MERGES: [&str; 9] = [MERGE_SOURCE, MERGE_PNF, MERGE_ATOM, MERGE_GAP, MERGE_OBLIGATION, MERGE_ROUTE, MERGE_ITERATION, MERGE_PAYMENT, MERGE_REVIEW];

pub fn world_schema_sql() -> &'static str { r#"
CREATE TABLE IF NOT EXISTS slr_world_v2_source_manifestation (source_manifestation_id TEXT PRIMARY KEY,payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_pnf_candidate (claim_candidate_id TEXT PRIMARY KEY,source_manifestation_id TEXT NOT NULL DEFAULT '',payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_atom (atom_id TEXT PRIMARY KEY,source_manifestation_id TEXT NOT NULL DEFAULT '',payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_gap (gap_id TEXT NOT NULL,iteration_index BIGINT NOT NULL,surface_id TEXT NOT NULL DEFAULT '',payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),PRIMARY KEY(gap_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_obligation (obligation_id TEXT NOT NULL,iteration_index BIGINT NOT NULL,obligation_kind TEXT NOT NULL DEFAULT '',payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),PRIMARY KEY(obligation_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_route_action (action_id TEXT NOT NULL,iteration_index BIGINT NOT NULL,payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),PRIMARY KEY(action_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_iteration (iteration_index BIGINT PRIMARY KEY,payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_payment (payment_id TEXT NOT NULL,iteration_index BIGINT NOT NULL,target_residual_id TEXT NOT NULL,payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),PRIMARY KEY(payment_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_review (review_id TEXT NOT NULL,iteration_index BIGINT NOT NULL,evidence_reference TEXT NOT NULL,payload BYTEA NOT NULL,created_at TIMESTAMPTZ NOT NULL DEFAULT now(),PRIMARY KEY(review_id,iteration_index));
CREATE INDEX IF NOT EXISTS slr_world_v2_payment_target_iteration_idx ON slr_world_v2_payment (target_residual_id,iteration_index);
CREATE INDEX IF NOT EXISTS slr_world_v2_gap_identity_iteration_idx ON slr_world_v2_gap (gap_id,iteration_index);
CREATE INDEX IF NOT EXISTS slr_world_v2_obligation_identity_iteration_idx ON slr_world_v2_obligation (obligation_id,iteration_index);
CREATE INDEX IF NOT EXISTS slr_world_v2_review_evidence_iteration_idx ON slr_world_v2_review (evidence_reference,iteration_index);
"# }
fn stage_sql() -> &'static str { "CREATE TEMP TABLE slr_world_v2_stage_record (kind SMALLINT NOT NULL,record_id TEXT NOT NULL,iteration_index BIGINT,aux1 TEXT,payload BYTEA NOT NULL) ON COMMIT DROP" }

pub fn latest_iteration_sql() -> &'static str { "SELECT MAX(iteration_index) FROM slr_world_v2_iteration" }

pub fn active_frontier_gap_sql() -> &'static str { r#"
SELECT g.gap_id,g.iteration_index,NULLIF(g.surface_id,''),g.payload
FROM slr_world_v2_gap g
WHERE g.iteration_index <= $1
  AND NOT EXISTS (
    SELECT 1 FROM slr_world_v2_gap newer
    WHERE newer.gap_id = g.gap_id
      AND newer.iteration_index > g.iteration_index
      AND newer.iteration_index <= $1
  )
  AND NOT EXISTS (
    SELECT 1 FROM slr_world_v2_payment p
    WHERE p.target_residual_id = g.gap_id
      AND p.iteration_index >= g.iteration_index
      AND p.iteration_index <= $1
  )
"# }

pub fn active_frontier_obligation_sql() -> &'static str { r#"
SELECT o.obligation_id,o.iteration_index,NULLIF(o.obligation_kind,''),o.payload
FROM slr_world_v2_obligation o
WHERE o.iteration_index <= $1
  AND NOT EXISTS (
    SELECT 1 FROM slr_world_v2_obligation newer
    WHERE newer.obligation_id = o.obligation_id
      AND newer.iteration_index > o.iteration_index
      AND newer.iteration_index <= $1
  )
  AND NOT EXISTS (
    SELECT 1 FROM slr_world_v2_payment p
    WHERE p.target_residual_id = o.obligation_id
      AND p.iteration_index >= o.iteration_index
      AND p.iteration_index <= $1
  )
"# }

pub fn frontier_gap_sql() -> &'static str { active_frontier_gap_sql() }
pub fn frontier_obligation_sql() -> &'static str { active_frontier_obligation_sql() }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestReceipt {
    pub records: u64,
    pub copy_streams: u64,
    pub binary_wire: bool,
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

    pub fn ingest_wire<R: Read>(&mut self, mut reader: R) -> Result<IngestReceipt, WorldStoreError> {
        self.ensure_schema()?;
        let mut tx = self.client.transaction()?;
        tx.batch_execute(stage_sql())?;
        let copy = tx.copy_in("COPY slr_world_v2_stage_record (kind,record_id,iteration_index,aux1,payload) FROM STDIN BINARY")?;
        let mut writer = BinaryCopyInWriter::new(copy, &[Type::INT2, Type::TEXT, Type::INT8, Type::TEXT, Type::BYTEA]);
        let mut count = 0u64;
        while let Some(record) = decode_record(&mut reader)? {
            let kind = record.kind.as_i16();
            writer.write(&[&kind, &record.id, &record.iteration_index, &record.aux1, &record.payload])?;
            count += 1;
        }
        writer.finish()?;
        for sql in MERGES {
            tx.execute(sql, &[])?;
        }
        tx.commit()?;
        Ok(IngestReceipt {
            records: count,
            copy_streams: 1,
            binary_wire: true,
            postgres_persistence_is_semantic_authority: false,
            semantic_promotion: false,
        })
    }

    pub fn write_latest_frontier<W: Write>(&mut self, writer: &mut W) -> Result<u64, WorldStoreError> {
        self.ensure_schema()?;
        let latest: Option<i64> = self.client.query_one(latest_iteration_sql(), &[])?.get(0);
        let Some(iteration_index) = latest else {
            return Ok(0);
        };
        let mut count = 0;
        {
            let mut rows = self.client.query_raw(active_frontier_gap_sql(), [&iteration_index])?;
            while let Some(row) = rows.next()? {
                let record = frontier_row(WorldRecordKind::Gap, row)?;
                encode_record(writer, &record)?;
                writer.flush()?;
                count += 1;
            }
        }
        {
            let mut rows = self.client.query_raw(active_frontier_obligation_sql(), [&iteration_index])?;
            while let Some(row) = rows.next()? {
                let record = frontier_row(WorldRecordKind::Obligation, row)?;
                encode_record(writer, &record)?;
                writer.flush()?;
                count += 1;
            }
        }
        Ok(count)
    }
}

fn frontier_row(kind: WorldRecordKind, row: Row) -> Result<WireRecord, WorldStoreError> {
    let id: String = row.get(0);
    let iteration_index: i64 = row.get(1);
    let aux1: Option<String> = row.get(2);
    let payload: Vec<u8> = row.get(3);
    Ok(WireRecord { kind, id, iteration_index: Some(iteration_index), aux1, payload })
}
