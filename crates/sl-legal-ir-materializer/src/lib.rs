use std::env;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const SLRI_MAGIC: [u8; 4] = *b"SLRI";
pub const SLRI_VERSION: u16 = 1;
const MAX_FIELD_BYTES: usize = 16 << 20;
const MAX_BODY_BYTES: usize = 64 << 20;
const MAX_LIST_ITEMS: usize = 1 << 20;

#[derive(Debug, Error)]
pub enum LegalIrError {
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
    #[error("invalid SLRI record: {0}")]
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

pub fn load_database_config(explicit_env_file: Option<&Path>) -> Result<DatabaseConfig, LegalIrError> {
    if let Ok(value) = env::var("DATABASE_URL") {
        if !value.trim().is_empty() {
            return Ok(DatabaseConfig { database_url: value });
        }
    }
    if let Some(path) = explicit_env_file {
        if !path.exists() {
            return Err(LegalIrError::MissingEnvFile(path.to_path_buf()));
        }
        dotenvy::from_path(path).map_err(|e| LegalIrError::EnvFile(e.to_string()))?;
        let value = env::var("DATABASE_URL").map_err(|_| LegalIrError::MissingDatabaseUrl)?;
        if value.trim().is_empty() {
            return Err(LegalIrError::MissingDatabaseUrl);
        }
        return Ok(DatabaseConfig { database_url: value });
    }
    Err(LegalIrError::MissingDatabaseUrl)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticBuild {
    pub build_ref: String,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_ref: String,
    pub parser_build_ref: String,
    pub pnf_build_ref: String,
    pub refined_pnf_graph_ref: String,
    pub legal_ir_projection_ref: String,
    pub build_state_ref: String,
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projection {
    pub projection_ref: String,
    pub build_ref: String,
    pub pnf_build_ref: String,
    pub projection_contract_ref: String,
    pub omitted_factor_refs: Vec<String>,
    pub projection_residuals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub observation_ref: String,
    pub projection_ref: String,
    pub pnf_factor_ref: String,
    pub pnf_revision_ref: String,
    pub structural_signature_ref: String,
    pub predicate_ref: String,
    pub observation_body: Vec<u8>,
    pub provenance_refs: Vec<String>,
    pub residual_refs: Vec<String>,
    pub projection_state_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphRevision {
    pub revision_ref: String,
    pub subject_ref: String,
    pub prior_revision_refs: Vec<String>,
    pub source_span_refs: Vec<String>,
    pub legal_system_refs: Vec<String>,
    pub jurisdiction_refs: Vec<String>,
    pub temporal_refs: Vec<String>,
    pub author_ref: String,
    pub institution_ref: Option<String>,
    pub build_ref: String,
    pub revision_state_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalIrRecord {
    SemanticBuild(SemanticBuild),
    Projection(Projection),
    Observation(Observation),
    GraphRevision(GraphRevision),
}

impl LegalIrRecord {
    fn tag(&self) -> u8 {
        match self {
            Self::SemanticBuild(_) => 1,
            Self::Projection(_) => 2,
            Self::Observation(_) => 3,
            Self::GraphRevision(_) => 4,
        }
    }

    fn identity(&self) -> &str {
        match self {
            Self::SemanticBuild(v) => &v.build_ref,
            Self::Projection(v) => &v.projection_ref,
            Self::Observation(v) => &v.observation_ref,
            Self::GraphRevision(v) => &v.revision_ref,
        }
    }
}

fn put_u32(out: &mut Vec<u8>, value: usize) -> Result<(), LegalIrError> {
    let value = u32::try_from(value)
        .map_err(|_| LegalIrError::InvalidRecord("field length exceeds u32".into()))?;
    out.extend_from_slice(&value.to_le_bytes());
    Ok(())
}

fn put_string(out: &mut Vec<u8>, value: &str) -> Result<(), LegalIrError> {
    if value.is_empty() {
        return Err(LegalIrError::InvalidRecord("required string is empty".into()));
    }
    if value.len() > MAX_FIELD_BYTES {
        return Err(LegalIrError::InvalidRecord("string field too large".into()));
    }
    put_u32(out, value.len())?;
    out.extend_from_slice(value.as_bytes());
    Ok(())
}

fn put_string_list(out: &mut Vec<u8>, values: &[String]) -> Result<(), LegalIrError> {
    if values.len() > MAX_LIST_ITEMS {
        return Err(LegalIrError::InvalidRecord("string list too large".into()));
    }
    put_u32(out, values.len())?;
    for value in values { put_string(out, value)?; }
    Ok(())
}

fn put_optional_string(out: &mut Vec<u8>, value: &Option<String>) -> Result<(), LegalIrError> {
    match value {
        Some(value) => { out.push(1); put_string(out, value)?; }
        None => out.push(0),
    }
    Ok(())
}

fn put_bytes(out: &mut Vec<u8>, value: &[u8]) -> Result<(), LegalIrError> {
    if value.len() > MAX_BODY_BYTES {
        return Err(LegalIrError::InvalidRecord("binary body too large".into()));
    }
    put_u32(out, value.len())?;
    out.extend_from_slice(value);
    Ok(())
}

fn encode_body(record: &LegalIrRecord) -> Result<Vec<u8>, LegalIrError> {
    let mut out = Vec::new();
    match record {
        LegalIrRecord::SemanticBuild(v) => {
            put_string(&mut out, &v.build_ref)?;
            put_string(&mut out, &v.document_ref)?;
            put_string(&mut out, &v.source_revision_ref)?;
            put_string(&mut out, &v.canonical_text_ref)?;
            put_string(&mut out, &v.parser_build_ref)?;
            put_string(&mut out, &v.pnf_build_ref)?;
            put_string(&mut out, &v.refined_pnf_graph_ref)?;
            put_string(&mut out, &v.legal_ir_projection_ref)?;
            put_string(&mut out, &v.build_state_ref)?;
            put_string_list(&mut out, &v.provenance_refs)?;
        }
        LegalIrRecord::Projection(v) => {
            put_string(&mut out, &v.projection_ref)?;
            put_string(&mut out, &v.build_ref)?;
            put_string(&mut out, &v.pnf_build_ref)?;
            put_string(&mut out, &v.projection_contract_ref)?;
            put_string_list(&mut out, &v.omitted_factor_refs)?;
            put_string_list(&mut out, &v.projection_residuals)?;
        }
        LegalIrRecord::Observation(v) => {
            if !v.observation_body.starts_with(b"OBS1") {
                return Err(LegalIrError::InvalidRecord("observation body must begin OBS1".into()));
            }
            put_string(&mut out, &v.observation_ref)?;
            put_string(&mut out, &v.projection_ref)?;
            put_string(&mut out, &v.pnf_factor_ref)?;
            put_string(&mut out, &v.pnf_revision_ref)?;
            put_string(&mut out, &v.structural_signature_ref)?;
            put_string(&mut out, &v.predicate_ref)?;
            put_bytes(&mut out, &v.observation_body)?;
            put_string_list(&mut out, &v.provenance_refs)?;
            put_string_list(&mut out, &v.residual_refs)?;
            put_string(&mut out, &v.projection_state_ref)?;
        }
        LegalIrRecord::GraphRevision(v) => {
            put_string(&mut out, &v.revision_ref)?;
            put_string(&mut out, &v.subject_ref)?;
            put_string_list(&mut out, &v.prior_revision_refs)?;
            put_string_list(&mut out, &v.source_span_refs)?;
            put_string_list(&mut out, &v.legal_system_refs)?;
            put_string_list(&mut out, &v.jurisdiction_refs)?;
            put_string_list(&mut out, &v.temporal_refs)?;
            put_string(&mut out, &v.author_ref)?;
            put_optional_string(&mut out, &v.institution_ref)?;
            put_string(&mut out, &v.build_ref)?;
            put_string(&mut out, &v.revision_state_ref)?;
        }
    }
    Ok(out)
}

pub fn encode_record<W: Write>(writer: &mut W, record: &LegalIrRecord) -> Result<(), LegalIrError> {
    let body = encode_body(record)?;
    writer.write_all(&SLRI_MAGIC)?;
    writer.write_all(&SLRI_VERSION.to_le_bytes())?;
    writer.write_all(&[record.tag(), 0])?;
    writer.write_all(&(body.len() as u32).to_le_bytes())?;
    writer.write_all(body.as_slice())?;
    Ok(())
}

fn read_exact_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<bool, LegalIrError> {
    let mut offset = 0;
    while offset < buf.len() {
        match reader.read(&mut buf[offset..]) {
            Ok(0) if offset == 0 => return Ok(false),
            Ok(0) => return Err(LegalIrError::InvalidRecord("truncated frame".into())),
            Ok(n) => offset += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

struct BodyReader<'a> { body: &'a [u8], offset: usize }
impl<'a> BodyReader<'a> {
    fn new(body: &'a [u8]) -> Self { Self { body, offset: 0 } }
    fn take(&mut self, n: usize) -> Result<&'a [u8], LegalIrError> {
        let end = self.offset.checked_add(n)
            .ok_or_else(|| LegalIrError::InvalidRecord("field length overflow".into()))?;
        if end > self.body.len() {
            return Err(LegalIrError::InvalidRecord("truncated body field".into()));
        }
        let slice = &self.body[self.offset..end];
        self.offset = end;
        Ok(slice)
    }
    fn u32(&mut self) -> Result<usize, LegalIrError> {
        let bytes: [u8; 4] = self.take(4)?.try_into().expect("fixed length");
        Ok(u32::from_le_bytes(bytes) as usize)
    }
    fn string(&mut self) -> Result<String, LegalIrError> {
        let len = self.u32()?;
        if len == 0 || len > MAX_FIELD_BYTES {
            return Err(LegalIrError::InvalidRecord("invalid string length".into()));
        }
        String::from_utf8(self.take(len)?.to_vec())
            .map_err(|_| LegalIrError::InvalidRecord("string is not UTF-8".into()))
    }
    fn string_list(&mut self) -> Result<Vec<String>, LegalIrError> {
        let count = self.u32()?;
        if count > MAX_LIST_ITEMS {
            return Err(LegalIrError::InvalidRecord("string list too large".into()));
        }
        (0..count).map(|_| self.string()).collect()
    }
    fn optional_string(&mut self) -> Result<Option<String>, LegalIrError> {
        match self.take(1)?[0] {
            0 => Ok(None),
            1 => Ok(Some(self.string()?)),
            _ => Err(LegalIrError::InvalidRecord("invalid optional-string tag".into())),
        }
    }
    fn bytes(&mut self) -> Result<Vec<u8>, LegalIrError> {
        let len = self.u32()?;
        if len > MAX_BODY_BYTES {
            return Err(LegalIrError::InvalidRecord("binary body too large".into()));
        }
        Ok(self.take(len)?.to_vec())
    }
    fn finish(self) -> Result<(), LegalIrError> {
        if self.offset == self.body.len() { Ok(()) }
        else { Err(LegalIrError::InvalidRecord("trailing body bytes".into())) }
    }
}

fn decode_body(tag: u8, body: &[u8]) -> Result<LegalIrRecord, LegalIrError> {
    let mut r = BodyReader::new(body);
    let record = match tag {
        1 => LegalIrRecord::SemanticBuild(SemanticBuild {
            build_ref: r.string()?, document_ref: r.string()?, source_revision_ref: r.string()?,
            canonical_text_ref: r.string()?, parser_build_ref: r.string()?, pnf_build_ref: r.string()?,
            refined_pnf_graph_ref: r.string()?, legal_ir_projection_ref: r.string()?,
            build_state_ref: r.string()?, provenance_refs: r.string_list()?,
        }),
        2 => LegalIrRecord::Projection(Projection {
            projection_ref: r.string()?, build_ref: r.string()?, pnf_build_ref: r.string()?,
            projection_contract_ref: r.string()?, omitted_factor_refs: r.string_list()?,
            projection_residuals: r.string_list()?,
        }),
        3 => {
            let value = Observation {
                observation_ref: r.string()?, projection_ref: r.string()?, pnf_factor_ref: r.string()?,
                pnf_revision_ref: r.string()?, structural_signature_ref: r.string()?, predicate_ref: r.string()?,
                observation_body: r.bytes()?, provenance_refs: r.string_list()?, residual_refs: r.string_list()?,
                projection_state_ref: r.string()?,
            };
            if !value.observation_body.starts_with(b"OBS1") {
                return Err(LegalIrError::InvalidRecord("observation body must begin OBS1".into()));
            }
            LegalIrRecord::Observation(value)
        }
        4 => LegalIrRecord::GraphRevision(GraphRevision {
            revision_ref: r.string()?, subject_ref: r.string()?, prior_revision_refs: r.string_list()?,
            source_span_refs: r.string_list()?, legal_system_refs: r.string_list()?, jurisdiction_refs: r.string_list()?,
            temporal_refs: r.string_list()?, author_ref: r.string()?, institution_ref: r.optional_string()?,
            build_ref: r.string()?, revision_state_ref: r.string()?,
        }),
        _ => return Err(LegalIrError::InvalidRecord(format!("unknown record tag {tag}"))),
    };
    r.finish()?;
    Ok(record)
}

pub fn decode_record<R: Read>(reader: &mut R) -> Result<Option<LegalIrRecord>, LegalIrError> {
    let mut magic = [0u8; 4];
    if !read_exact_or_eof(reader, &mut magic)? { return Ok(None); }
    if magic != SLRI_MAGIC { return Err(LegalIrError::InvalidRecord("bad magic".into())); }
    let mut v = [0u8; 2]; reader.read_exact(&mut v)?;
    if u16::from_le_bytes(v) != SLRI_VERSION {
        return Err(LegalIrError::InvalidRecord("unsupported version".into()));
    }
    let mut tag_flags = [0u8; 2]; reader.read_exact(&mut tag_flags)?;
    if tag_flags[1] != 0 { return Err(LegalIrError::InvalidRecord("unsupported flags".into())); }
    let mut b4 = [0u8; 4]; reader.read_exact(&mut b4)?;
    let body_len = u32::from_le_bytes(b4) as usize;
    if body_len > MAX_BODY_BYTES { return Err(LegalIrError::InvalidRecord("frame too large".into())); }
    let mut body = vec![0u8; body_len]; reader.read_exact(&mut body)?;
    Ok(Some(decode_body(tag_flags[0], &body)?))
}

pub fn legal_ir_schema_sql() -> &'static str { r#"
CREATE SCHEMA IF NOT EXISTS legal_ir;
CREATE TABLE IF NOT EXISTS legal_ir.semantic_build_v2 (
    build_ref TEXT PRIMARY KEY,
    document_ref TEXT NOT NULL,
    source_revision_ref TEXT NOT NULL,
    canonical_text_ref TEXT NOT NULL,
    parser_build_ref TEXT NOT NULL,
    pnf_build_ref TEXT NOT NULL,
    refined_pnf_graph_ref TEXT NOT NULL,
    legal_ir_projection_ref TEXT NOT NULL,
    build_state_ref TEXT NOT NULL,
    provenance_refs TEXT[] NOT NULL DEFAULT '{}',
    payload BYTEA NOT NULL,
    record_sha256 BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS legal_ir.projection_v2 (
    projection_ref TEXT PRIMARY KEY,
    build_ref TEXT NOT NULL REFERENCES legal_ir.semantic_build_v2(build_ref),
    pnf_build_ref TEXT NOT NULL,
    projection_contract_ref TEXT NOT NULL,
    omitted_factor_refs TEXT[] NOT NULL DEFAULT '{}',
    projection_residuals TEXT[] NOT NULL DEFAULT '{}',
    payload BYTEA NOT NULL,
    record_sha256 BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS legal_ir.observation_v2 (
    observation_ref TEXT PRIMARY KEY,
    projection_ref TEXT NOT NULL REFERENCES legal_ir.projection_v2(projection_ref),
    pnf_factor_ref TEXT NOT NULL,
    pnf_revision_ref TEXT NOT NULL,
    structural_signature_ref TEXT NOT NULL,
    predicate_ref TEXT NOT NULL,
    observation_body BYTEA NOT NULL,
    provenance_refs TEXT[] NOT NULL DEFAULT '{}',
    residual_refs TEXT[] NOT NULL DEFAULT '{}',
    projection_state_ref TEXT NOT NULL DEFAULT 'candidate',
    payload BYTEA NOT NULL,
    record_sha256 BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS legal_ir.graph_revision_v2 (
    revision_ref TEXT PRIMARY KEY,
    subject_ref TEXT NOT NULL,
    prior_revision_refs TEXT[] NOT NULL DEFAULT '{}',
    source_span_refs TEXT[] NOT NULL DEFAULT '{}',
    legal_system_refs TEXT[] NOT NULL DEFAULT '{}',
    jurisdiction_refs TEXT[] NOT NULL DEFAULT '{}',
    temporal_refs TEXT[] NOT NULL DEFAULT '{}',
    author_ref TEXT NOT NULL,
    institution_ref TEXT,
    build_ref TEXT NOT NULL REFERENCES legal_ir.semantic_build_v2(build_ref),
    revision_state_ref TEXT NOT NULL DEFAULT 'candidate',
    payload BYTEA NOT NULL,
    record_sha256 BYTEA NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS legal_ir_graph_revision_v2_subject_idx ON legal_ir.graph_revision_v2(subject_ref, created_at);
CREATE INDEX IF NOT EXISTS legal_ir_semantic_build_v2_source_idx ON legal_ir.semantic_build_v2(source_revision_ref, document_ref);
"# }

pub fn exact_source_weld_sql() -> &'static str { r#"
SELECT EXISTS (
    SELECT 1
    FROM legal_ir.graph_revision_v2 g
    JOIN legal_ir.semantic_build_v2 b ON g.build_ref = b.build_ref
    WHERE g.subject_ref = $1
      AND b.source_revision_ref = $2
      AND $3 = ANY(g.source_span_refs)
) AS exact_source_paid
"# }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceWeldState {
    pub exact_source_paid: bool,
    pub proposition_truth_paid: bool,
    pub applicability_paid: bool,
}
impl SourceWeldState {
    pub fn from_exact_source_match(exact_source_paid: bool) -> Self {
        Self { exact_source_paid, proposition_truth_paid: false, applicability_paid: false }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IngestReceipt {
    pub records_seen: u64,
    pub rows_inserted: u64,
    pub binary_wire: bool,
    pub append_only: bool,
    pub semantic_promotion: bool,
}

pub struct LegalIrMaterializer { client: Client }
impl LegalIrMaterializer {
    pub fn connect(config: &DatabaseConfig) -> Result<Self, LegalIrError> {
        let mut client = Client::connect(config.database_url(), NoTls)?;
        client.batch_execute(legal_ir_schema_sql())?;
        Ok(Self { client })
    }

    pub fn ingest<R: Read>(&mut self, reader: &mut R) -> Result<IngestReceipt, LegalIrError> {
        let mut records_seen = 0u64;
        let mut rows_inserted = 0u64;
        while let Some(record) = decode_record(reader)? {
            records_seen += 1;
            let mut payload = Vec::new();
            encode_record(&mut payload, &record)?;
            let digest = Sha256::digest(&payload).to_vec();
            let inserted = match &record {
                LegalIrRecord::SemanticBuild(v) => self.client.execute(
                    "INSERT INTO legal_ir.semantic_build_v2 (build_ref,document_ref,source_revision_ref,canonical_text_ref,parser_build_ref,pnf_build_ref,refined_pnf_graph_ref,legal_ir_projection_ref,build_state_ref,provenance_refs,payload,record_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT (build_ref) DO NOTHING",
                    &[&v.build_ref,&v.document_ref,&v.source_revision_ref,&v.canonical_text_ref,&v.parser_build_ref,&v.pnf_build_ref,&v.refined_pnf_graph_ref,&v.legal_ir_projection_ref,&v.build_state_ref,&v.provenance_refs,&payload,&digest],
                )?,
                LegalIrRecord::Projection(v) => self.client.execute(
                    "INSERT INTO legal_ir.projection_v2 (projection_ref,build_ref,pnf_build_ref,projection_contract_ref,omitted_factor_refs,projection_residuals,payload,record_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (projection_ref) DO NOTHING",
                    &[&v.projection_ref,&v.build_ref,&v.pnf_build_ref,&v.projection_contract_ref,&v.omitted_factor_refs,&v.projection_residuals,&payload,&digest],
                )?,
                LegalIrRecord::Observation(v) => self.client.execute(
                    "INSERT INTO legal_ir.observation_v2 (observation_ref,projection_ref,pnf_factor_ref,pnf_revision_ref,structural_signature_ref,predicate_ref,observation_body,provenance_refs,residual_refs,projection_state_ref,payload,record_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) ON CONFLICT (observation_ref) DO NOTHING",
                    &[&v.observation_ref,&v.projection_ref,&v.pnf_factor_ref,&v.pnf_revision_ref,&v.structural_signature_ref,&v.predicate_ref,&v.observation_body,&v.provenance_refs,&v.residual_refs,&v.projection_state_ref,&payload,&digest],
                )?,
                LegalIrRecord::GraphRevision(v) => self.client.execute(
                    "INSERT INTO legal_ir.graph_revision_v2 (revision_ref,subject_ref,prior_revision_refs,source_span_refs,legal_system_refs,jurisdiction_refs,temporal_refs,author_ref,institution_ref,build_ref,revision_state_ref,payload,record_sha256) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) ON CONFLICT (revision_ref) DO NOTHING",
                    &[&v.revision_ref,&v.subject_ref,&v.prior_revision_refs,&v.source_span_refs,&v.legal_system_refs,&v.jurisdiction_refs,&v.temporal_refs,&v.author_ref,&v.institution_ref,&v.build_ref,&v.revision_state_ref,&payload,&digest],
                )?,
            };
            rows_inserted += inserted;
        }
        Ok(IngestReceipt { records_seen, rows_inserted, binary_wire: true, append_only: true, semantic_promotion: false })
    }

    pub fn exact_source_weld(&mut self, subject_ref: &str, source_revision_ref: &str, span_ref: &str) -> Result<SourceWeldState, LegalIrError> {
        let row = self.client.query_one(exact_source_weld_sql(), &[&subject_ref, &source_revision_ref, &span_ref])?;
        let exact: bool = row.get(0);
        Ok(SourceWeldState::from_exact_source_match(exact))
    }
}

pub fn record_identity(record: &LegalIrRecord) -> &str { record.identity() }
