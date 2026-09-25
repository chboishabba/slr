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

CREATE TABLE IF NOT EXISTS digital_esd.corpus_source (
    corpus_ref TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    metadata_revision_ref TEXT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (corpus_ref, source_ref)
);

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
    pub screened_sources: i64,
    pub retained_sources: i64,
    pub verified_fulltext_sources: i64,
    pub parsed_sources: i64,
    pub reviewed_sources: i64,
    pub admitted_sources: i64,
    pub substrate_source_revisions: i64,
    pub substrate_exact_regions: i64,
    pub substrate_statements: i64,
    pub substrate_candidate_pnf_batches: i64,
    pub substrate_candidate_observations: i64,
    pub substrate_review_records: i64,
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
    corpus_ref: &str,
) -> Result<(usize, DigitalEsdWorldCounts), DigitalEsdWorldError> {
    let file = BufReader::new(File::open(processing_ledger)?);
    let mut wire = Vec::new();
    let mut count = 0usize;
    let mut screened = 0i64;
    let mut retained = 0i64;
    let mut verified = 0i64;
    let mut parsed = 0i64;
    let mut reviewed = 0i64;
    let mut admitted = 0i64;
    let mut memberships: Vec<(String, Option<String>)> = Vec::new();

    for line in file.lines() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        let row: Value = serde_json::from_str(&line)?;
        let id = source_ref(&row)
            .filter(|value| !value.trim().is_empty())
            .ok_or(DigitalEsdWorldError::MissingSourceIdentity)?
            .to_owned();
        let metadata_revision = row
            .get("metadata_revision_reference")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let manifestation_payload = serde_json::json!({
            "schema": "digital-esd-metadata-manifestation-v1",
            "source_identity_reference": id,
            "metadata_revision_reference": metadata_revision,
            "candidate_only": true,
            "creates_semantic_authority": false,
            "claim_truth_promoted": false
        });
        encode_record(
            &mut wire,
            &WireRecord {
                kind: WorldRecordKind::SourceManifestation,
                id: id.clone(),
                iteration_index: None,
                aux1: None,
                payload: serde_json::to_vec(&manifestation_payload)?,
            },
        )?;
        memberships.push((id, metadata_revision));
        screened += i64::from(row.get("screened").and_then(Value::as_bool).unwrap_or(false));
        retained += i64::from(row.get("retained").and_then(Value::as_bool).unwrap_or(false));
        verified += i64::from(row.get("verified").and_then(Value::as_bool).unwrap_or(false));
        parsed += i64::from(row.get("parsed").and_then(Value::as_bool).unwrap_or(false));
        reviewed += i64::from(row.get("reviewed").and_then(Value::as_bool).unwrap_or(false));
        admitted += i64::from(row.get("admitted").and_then(Value::as_bool).unwrap_or(false));
        count += 1;
    }

    let mut store = WorldStore::connect(config)?;
    store.ingest_wire(Cursor::new(wire))?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(WORLD_SCHEMA_SQL)?;
    let mut tx = client.transaction()?;
    for (source_ref, metadata_revision_ref) in memberships {
        tx.execute(
            r#"INSERT INTO digital_esd.corpus_source (
                corpus_ref, source_ref, metadata_revision_ref,
                candidate_only, creates_semantic_authority
            ) VALUES ($1,$2,$3,TRUE,FALSE)
            ON CONFLICT (corpus_ref, source_ref) DO UPDATE SET
                metadata_revision_ref = EXCLUDED.metadata_revision_ref,
                candidate_only = TRUE,
                creates_semantic_authority = FALSE"#,
            &[&corpus_ref, &source_ref, &metadata_revision_ref],
        )?;
    }
    tx.commit()?;

    Ok((count, DigitalEsdWorldCounts {
        metadata_sources: count as i64,
        screened_sources: screened,
        retained_sources: retained,
        verified_fulltext_sources: verified,
        parsed_sources: parsed,
        reviewed_sources: reviewed,
        admitted_sources: admitted,
        substrate_source_revisions: 0,
        substrate_exact_regions: 0,
        substrate_statements: 0,
        substrate_candidate_pnf_batches: 0,
        substrate_candidate_observations: 0,
        substrate_review_records: 0,
        active_gaps: 0,
        active_obligations: 0,
    }))
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
    let processing_ledger_sha256 = sha256_file(processing_ledger)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(WORLD_SCHEMA_SQL)?;

    let (denominator_rows_ingested, mut counts) =
        ingest_processing_denominator(config, processing_ledger, corpus_ref)?;

    let (active_gaps, active_obligations, inspection) =
        active_frontier(&mut client, inspection_limit)?;

    counts.substrate_source_revisions =
        table_count(&mut client, "ingest.generic_source_revision")?;
    counts.substrate_exact_regions =
        table_count(&mut client, "ingest.long_document_region")?;
    counts.substrate_statements =
        table_count(&mut client, "corpus.source_statement")?;
    counts.substrate_candidate_pnf_batches =
        table_count(&mut client, "pnf.statement_candidate_batch")?;
    counts.substrate_candidate_observations =
        table_count(&mut client, "semantic.scale1_auto_observation_candidate")?;
    counts.substrate_review_records =
        table_count(&mut client, "slr_world_v2_review")?;
    counts.active_gaps = active_gaps;
    counts.active_obligations = active_obligations;

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
            &counts.substrate_source_revisions,
            &counts.substrate_exact_regions,
            &counts.substrate_statements,
            &counts.substrate_candidate_pnf_batches,
            &counts.substrate_candidate_observations,
            &counts.substrate_review_records,
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
