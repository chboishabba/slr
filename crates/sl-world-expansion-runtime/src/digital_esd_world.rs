use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::Path;

use postgres::{Client, NoTls};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use sensiblaw_world_store::{
    active_frontier_gap_sql, active_frontier_obligation_sql, latest_iteration_sql,
    DatabaseConfig, WireRecord, WorldRecordKind, WorldStore, WorldStoreError,
    encode_record,
};

const WORLD_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS digital_esd;

CREATE TABLE IF NOT EXISTS digital_esd.world_revision (
    world_revision_ref TEXT PRIMARY KEY,
    corpus_ref TEXT NOT NULL,
    compiler_ref TEXT NOT NULL,
    processing_ledger_sha256 TEXT NOT NULL,
    metadata_source_count BIGINT NOT NULL,
    canonical_source_revision_count BIGINT NOT NULL,
    exact_region_count BIGINT NOT NULL,
    statement_count BIGINT NOT NULL,
    candidate_pnf_batch_count BIGINT NOT NULL,
    candidate_observation_count BIGINT NOT NULL,
    reviewed_world_record_count BIGINT NOT NULL,
    active_gap_count BIGINT NOT NULL,
    active_obligation_count BIGINT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS digital_esd_world_revision_created_idx
ON digital_esd.world_revision(created_at DESC);
"#;

#[derive(Debug, Error)]
pub enum DigitalEsdWorldError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("world store error: {0}")]
    WorldStore(#[from] WorldStoreError),
    #[error("processing row lacks source_identity_reference")]
    MissingSourceIdentity,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigitalEsdWorldCounts {
    pub metadata_sources: i64,
    pub canonical_source_revisions: i64,
    pub exact_regions: i64,
    pub statements: i64,
    pub candidate_pnf_batches: i64,
    pub candidate_observations: i64,
    pub reviewed_world_records: i64,
    pub active_gaps: i64,
    pub active_obligations: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectionResidual {
    pub residual_kind: String,
    pub residual_ref: String,
    pub iteration_index: i64,
    pub surface_or_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DigitalEsdWorldReceipt {
    pub schema: &'static str,
    pub world_revision_ref: String,
    pub corpus_ref: String,
    pub compiler_ref: String,
    pub processing_ledger_sha256: String,
    pub denominator_rows_ingested: usize,
    pub counts: DigitalEsdWorldCounts,
    pub inspection: Vec<InspectionResidual>,
    pub inspection_is_bounded: bool,
    pub postgres_is_canonical_runtime_state: bool,
    pub json_is_canonical_runtime_state: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn source_ref(row: &Value) -> Option<&str> {
    row.get("source_identity_reference")
        .and_then(Value::as_str)
        .or_else(|| row.get("sourceIdentityReference").and_then(Value::as_str))
        .or_else(|| row.get("source_ref").and_then(Value::as_str))
}

pub fn ingest_processing_denominator(
    config: &DatabaseConfig,
    processing_ledger: &Path,
) -> Result<usize, DigitalEsdWorldError> {
    let file = BufReader::new(File::open(processing_ledger)?);
    let mut wire = Vec::new();
    let mut count = 0usize;

    for line in file.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let row: Value = serde_json::from_str(&line)?;
        let id = source_ref(&row)
            .filter(|value| !value.trim().is_empty())
            .ok_or(DigitalEsdWorldError::MissingSourceIdentity)?
            .to_owned();
        encode_record(
            &mut wire,
            &WireRecord {
                kind: WorldRecordKind::SourceManifestation,
                id,
                iteration_index: None,
                aux1: None,
                payload: serde_json::to_vec(&row)?,
            },
        )?;
        count += 1;
    }

    let mut store = WorldStore::connect(config)?;
    store.ingest_wire(Cursor::new(wire))?;
    Ok(count)
}

fn table_count(client: &mut Client, table: &str) -> Result<i64, postgres::Error> {
    let exists: bool = client
        .query_one("SELECT to_regclass($1) IS NOT NULL", &[&table])?
        .get(0);
    if !exists { return Ok(0); }
    let sql = format!("SELECT COUNT(*)::BIGINT FROM {table}");
    Ok(client.query_one(&sql, &[])?.get(0))
}

fn latest_iteration(client: &mut Client) -> Result<Option<i64>, postgres::Error> {
    let exists: bool = client
        .query_one("SELECT to_regclass('slr_world_v2_iteration') IS NOT NULL", &[])?
        .get(0);
    if !exists { return Ok(None); }
    Ok(client.query_one(latest_iteration_sql(), &[])?.get(0))
}

fn active_frontier(
    client: &mut Client,
    limit: usize,
) -> Result<(i64, i64, Vec<InspectionResidual>), postgres::Error> {
    let Some(iteration) = latest_iteration(client)? else {
        return Ok((0, 0, Vec::new()));
    };

    let gaps = client.query(active_frontier_gap_sql(), &[&iteration])?;
    let obligations = client.query(active_frontier_obligation_sql(), &[&iteration])?;
    let gap_count = gaps.len() as i64;
    let obligation_count = obligations.len() as i64;

    let mut inspection = Vec::new();
    for row in gaps.into_iter().take(limit) {
        inspection.push(InspectionResidual {
            residual_kind: "gap".into(),
            residual_ref: row.get(0),
            iteration_index: row.get(1),
            surface_or_kind: row.get(2),
        });
    }
    let remaining = limit.saturating_sub(inspection.len());
    for row in obligations.into_iter().take(remaining) {
        inspection.push(InspectionResidual {
            residual_kind: "obligation".into(),
            residual_ref: row.get(0),
            iteration_index: row.get(1),
            surface_or_kind: row.get(2),
        });
    }
    Ok((gap_count, obligation_count, inspection))
}

fn world_revision_ref(
    corpus_ref: &str,
    compiler_ref: &str,
    ledger_sha: &str,
    counts: &DigitalEsdWorldCounts,
) -> String {
    let payload = serde_json::to_vec(&(corpus_ref, compiler_ref, ledger_sha, counts))
        .expect("serializing fixed world revision payload cannot fail");
    format!("digital-esd-world:sha256:{:x}", Sha256::digest(payload))
}

pub fn materialize_digital_esd_world(
    config: &DatabaseConfig,
    processing_ledger: &Path,
    corpus_ref: &str,
    compiler_ref: &str,
    inspection_limit: usize,
) -> Result<DigitalEsdWorldReceipt, DigitalEsdWorldError> {
    let denominator_rows_ingested = ingest_processing_denominator(config, processing_ledger)?;
    let processing_ledger_sha256 = sha256_file(processing_ledger)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(WORLD_SCHEMA_SQL)?;

    let (active_gaps, active_obligations, inspection) =
        active_frontier(&mut client, inspection_limit)?;

    let counts = DigitalEsdWorldCounts {
        metadata_sources: table_count(&mut client, "slr_world_v2_source_manifestation")?,
        canonical_source_revisions: table_count(&mut client, "ingest.generic_source_revision")?,
        exact_regions: table_count(&mut client, "ingest.long_document_region")?,
        statements: table_count(&mut client, "corpus.source_statement")?,
        candidate_pnf_batches: table_count(&mut client, "pnf.statement_candidate_batch")?,
        candidate_observations: table_count(
            &mut client,
            "semantic.scale1_auto_observation_candidate",
        )?,
        reviewed_world_records: table_count(&mut client, "slr_world_v2_review")?,
        active_gaps,
        active_obligations,
    };

    let world_revision_ref =
        world_revision_ref(corpus_ref, compiler_ref, &processing_ledger_sha256, &counts);

    client.execute(
        r#"INSERT INTO digital_esd.world_revision (
            world_revision_ref, corpus_ref, compiler_ref, processing_ledger_sha256,
            metadata_source_count, canonical_source_revision_count, exact_region_count,
            statement_count, candidate_pnf_batch_count, candidate_observation_count,
            reviewed_world_record_count, active_gap_count, active_obligation_count,
            candidate_only, creates_semantic_authority, applicability_promoted,
            claim_truth_promoted
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,
            TRUE,FALSE,FALSE,FALSE
        ) ON CONFLICT (world_revision_ref) DO NOTHING"#,
        &[
            &world_revision_ref,
            &corpus_ref,
            &compiler_ref,
            &processing_ledger_sha256,
            &counts.metadata_sources,
            &counts.canonical_source_revisions,
            &counts.exact_regions,
            &counts.statements,
            &counts.candidate_pnf_batches,
            &counts.candidate_observations,
            &counts.reviewed_world_records,
            &counts.active_gaps,
            &counts.active_obligations,
        ],
    )?;

    Ok(DigitalEsdWorldReceipt {
        schema: "sensiblaw.digital-esd.db-native-world.v0_1",
        world_revision_ref,
        corpus_ref: corpus_ref.to_owned(),
        compiler_ref: compiler_ref.to_owned(),
        processing_ledger_sha256,
        denominator_rows_ingested,
        counts,
        inspection,
        inspection_is_bounded: true,
        postgres_is_canonical_runtime_state: true,
        json_is_canonical_runtime_state: false,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
