use std::env;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use postgres::binary_copy::BinaryCopyInWriter;
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
    InvalidWire(String),
    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WorldRecordKind {
    SourceManifestation = 1,
    PnfCandidate = 2,
    WorldAtom = 3,
    Gap = 4,
    Obligation = 5,
    RouteAction = 6,
    Iteration = 7,
}

impl WorldRecordKind {
    fn from_u8(value: u8) -> Result<Self, WorldStoreError> {
        match value {
            1 => Ok(Self::SourceManifestation),
            2 => Ok(Self::PnfCandidate),
            3 => Ok(Self::WorldAtom),
            4 => Ok(Self::Gap),
            5 => Ok(Self::Obligation),
            6 => Ok(Self::RouteAction),
            7 => Ok(Self::Iteration),
            _ => Err(WorldStoreError::InvalidWire(format!("unknown record kind {value}"))),
        }
    }

    fn as_i16(self) -> i16 {
        self as u8 as i16
    }
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
    let aux = record.aux1.as_deref().unwrap_or("").as_bytes();
    if id.len() > MAX_ID_BYTES || aux.len() > MAX_AUX_BYTES || record.payload.len() > MAX_PAYLOAD_BYTES {
        return Err(WorldStoreError::InvalidWire("wire field exceeds bounded size".into()));
    }
    let mut flags = 0u8;
    if record.iteration_index.is_some() {
        flags |= FLAG_ITERATION;
    }
    if record.aux1.is_some() {
        flags |= FLAG_AUX1;
    }
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

fn read_exact_or_eof<R: Read>(reader: &mut R, first: &mut [u8; 1]) -> Result<bool, WorldStoreError> {
    match reader.read(first) {
        Ok(0) => Ok(false),
        Ok(1) => Ok(true),
        Ok(_) => unreachable!(),
        Err(e) if e.kind() == ErrorKind::Interrupted => read_exact_or_eof(reader, first),
        Err(e) => Err(WorldStoreError::Io(e)),
    }
}

pub fn decode_record<R: Read>(reader: &mut R) -> Result<Option<WireRecord>, WorldStoreError> {
    let mut first = [0u8; 1];
    if !read_exact_or_eof(reader, &mut first)? {
        return Ok(None);
    }
    let mut magic = [0u8; 4];
    magic[0] = first[0];
    reader.read_exact(&mut magic[1..])?;
    if magic != WIRE_MAGIC {
        return Err(WorldStoreError::InvalidWire("bad magic".into()));
    }
    let mut u16buf = [0u8; 2];
    reader.read_exact(&mut u16buf)?;
    let version = u16::from_le_bytes(u16buf);
    if version != WIRE_VERSION {
        return Err(WorldStoreError::InvalidWire(format!("unsupported version {version}")));
    }
    let mut kind_flags = [0u8; 2];
    reader.read_exact(&mut kind_flags)?;
    let kind = WorldRecordKind::from_u8(kind_flags[0])?;
    let flags = kind_flags[1];
    let mut u32buf = [0u8; 4];
    reader.read_exact(&mut u32buf)?;
    let id_len = u32::from_le_bytes(u32buf) as usize;
    reader.read_exact(&mut u32buf)?;
    let aux_len = u32::from_le_bytes(u32buf) as usize;
    reader.read_exact(&mut u32buf)?;
    let payload_len = u32::from_le_bytes(u32buf) as usize;
    if id_len > MAX_ID_BYTES || aux_len > MAX_AUX_BYTES || payload_len > MAX_PAYLOAD_BYTES {
        return Err(WorldStoreError::InvalidWire("wire field exceeds bounded size".into()));
    }
    let mut i64buf = [0u8; 8];
    reader.read_exact(&mut i64buf)?;
    let raw_iteration = i64::from_le_bytes(i64buf);
    let mut id = vec![0u8; id_len];
    let mut aux = vec![0u8; aux_len];
    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut id)?;
    reader.read_exact(&mut aux)?;
    reader.read_exact(&mut payload)?;
    Ok(Some(WireRecord {
        kind,
        id: String::from_utf8(id)?,
        iteration_index: if flags & FLAG_ITERATION != 0 { Some(raw_iteration) } else { None },
        aux1: if flags & FLAG_AUX1 != 0 { Some(String::from_utf8(aux)?) } else { None },
        payload,
    }))
}

pub struct CopyTarget {
    pub staging_table: &'static str,
    pub final_table: &'static str,
    pub merge_sql: &'static str,
}

const STAGE: &str = "slr_world_v2_stage_record";
const MERGE_SOURCE: &str = "INSERT INTO slr_world_v2_source_manifestation (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=1 ON CONFLICT (record_id) DO NOTHING";
const MERGE_PNF: &str = "INSERT INTO slr_world_v2_pnf_candidate (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=2 ON CONFLICT (record_id) DO NOTHING";
const MERGE_ATOM: &str = "INSERT INTO slr_world_v2_atom (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=3 ON CONFLICT (record_id) DO NOTHING";
const MERGE_GAP: &str = "INSERT INTO slr_world_v2_gap (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=4 AND iteration_index IS NOT NULL ON CONFLICT (record_id,iteration_index) DO NOTHING";
const MERGE_OBLIGATION: &str = "INSERT INTO slr_world_v2_obligation (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=5 AND iteration_index IS NOT NULL ON CONFLICT (record_id,iteration_index) DO NOTHING";
const MERGE_ROUTE: &str = "INSERT INTO slr_world_v2_route_action (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=6 AND iteration_index IS NOT NULL ON CONFLICT (record_id,iteration_index) DO NOTHING";
const MERGE_ITERATION: &str = "INSERT INTO slr_world_v2_iteration (record_id,iteration_index,aux1,payload) SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_stage_record WHERE kind=7 AND iteration_index IS NOT NULL ON CONFLICT (iteration_index) DO NOTHING";
const MERGES: [&str; 7] = [MERGE_SOURCE, MERGE_PNF, MERGE_ATOM, MERGE_GAP, MERGE_OBLIGATION, MERGE_ROUTE, MERGE_ITERATION];

pub fn copy_target_for_kind(kind: WorldRecordKind) -> CopyTarget {
    match kind {
        WorldRecordKind::SourceManifestation => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_source_manifestation", merge_sql: MERGE_SOURCE },
        WorldRecordKind::PnfCandidate => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_pnf_candidate", merge_sql: MERGE_PNF },
        WorldRecordKind::WorldAtom => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_atom", merge_sql: MERGE_ATOM },
        WorldRecordKind::Gap => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_gap", merge_sql: MERGE_GAP },
        WorldRecordKind::Obligation => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_obligation", merge_sql: MERGE_OBLIGATION },
        WorldRecordKind::RouteAction => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_route_action", merge_sql: MERGE_ROUTE },
        WorldRecordKind::Iteration => CopyTarget { staging_table: STAGE, final_table: "slr_world_v2_iteration", merge_sql: MERGE_ITERATION },
    }
}

pub fn world_schema_sql() -> &'static str {
    r#"
CREATE TABLE IF NOT EXISTS slr_world_v2_source_manifestation (record_id TEXT PRIMARY KEY, iteration_index BIGINT, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_pnf_candidate (record_id TEXT PRIMARY KEY, iteration_index BIGINT, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_atom (record_id TEXT PRIMARY KEY, iteration_index BIGINT, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
CREATE TABLE IF NOT EXISTS slr_world_v2_gap (record_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (record_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_obligation (record_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (record_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_route_action (record_id TEXT NOT NULL, iteration_index BIGINT NOT NULL, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now(), PRIMARY KEY (record_id,iteration_index));
CREATE TABLE IF NOT EXISTS slr_world_v2_iteration (record_id TEXT NOT NULL, iteration_index BIGINT PRIMARY KEY, aux1 TEXT, payload BYTEA NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT now());
"#
}

fn stage_sql() -> &'static str {
    "CREATE TEMP TABLE slr_world_v2_stage_record (kind SMALLINT NOT NULL,record_id TEXT NOT NULL,iteration_index BIGINT,aux1 TEXT,payload BYTEA NOT NULL) ON COMMIT DROP"
}

pub fn latest_iteration_sql() -> &'static str {
    "SELECT MAX(iteration_index) FROM slr_world_v2_iteration"
}

pub fn frontier_gap_sql() -> &'static str {
    "SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_gap WHERE iteration_index=$1"
}

pub fn frontier_obligation_sql() -> &'static str {
    "SELECT record_id,iteration_index,aux1,payload FROM slr_world_v2_obligation WHERE iteration_index=$1"
}

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
            let rows = self.client.query_raw(frontier_gap_sql(), [&iteration_index])?;
            for row in rows {
                let record = frontier_row(WorldRecordKind::Gap, row?)?;
                encode_record(writer, &record)?;
                writer.flush()?;
                count += 1;
            }
        }
        {
            let rows = self.client.query_raw(frontier_obligation_sql(), [&iteration_index])?;
            for row in rows {
                let record = frontier_row(WorldRecordKind::Obligation, row?)?;
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
