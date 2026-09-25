//! SCALE-1 database-native parser compilation state.
//!
//! Production parser state lives in Postgres, not in TSV/JSON handoff files.
//! JSON remains appropriate for nested parser configuration/artifact metadata;
//! normalized token/dependency coordinates remain queryable relational rows.
//! Full parser artifacts may be retained as JSON or content-addressed locators.
//!
//! Deterministic compilation identity:
//!
//!   H(source_revision, region, parser_family, parser_version, model, config)
//!
//! lets workers reuse exact prior outputs and recompute only invalidated layers.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    candidate_pnf::role_for, CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor,
    CandidatePnfProducer, DatabaseConfig, ExactSourceSpan,
};

pub const DB_NATIVE_PARSER_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS ingest;

CREATE TABLE IF NOT EXISTS ingest.parser_run (
    parser_run_ref TEXT PRIMARY KEY,
    source_revision_ref TEXT NOT NULL,
    parser_family TEXT NOT NULL,
    parser_version TEXT NOT NULL,
    model_ref TEXT NOT NULL,
    config_digest_ref TEXT NOT NULL,
    config_json TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('active', 'complete')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL,
    candidate_only BOOLEAN NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL,
    applicability_promoted BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL
);

CREATE INDEX IF NOT EXISTS parser_run_source_idx
ON ingest.parser_run(source_revision_ref);

CREATE TABLE IF NOT EXISTS ingest.parser_job (
    compilation_key TEXT PRIMARY KEY,
    parser_run_ref TEXT NOT NULL REFERENCES ingest.parser_run(parser_run_ref) ON DELETE CASCADE,
    source_revision_ref TEXT NOT NULL,
    region_ref TEXT NOT NULL,
    region_start_char BIGINT NOT NULL,
    region_end_char BIGINT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('queued', 'leased', 'succeeded', 'residual')),
    lease_owner TEXT NULL,
    lease_expires_at TIMESTAMPTZ NULL,
    attempt_count INTEGER NOT NULL DEFAULT 0,
    error_ref TEXT NULL,
    queued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ NULL,
    candidate_only BOOLEAN NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL,
    applicability_promoted BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL,
    UNIQUE (parser_run_ref, region_ref)
);

CREATE INDEX IF NOT EXISTS parser_job_claim_idx
ON ingest.parser_job(parser_run_ref, status, lease_expires_at, region_ref);

CREATE TABLE IF NOT EXISTS ingest.parser_token (
    compilation_key TEXT NOT NULL REFERENCES ingest.parser_job(compilation_key) ON DELETE CASCADE,
    token_ordinal INTEGER NOT NULL,
    start_char BIGINT NOT NULL,
    end_char BIGINT NOT NULL,
    surface TEXT NOT NULL,
    lemma TEXT NOT NULL,
    pos TEXT NOT NULL,
    morph_json TEXT NULL,
    head_ordinal INTEGER NULL,
    dependency_ref TEXT NOT NULL,
    PRIMARY KEY (compilation_key, token_ordinal)
);

CREATE INDEX IF NOT EXISTS parser_token_span_idx
ON ingest.parser_token(compilation_key, start_char, end_char);

CREATE TABLE IF NOT EXISTS ingest.parser_entity (
    compilation_key TEXT NOT NULL REFERENCES ingest.parser_job(compilation_key) ON DELETE CASCADE,
    entity_ordinal INTEGER NOT NULL,
    start_char BIGINT NOT NULL,
    end_char BIGINT NOT NULL,
    surface TEXT NOT NULL,
    label_ref TEXT NOT NULL,
    PRIMARY KEY (compilation_key, entity_ordinal)
);

CREATE INDEX IF NOT EXISTS parser_entity_span_idx
ON ingest.parser_entity(compilation_key, start_char, end_char, label_ref);

CREATE TABLE IF NOT EXISTS ingest.parser_artifact (
    compilation_key TEXT PRIMARY KEY REFERENCES ingest.parser_job(compilation_key) ON DELETE CASCADE,
    format_ref TEXT NOT NULL,
    content_digest_ref TEXT NOT NULL,
    artifact_json TEXT NULL,
    object_locator TEXT NULL,
    candidate_only BOOLEAN NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL,
    CHECK (artifact_json IS NOT NULL OR object_locator IS NOT NULL)
);
"#;

#[derive(Debug, Error)]
pub enum DbNativeParserError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("parser family/version/model/source revision must be non-empty")]
    EmptyCoordinate,
    #[error("parser configuration is not valid JSON: {0}")]
    InvalidConfigJson(#[from] serde_json::Error),
    #[error("region span must have start < end")]
    InvalidRegionSpan,
    #[error("token {ordinal} lies outside parser job region")]
    TokenOutsideRegion { ordinal: u32 },
    #[error("token stream is not monotonic")]
    NonMonotonicTokens,
    #[error("duplicate token ordinal {0}")]
    DuplicateTokenOrdinal(u32),
    #[error("worker no longer owns parser job lease {0}")]
    LeaseLost(String),
    #[error("parser job {0} does not exist")]
    MissingJob(String),
    #[error("parser job/source revision mismatch")]
    SourceRevisionMismatch,
    #[error("parser artifact digest does not match exact JSON bytes")]
    ArtifactDigestMismatch,
    #[error("persisted parser output violates candidate-only boundary")]
    PromotionBoundary,
    #[error("parser run cannot complete while queued or leased work remains")]
    RunStillActive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserRunReceipt {
    pub parser_run_ref: String,
    pub source_revision_ref: String,
    pub parser_family: String,
    pub parser_version: String,
    pub model_ref: String,
    pub config_digest_ref: String,
    pub config_json: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserRegionJobSpec {
    pub region_ref: String,
    pub start_char: u64,
    pub end_char: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedParserJob {
    pub compilation_key: String,
    pub parser_run_ref: String,
    pub source_revision_ref: String,
    pub region_ref: String,
    pub start_char: u64,
    pub end_char: u64,
    pub attempt_count: u32,
    pub lease_owner: String,
    pub parser_family: String,
    pub parser_version: String,
    pub model_ref: String,
    pub config_digest_ref: String,
    pub config_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserTokenRecord {
    pub token_ordinal: u32,
    pub start_char: u32,
    pub end_char: u32,
    pub surface: String,
    pub lemma: String,
    pub pos: String,
    pub morph_json: Option<String>,
    pub head_ordinal: Option<u32>,
    pub dependency_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserEntityRecord {
    pub entity_ordinal: u32,
    pub start_char: u32,
    pub end_char: u32,
    pub surface: String,
    pub label_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserArtifactRecord {
    pub format_ref: String,
    pub content_digest_ref: String,
    pub artifact_json: Option<String>,
    pub object_locator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedParserOutputReceipt {
    pub compilation_key: String,
    pub region_ref: String,
    pub token_count: usize,
    pub parser_artifact_persisted: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserRunState {
    pub parser_run_ref: String,
    pub queued: usize,
    pub leased: usize,
    pub succeeded: usize,
    pub residual: usize,
    pub unattempted_semantic_regions: usize,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PersistedParserToken {
    ordinal: u32,
    start_char: u32,
    end_char: u32,
    surface: String,
    lemma: String,
    pos: String,
    dependency_ref: String,
}

#[derive(Debug, Clone)]
pub struct DbNativeParserSnapshot {
    parser_run_ref: String,
    source_revision_ref: String,
    by_region: BTreeMap<String, Vec<PersistedParserToken>>,
    residual_by_region: BTreeMap<String, String>,
}

pub struct DbNativeParserWriter {
    client: Client,
}

impl DbNativeParserWriter {
    pub fn connect(config: &DatabaseConfig) -> Result<Self, DbNativeParserError> {
        let mut client = Client::connect(config.database_url(), NoTls)?;
        client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
        Ok(Self { client })
    }

    pub fn persist_success_with_entities(
        &mut self,
        job: &ClaimedParserJob,
        worker_ref: &str,
        tokens: &[ParserTokenRecord],
        entities: &[ParserEntityRecord],
        artifact: Option<&ParserArtifactRecord>,
    ) -> Result<PersistedParserOutputReceipt, DbNativeParserError> {
        persist_parser_success_with_entities_on_client(
            &mut self.client,
            job,
            worker_ref,
            tokens,
            entities,
            artifact,
        )
    }
}

fn require(value: &str) -> Result<(), DbNativeParserError> {
    if value.trim().is_empty() {
        Err(DbNativeParserError::EmptyCoordinate)
    } else {
        Ok(())
    }
}

fn sha256_ref(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn sha256_content_ref(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{:x}", hasher.finalize())
}

fn canonical_json_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => out.push_str(&value.to_string()),
        Value::String(value) => out.push_str(&serde_json::to_string(value).expect("string JSON")),
        Value::Array(values) => {
            out.push('[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                canonical_json_value(value, out);
            }
            out.push(']');
        }
        Value::Object(values) => {
            out.push('{');
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).expect("object key JSON"));
                out.push(':');
                canonical_json_value(&values[key], out);
            }
            out.push('}');
        }
    }
}

fn canonical_json_text(raw: &str) -> Result<String, DbNativeParserError> {
    let value: Value = serde_json::from_str(raw)?;
    let mut out = String::new();
    canonical_json_value(&value, &mut out);
    Ok(out)
}

pub fn parser_run_ref(
    source_revision_ref: &str,
    parser_family: &str,
    parser_version: &str,
    model_ref: &str,
    config_digest_ref: &str,
) -> String {
    let digest = sha256_ref(&[
        source_revision_ref,
        parser_family,
        parser_version,
        model_ref,
        config_digest_ref,
    ]);
    format!("parser-run:{digest}")
}

pub fn compilation_key(
    source_revision_ref: &str,
    region_ref: &str,
    parser_family: &str,
    parser_version: &str,
    model_ref: &str,
    config_digest_ref: &str,
) -> String {
    sha256_ref(&[
        source_revision_ref,
        region_ref,
        parser_family,
        parser_version,
        model_ref,
        config_digest_ref,
    ])
}

pub fn install_db_native_parser_schema(
    config: &DatabaseConfig,
) -> Result<(), DbNativeParserError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    Ok(())
}

pub fn start_parser_run(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    parser_family: &str,
    parser_version: &str,
    model_ref: &str,
    config_json: &str,
) -> Result<ParserRunReceipt, DbNativeParserError> {
    for value in [
        source_revision_ref,
        parser_family,
        parser_version,
        model_ref,
    ] {
        require(value)?;
    }

    let canonical_config = canonical_json_text(config_json)?;
    let config_digest_ref = sha256_ref(&[&canonical_config]);
    let run_ref = parser_run_ref(
        source_revision_ref,
        parser_family,
        parser_version,
        model_ref,
        &config_digest_ref,
    );

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    client.execute(
        "INSERT INTO ingest.parser_run (
            parser_run_ref, source_revision_ref, parser_family, parser_version,
            model_ref, config_digest_ref, config_json, status,
            candidate_only, creates_semantic_authority,
            applicability_promoted, claim_truth_promoted
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,'active',TRUE,FALSE,FALSE,FALSE)
         ON CONFLICT (parser_run_ref) DO NOTHING",
        &[
            &run_ref,
            &source_revision_ref,
            &parser_family,
            &parser_version,
            &model_ref,
            &config_digest_ref,
            &canonical_config,
        ],
    )?;

    Ok(ParserRunReceipt {
        parser_run_ref: run_ref,
        source_revision_ref: source_revision_ref.to_owned(),
        parser_family: parser_family.to_owned(),
        parser_version: parser_version.to_owned(),
        model_ref: model_ref.to_owned(),
        config_digest_ref,
        config_json: canonical_config,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn enqueue_parser_regions(
    config: &DatabaseConfig,
    run: &ParserRunReceipt,
    regions: &[ParserRegionJobSpec],
) -> Result<usize, DbNativeParserError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    let mut tx = client.transaction()?;
    let mut inserted = 0usize;

    for region in regions {
        require(&region.region_ref)?;
        if region.start_char >= region.end_char {
            return Err(DbNativeParserError::InvalidRegionSpan);
        }
        let key = compilation_key(
            &run.source_revision_ref,
            &region.region_ref,
            &run.parser_family,
            &run.parser_version,
            &run.model_ref,
            &run.config_digest_ref,
        );
        inserted += tx.execute(
            "INSERT INTO ingest.parser_job (
                compilation_key, parser_run_ref, source_revision_ref, region_ref,
                region_start_char, region_end_char, status,
                candidate_only, creates_semantic_authority,
                applicability_promoted, claim_truth_promoted
             ) VALUES ($1,$2,$3,$4,$5,$6,'queued',TRUE,FALSE,FALSE,FALSE)
             ON CONFLICT (compilation_key) DO NOTHING",
            &[
                &key,
                &run.parser_run_ref,
                &run.source_revision_ref,
                &region.region_ref,
                &(region.start_char as i64),
                &(region.end_char as i64),
            ],
        )? as usize;
    }

    tx.commit()?;
    Ok(inserted)
}

pub fn claim_parser_jobs(
    config: &DatabaseConfig,
    parser_run_ref: &str,
    worker_ref: &str,
    limit: usize,
) -> Result<Vec<ClaimedParserJob>, DbNativeParserError> {
    require(parser_run_ref)?;
    require(worker_ref)?;
    if limit == 0 {
        return Ok(vec![]);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    let rows = client.query(
        "WITH selected AS (
            SELECT compilation_key
            FROM ingest.parser_job
            WHERE parser_run_ref = $1
              AND (
                (status = 'queued'
                 AND (lease_expires_at IS NULL OR lease_expires_at < NOW()))
                OR (status = 'leased' AND lease_expires_at < NOW())
              )
            ORDER BY region_ref
            FOR UPDATE SKIP LOCKED
            LIMIT $3
         ),
         updated AS (
            UPDATE ingest.parser_job j
            SET status = 'leased',
                lease_owner = $2,
                lease_expires_at = NOW() + INTERVAL '15 minutes',
                attempt_count = attempt_count + 1,
                error_ref = NULL
            FROM selected
            WHERE j.compilation_key = selected.compilation_key
            RETURNING
              j.compilation_key, j.parser_run_ref, j.source_revision_ref,
              j.region_ref, j.region_start_char, j.region_end_char,
              j.attempt_count, j.lease_owner
         )
         SELECT
           u.compilation_key, u.parser_run_ref, u.source_revision_ref,
           u.region_ref, u.region_start_char, u.region_end_char,
           u.attempt_count, u.lease_owner,
           r.parser_family, r.parser_version, r.model_ref,
           r.config_digest_ref, r.config_json
         FROM updated u
         JOIN ingest.parser_run r ON r.parser_run_ref = u.parser_run_ref
         ORDER BY u.region_ref",
        &[&parser_run_ref, &worker_ref, &(limit as i64)],
    )?;

    rows.into_iter()
        .map(|row| {
            let start = row.get::<_, i64>(4);
            let end = row.get::<_, i64>(5);
            let attempts = row.get::<_, i32>(6);
            if start < 0 || end < 0 || attempts < 0 {
                return Err(DbNativeParserError::InvalidRegionSpan);
            }
            Ok(ClaimedParserJob {
                compilation_key: row.get(0),
                parser_run_ref: row.get(1),
                source_revision_ref: row.get(2),
                region_ref: row.get(3),
                start_char: start as u64,
                end_char: end as u64,
                attempt_count: attempts as u32,
                lease_owner: row.get(7),
                parser_family: row.get(8),
                parser_version: row.get(9),
                model_ref: row.get(10),
                config_digest_ref: row.get(11),
                config_json: row.get(12),
            })
        })
        .collect()
}


pub fn load_claimed_job_text(
    config: &DatabaseConfig,
    job: &ClaimedParserJob,
) -> Result<String, DbNativeParserError> {
    let (source, canonical_text) = crate::load_generic_text_source(
        config,
        &job.source_revision_ref,
    )
    .map_err(|_| DbNativeParserError::SourceRevisionMismatch)?;
    if source.source_revision_ref != job.source_revision_ref {
        return Err(DbNativeParserError::SourceRevisionMismatch);
    }
    let text_len = canonical_text.chars().count() as u64;
    if job.start_char >= job.end_char || job.end_char > text_len {
        return Err(DbNativeParserError::InvalidRegionSpan);
    }
    Ok(canonical_text
        .chars()
        .skip(job.start_char as usize)
        .take((job.end_char - job.start_char) as usize)
        .collect())
}

fn validate_tokens(
    job: &ClaimedParserJob,
    tokens: &[ParserTokenRecord],
) -> Result<(), DbNativeParserError> {
    let mut ordinals = BTreeSet::new();
    let mut previous_start = None;
    let mut previous_end = None;

    for token in tokens {
        if token.start_char >= token.end_char
            || u64::from(token.start_char) < job.start_char
            || u64::from(token.end_char) > job.end_char
        {
            return Err(DbNativeParserError::TokenOutsideRegion {
                ordinal: token.token_ordinal,
            });
        }
        if !ordinals.insert(token.token_ordinal) {
            return Err(DbNativeParserError::DuplicateTokenOrdinal(
                token.token_ordinal,
            ));
        }
        if previous_start.is_some_and(|value| token.start_char < value)
            || previous_end.is_some_and(|value| token.end_char < value)
        {
            return Err(DbNativeParserError::NonMonotonicTokens);
        }
        if let Some(morph_json) = token.morph_json.as_deref() {
            let _: Value = serde_json::from_str(morph_json)?;
        }
        previous_start = Some(token.start_char);
        previous_end = Some(token.end_char);
    }
    Ok(())
}

pub fn persist_parser_success(
    config: &DatabaseConfig,
    job: &ClaimedParserJob,
    worker_ref: &str,
    tokens: &[ParserTokenRecord],
    artifact: Option<&ParserArtifactRecord>,
) -> Result<PersistedParserOutputReceipt, DbNativeParserError> {
    persist_parser_success_with_entities(
        config,
        job,
        worker_ref,
        tokens,
        &[],
        artifact,
    )
}

pub fn persist_parser_success_with_entities(
    config: &DatabaseConfig,
    job: &ClaimedParserJob,
    worker_ref: &str,
    tokens: &[ParserTokenRecord],
    entities: &[ParserEntityRecord],
    artifact: Option<&ParserArtifactRecord>,
) -> Result<PersistedParserOutputReceipt, DbNativeParserError> {
    let mut writer = DbNativeParserWriter::connect(config)?;
    writer.persist_success_with_entities(job, worker_ref, tokens, entities, artifact)
}

fn persist_parser_success_with_entities_on_client(
    client: &mut Client,
    job: &ClaimedParserJob,
    worker_ref: &str,
    tokens: &[ParserTokenRecord],
    entities: &[ParserEntityRecord],
    artifact: Option<&ParserArtifactRecord>,
) -> Result<PersistedParserOutputReceipt, DbNativeParserError> {
    validate_tokens(job, tokens)?;
    let mut entity_ordinals = BTreeSet::new();
    for entity in entities {
        if entity.start_char >= entity.end_char
            || u64::from(entity.start_char) < job.start_char
            || u64::from(entity.end_char) > job.end_char
            || entity.surface.trim().is_empty()
            || entity.label_ref.trim().is_empty()
        {
            return Err(DbNativeParserError::TokenOutsideRegion {
                ordinal: entity.entity_ordinal,
            });
        }
        if !entity_ordinals.insert(entity.entity_ordinal) {
            return Err(DbNativeParserError::DuplicateTokenOrdinal(
                entity.entity_ordinal,
            ));
        }
    }
    if let Some(artifact) = artifact {
        require(&artifact.format_ref)?;
        require(&artifact.content_digest_ref)?;
        if let Some(json) = artifact.artifact_json.as_deref() {
            let _: Value = serde_json::from_str(json)?;
            if sha256_content_ref(json.as_bytes()) != artifact.content_digest_ref {
                return Err(DbNativeParserError::ArtifactDigestMismatch);
            }
        }
        if artifact.artifact_json.is_none() && artifact.object_locator.is_none() {
            return Err(DbNativeParserError::EmptyCoordinate);
        }
    }

    let mut tx = client.transaction()?;


    let owned = tx.query_opt(
        "SELECT source_revision_ref, region_ref, region_start_char, region_end_char
         FROM ingest.parser_job
         WHERE compilation_key = $1
           AND status = 'leased'
           AND lease_owner = $2
           AND lease_expires_at >= NOW()
         FOR UPDATE",
        &[&job.compilation_key, &worker_ref],
    )?;
    let Some(owned) = owned else {
        return Err(DbNativeParserError::LeaseLost(job.compilation_key.clone()));
    };
    let stored_revision: String = owned.get(0);
    let stored_region: String = owned.get(1);
    if stored_revision != job.source_revision_ref || stored_region != job.region_ref {
        return Err(DbNativeParserError::SourceRevisionMismatch);
    }

    tx.execute(
        "DELETE FROM ingest.parser_token WHERE compilation_key = $1",
        &[&job.compilation_key],
    )?;
    if !tokens.is_empty() {
        let token_ordinals = tokens
            .iter()
            .map(|token| token.token_ordinal as i32)
            .collect::<Vec<_>>();
        let starts = tokens
            .iter()
            .map(|token| token.start_char as i64)
            .collect::<Vec<_>>();
        let ends = tokens
            .iter()
            .map(|token| token.end_char as i64)
            .collect::<Vec<_>>();
        let surfaces = tokens
            .iter()
            .map(|token| token.surface.clone())
            .collect::<Vec<_>>();
        let lemmas = tokens
            .iter()
            .map(|token| token.lemma.clone())
            .collect::<Vec<_>>();
        let pos = tokens
            .iter()
            .map(|token| token.pos.clone())
            .collect::<Vec<_>>();
        let morph_json = tokens
            .iter()
            .map(|token| {
                token
                    .morph_json
                    .as_deref()
                    .map(canonical_json_text)
                    .transpose()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let head_ordinals = tokens
            .iter()
            .map(|token| token.head_ordinal.map(|ordinal| ordinal as i32))
            .collect::<Vec<_>>();
        let dependencies = tokens
            .iter()
            .map(|token| token.dependency_ref.clone())
            .collect::<Vec<_>>();

        tx.execute(
            r#"
            INSERT INTO ingest.parser_token (
              compilation_key, token_ordinal, start_char, end_char,
              surface, lemma, pos, morph_json, head_ordinal, dependency_ref
            )
            SELECT $1, token_ordinal, start_char, end_char,
                   surface, lemma, pos, morph_json, head_ordinal, dependency_ref
            FROM UNNEST(
              $2::INTEGER[], $3::BIGINT[], $4::BIGINT[],
              $5::TEXT[], $6::TEXT[], $7::TEXT[], $8::TEXT[],
              $9::INTEGER[], $10::TEXT[]
            ) AS u(
              token_ordinal, start_char, end_char,
              surface, lemma, pos, morph_json, head_ordinal, dependency_ref
            )
            "#,
            &[
                &job.compilation_key,
                &token_ordinals,
                &starts,
                &ends,
                &surfaces,
                &lemmas,
                &pos,
                &morph_json,
                &head_ordinals,
                &dependencies,
            ],
        )?;
    }

    tx.execute(
        "DELETE FROM ingest.parser_entity WHERE compilation_key = $1",
        &[&job.compilation_key],
    )?;
    if !entities.is_empty() {
        let entity_ordinals = entities
            .iter()
            .map(|entity| entity.entity_ordinal as i32)
            .collect::<Vec<_>>();
        let starts = entities
            .iter()
            .map(|entity| entity.start_char as i64)
            .collect::<Vec<_>>();
        let ends = entities
            .iter()
            .map(|entity| entity.end_char as i64)
            .collect::<Vec<_>>();
        let surfaces = entities
            .iter()
            .map(|entity| entity.surface.clone())
            .collect::<Vec<_>>();
        let labels = entities
            .iter()
            .map(|entity| entity.label_ref.clone())
            .collect::<Vec<_>>();

        tx.execute(
            r#"
            INSERT INTO ingest.parser_entity (
              compilation_key, entity_ordinal, start_char, end_char,
              surface, label_ref
            )
            SELECT $1, entity_ordinal, start_char, end_char, surface, label_ref
            FROM UNNEST(
              $2::INTEGER[], $3::BIGINT[], $4::BIGINT[], $5::TEXT[], $6::TEXT[]
            ) AS u(entity_ordinal, start_char, end_char, surface, label_ref)
            "#,
            &[
                &job.compilation_key,
                &entity_ordinals,
                &starts,
                &ends,
                &surfaces,
                &labels,
            ],
        )?;
    }

    tx.execute(
        "DELETE FROM ingest.parser_artifact WHERE compilation_key = $1",
        &[&job.compilation_key],
    )?;
    if let Some(artifact) = artifact {
        let artifact_json = artifact.artifact_json.clone();
        tx.execute(
            "INSERT INTO ingest.parser_artifact (
                compilation_key, format_ref, content_digest_ref,
                artifact_json, object_locator,
                candidate_only, creates_semantic_authority, claim_truth_promoted
             ) VALUES ($1,$2,$3,$4,$5,TRUE,FALSE,FALSE)",
            &[
                &job.compilation_key,
                &artifact.format_ref,
                &artifact.content_digest_ref,
                &artifact_json,
                &artifact.object_locator,
            ],
        )?;
    }

    let changed = tx.execute(
        "UPDATE ingest.parser_job
         SET status = 'succeeded',
             lease_owner = NULL,
             lease_expires_at = NULL,
             completed_at = NOW(),
             error_ref = NULL,
             candidate_only = TRUE,
             creates_semantic_authority = FALSE,
             applicability_promoted = FALSE,
             claim_truth_promoted = FALSE
         WHERE compilation_key = $1
           AND status = 'leased'
           AND lease_owner = $2",
        &[&job.compilation_key, &worker_ref],
    )?;
    if changed != 1 {
        return Err(DbNativeParserError::LeaseLost(job.compilation_key.clone()));
    }
    tx.commit()?;

    Ok(PersistedParserOutputReceipt {
        compilation_key: job.compilation_key.clone(),
        region_ref: job.region_ref.clone(),
        token_count: tokens.len(),
        parser_artifact_persisted: artifact.is_some(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
}

pub fn defer_parser_job_retry(
    config: &DatabaseConfig,
    job: &ClaimedParserJob,
    worker_ref: &str,
    error_ref: &str,
) -> Result<(), DbNativeParserError> {
    require(error_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    let changed = client.execute(
        "UPDATE ingest.parser_job
         SET status = 'queued',
             lease_owner = NULL,
             lease_expires_at = NOW() + INTERVAL '5 minutes',
             error_ref = $3,
             completed_at = NULL,
             candidate_only = TRUE,
             creates_semantic_authority = FALSE,
             applicability_promoted = FALSE,
             claim_truth_promoted = FALSE
         WHERE compilation_key = $1
           AND status = 'leased'
           AND lease_owner = $2",
        &[&job.compilation_key, &worker_ref, &error_ref],
    )?;
    if changed != 1 {
        return Err(DbNativeParserError::LeaseLost(job.compilation_key.clone()));
    }
    Ok(())
}

pub fn renew_parser_job_lease(
    config: &DatabaseConfig,
    job: &ClaimedParserJob,
    worker_ref: &str,
) -> Result<(), DbNativeParserError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    let changed = client.execute(
        "UPDATE ingest.parser_job
         SET lease_expires_at = NOW() + INTERVAL '15 minutes'
         WHERE compilation_key = $1
           AND status = 'leased'
           AND lease_owner = $2
           AND lease_expires_at >= NOW()",
        &[&job.compilation_key, &worker_ref],
    )?;
    if changed != 1 {
        return Err(DbNativeParserError::LeaseLost(job.compilation_key.clone()));
    }
    Ok(())
}

pub fn persist_parser_residual(
    config: &DatabaseConfig,
    job: &ClaimedParserJob,
    worker_ref: &str,
    error_ref: &str,
) -> Result<(), DbNativeParserError> {
    require(error_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    let changed = client.execute(
        "UPDATE ingest.parser_job
         SET status = 'residual',
             lease_owner = NULL,
             lease_expires_at = NULL,
             completed_at = NOW(),
             error_ref = $3,
             candidate_only = TRUE,
             creates_semantic_authority = FALSE,
             applicability_promoted = FALSE,
             claim_truth_promoted = FALSE
         WHERE compilation_key = $1
           AND status = 'leased'
           AND lease_owner = $2",
        &[&job.compilation_key, &worker_ref, &error_ref],
    )?;
    if changed != 1 {
        return Err(DbNativeParserError::LeaseLost(job.compilation_key.clone()));
    }
    Ok(())
}

pub fn parser_run_state(
    config: &DatabaseConfig,
    parser_run_ref: &str,
) -> Result<ParserRunState, DbNativeParserError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT status, COUNT(*)::BIGINT
         FROM ingest.parser_job
         WHERE parser_run_ref = $1
         GROUP BY status",
        &[&parser_run_ref],
    )?;
    let counts = rows
        .into_iter()
        .map(|row| (row.get::<_, String>(0), row.get::<_, i64>(1) as usize))
        .collect::<BTreeMap<_, _>>();
    let queued = *counts.get("queued").unwrap_or(&0);
    let leased = *counts.get("leased").unwrap_or(&0);
    let succeeded = *counts.get("succeeded").unwrap_or(&0);
    let residual = *counts.get("residual").unwrap_or(&0);
    Ok(ParserRunState {
        parser_run_ref: parser_run_ref.to_owned(),
        queued,
        leased,
        succeeded,
        residual,
        unattempted_semantic_regions: queued + leased,
        complete: queued == 0 && leased == 0,
    })
}

pub fn complete_parser_run(
    config: &DatabaseConfig,
    parser_run_ref: &str,
) -> Result<ParserRunState, DbNativeParserError> {
    let state = parser_run_state(config, parser_run_ref)?;
    if !state.complete {
        return Err(DbNativeParserError::RunStillActive);
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        "UPDATE ingest.parser_run
         SET status = 'complete', completed_at = NOW()
         WHERE parser_run_ref = $1",
        &[&parser_run_ref],
    )?;
    Ok(state)
}

impl DbNativeParserSnapshot {
    pub fn load(
        config: &DatabaseConfig,
        parser_run_ref: &str,
    ) -> Result<Self, DbNativeParserError> {
        let mut client = Client::connect(config.database_url(), NoTls)?;
        client.batch_execute(DB_NATIVE_PARSER_SCHEMA_SQL)?;
        let run = client
            .query_opt(
                "SELECT source_revision_ref, candidate_only,
                        creates_semantic_authority, applicability_promoted,
                        claim_truth_promoted
                 FROM ingest.parser_run
                 WHERE parser_run_ref = $1",
                &[&parser_run_ref],
            )?
            .ok_or_else(|| DbNativeParserError::MissingJob(parser_run_ref.to_owned()))?;
        let source_revision_ref: String = run.get(0);
        let candidate_only: bool = run.get(1);
        let authority: bool = run.get(2);
        let applicability: bool = run.get(3);
        let truth: bool = run.get(4);
        if !candidate_only || authority || applicability || truth {
            return Err(DbNativeParserError::PromotionBoundary);
        }

        let succeeded_jobs = client.query(
            "SELECT region_ref, compilation_key
             FROM ingest.parser_job
             WHERE parser_run_ref = $1
               AND status = 'succeeded'
               AND candidate_only = TRUE
               AND creates_semantic_authority = FALSE
               AND applicability_promoted = FALSE
               AND claim_truth_promoted = FALSE
             ORDER BY region_ref",
            &[&parser_run_ref],
        )?;

        let mut by_region = BTreeMap::new();
        for job in succeeded_jobs {
            let region_ref: String = job.get(0);
            let key: String = job.get(1);
            let token_rows = client.query(
                "SELECT token_ordinal, start_char, end_char, surface, lemma,
                        pos, dependency_ref
                 FROM ingest.parser_token
                 WHERE compilation_key = $1
                 ORDER BY token_ordinal",
                &[&key],
            )?;
            let mut tokens = Vec::with_capacity(token_rows.len());
            for row in token_rows {
                let ordinal = row.get::<_, i32>(0);
                let start = row.get::<_, i64>(1);
                let end = row.get::<_, i64>(2);
                if ordinal < 0 || start < 0 || end < 0 {
                    return Err(DbNativeParserError::InvalidRegionSpan);
                }
                tokens.push(PersistedParserToken {
                    ordinal: ordinal as u32,
                    start_char: start as u32,
                    end_char: end as u32,
                    surface: row.get(3),
                    lemma: row.get(4),
                    pos: row.get(5),
                    dependency_ref: row.get(6),
                });
            }
            by_region.insert(region_ref, tokens);
        }

        let residual_rows = client.query(
            "SELECT region_ref, error_ref
             FROM ingest.parser_job
             WHERE parser_run_ref = $1
               AND status = 'residual'
               AND candidate_only = TRUE
               AND creates_semantic_authority = FALSE
               AND applicability_promoted = FALSE
               AND claim_truth_promoted = FALSE
             ORDER BY region_ref",
            &[&parser_run_ref],
        )?;
        let residual_by_region = residual_rows
            .into_iter()
            .map(|row| {
                let region_ref: String = row.get(0);
                let error_ref: Option<String> = row.get(1);
                (
                    region_ref,
                    error_ref.unwrap_or_else(|| "parser-residual:unspecified".into()),
                )
            })
            .collect::<BTreeMap<_, _>>();

        Ok(Self {
            parser_run_ref: parser_run_ref.to_owned(),
            source_revision_ref,
            by_region,
            residual_by_region,
        })
    }

    #[must_use]
    pub fn parser_run_ref(&self) -> &str {
        &self.parser_run_ref
    }

    #[must_use]
    pub fn source_revision_ref(&self) -> &str {
        &self.source_revision_ref
    }

    #[must_use]
    pub fn compiled_region_count(&self) -> usize {
        self.by_region.len()
    }

    #[must_use]
    pub fn residual_region_count(&self) -> usize {
        self.residual_by_region.len()
    }
}

impl CandidatePnfProducer for DbNativeParserSnapshot {
    fn produce(&self, source: &ExactSourceSpan) -> Result<CandidatePnfBatch, CandidatePnfError> {
        if source.start_char >= source.end_char {
            return Err(CandidatePnfError::InvalidExactSpan);
        }
        if let Some(error_ref) = self.residual_by_region.get(&source.span_ref) {
            return Err(CandidatePnfError::PersistedParserResidual {
                span_ref: source.span_ref.clone(),
                error_ref: error_ref.clone(),
            });
        }
        let tokens = self
            .by_region
            .get(&source.span_ref)
            .ok_or_else(|| CandidatePnfError::MissingPersistedParserRegion(source.span_ref.clone()))?;

        let candidates = tokens
            .iter()
            .filter(|token| token.start_char < source.end_char && token.end_char > source.start_char)
            .map(|token| CandidatePnfFactor {
                candidate_ref: format!(
                    "candidate-pnf-db:{}:{}:{}:{}:{}",
                    self.parser_run_ref,
                    source.span_ref,
                    token.ordinal,
                    token.start_char,
                    token.end_char
                ),
                role: role_for(&token.pos, &token.dependency_ref),
                source_start_char: token.start_char,
                source_end_char: token.end_char,
                surface: token.surface.clone(),
                lemma: token.lemma.clone(),
                dependency_ref: token.dependency_ref.clone(),
                candidate_only: true,
            })
            .collect();

        Ok(CandidatePnfBatch {
            exact_span_ref: source.span_ref.clone(),
            candidates,
            proposition_support_paid: false,
            applicability_paid: false,
            claim_truth_paid: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_makes_config_digest_order_independent() {
        let a = canonical_json_text(r#"{"model":"x","flags":{"b":2,"a":1}}"#).unwrap();
        let b = canonical_json_text(r#"{"flags":{"a":1,"b":2},"model":"x"}"#).unwrap();
        assert_eq!(a, b);
        assert_eq!(sha256_ref(&[&a]), sha256_ref(&[&b]));
    }

    #[test]
    fn compilation_key_changes_only_with_declared_compilation_inputs() {
        let a = compilation_key("rev:1", "region:1", "spacy", "3.8", "en_core", "cfg:1");
        let same = compilation_key("rev:1", "region:1", "spacy", "3.8", "en_core", "cfg:1");
        let changed_region =
            compilation_key("rev:1", "region:2", "spacy", "3.8", "en_core", "cfg:1");
        let changed_parser =
            compilation_key("rev:1", "region:1", "spacy", "3.9", "en_core", "cfg:1");
        assert_eq!(a, same);
        assert_ne!(a, changed_region);
        assert_ne!(a, changed_parser);
    }

    #[test]
    fn token_validation_requires_region_local_monotonic_coordinates() {
        let job = ClaimedParserJob {
            compilation_key: "key".into(),
            parser_run_ref: "run".into(),
            source_revision_ref: "revision".into(),
            region_ref: "region".into(),
            start_char: 10,
            end_char: 30,
            attempt_count: 1,
            lease_owner: "worker".into(),
            parser_family: "spacy".into(),
            parser_version: "fixture".into(),
            model_ref: "fixture-model".into(),
            config_digest_ref: "sha256:fixture".into(),
            config_json: "{}".into(),
        };
        let valid = vec![
            ParserTokenRecord {
                token_ordinal: 0,
                start_char: 10,
                end_char: 15,
                surface: "Alice".into(),
                lemma: "Alice".into(),
                pos: "PROPN".into(),
                morph_json: Some(r#"{"Number":"Sing"}"#.into()),
                head_ordinal: Some(1),
                dependency_ref: "nsubj".into(),
            },
            ParserTokenRecord {
                token_ordinal: 1,
                start_char: 16,
                end_char: 22,
                surface: "called".into(),
                lemma: "call".into(),
                pos: "VERB".into(),
                morph_json: None,
                head_ordinal: Some(1),
                dependency_ref: "ROOT".into(),
            },
        ];
        assert!(validate_tokens(&job, &valid).is_ok());

        let mut outside = valid.clone();
        outside[0].start_char = 0;
        assert!(matches!(
            validate_tokens(&job, &outside),
            Err(DbNativeParserError::TokenOutsideRegion { .. })
        ));
    }
}
