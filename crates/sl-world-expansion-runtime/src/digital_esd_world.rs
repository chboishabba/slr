use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::Path;

use postgres::{Client, NoTls};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::digital_esd_coordinates::{
    materialize_study_coordinate_candidates, CANONICAL_19_COORDINATES,
};
use crate::digital_esd_genealogy::ingest_study_family_hypotheses;

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
    screened BOOLEAN NOT NULL DEFAULT FALSE,
    screening_decision TEXT NOT NULL DEFAULT 'unresolved',
    retained BOOLEAN NOT NULL DEFAULT FALSE,
    verified BOOLEAN NOT NULL DEFAULT FALSE,
    materialised BOOLEAN NOT NULL DEFAULT FALSE,
    parsed BOOLEAN NOT NULL DEFAULT FALSE,
    reviewed BOOLEAN NOT NULL DEFAULT FALSE,
    admitted BOOLEAN NOT NULL DEFAULT FALSE,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (corpus_ref, source_ref),
    CHECK (NOT retained OR screened),
    CHECK (NOT verified OR retained),
    CHECK (NOT materialised OR verified),
    CHECK (NOT parsed OR materialised),
    CHECK (NOT reviewed OR parsed),
    CHECK (NOT admitted OR reviewed)
);

ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS screened BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS screening_decision TEXT NOT NULL DEFAULT 'unresolved';
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS retained BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS verified BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS materialised BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS parsed BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS reviewed BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE digital_esd.corpus_source
ADD COLUMN IF NOT EXISTS admitted BOOLEAN NOT NULL DEFAULT FALSE;

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
    candidate_study_coordinate_count BIGINT NOT NULL DEFAULT 0,
    reviewed_world_record_count BIGINT NOT NULL,
    study_family_hypothesis_count BIGINT NOT NULL DEFAULT 0,
    active_gap_count BIGINT NOT NULL,
    active_obligation_count BIGINT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE digital_esd.world_revision
ADD COLUMN IF NOT EXISTS candidate_study_coordinate_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE digital_esd.world_revision
ADD COLUMN IF NOT EXISTS study_family_hypothesis_count BIGINT NOT NULL DEFAULT 0;

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
    #[error("Digital-ESD coordinate materialization failed: {0}")]
    Coordinate(String),
    #[error("Digital-ESD genealogy materialization failed: {0}")]
    Genealogy(String),
}

#[derive(Debug, Clone)]
struct CorpusSourceState {
    source_ref: String,
    metadata_revision_ref: Option<String>,
    screened: bool,
    screening_decision: String,
    retained: bool,
    verified: bool,
    materialised: bool,
    parsed: bool,
    reviewed: bool,
    admitted: bool,
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
    pub candidate_study_coordinate_nominations: i64,
    pub substrate_review_records: i64,
    pub study_family_hypotheses: i64,
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
pub struct CoordinateCoverage {
    pub coordinate_ref: String,
    pub candidate_count: i64,
    pub distinct_source_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectionCoordinateCandidate {
    pub candidate_ref: String,
    pub source_ref: String,
    pub source_revision_ref: String,
    pub coordinate_ref: String,
    pub statement_ref: Option<String>,
    pub exact_span_ref: Option<String>,
    pub evidence_basis_ref: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InspectionStudyFamilyHypothesis {
    pub hypothesis_ref: String,
    pub left_source_ref: String,
    pub right_source_ref: String,
    pub relation_ref: String,
    pub evidence_json: String,
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
    pub coordinate_coverage: Vec<CoordinateCoverage>,
    pub coordinate_inspection: Vec<InspectionCoordinateCandidate>,
    pub genealogy_inspection: Vec<InspectionStudyFamilyHypothesis>,
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
    let mut memberships: Vec<CorpusSourceState> = Vec::new();

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
            "source_identity_reference": id.clone(),
            "metadata_revision_reference": metadata_revision.clone(),
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
        memberships.push(CorpusSourceState {
            source_ref: id,
            metadata_revision_ref: metadata_revision,
            screened: row.get("screened").and_then(Value::as_bool).unwrap_or(false),
            screening_decision: row
                .get("screening_decision")
                .and_then(Value::as_str)
                .unwrap_or("unresolved")
                .to_owned(),
            retained: row.get("retained").and_then(Value::as_bool).unwrap_or(false),
            verified: row.get("verified").and_then(Value::as_bool).unwrap_or(false),
            materialised: row.get("materialised").and_then(Value::as_bool).unwrap_or(false),
            parsed: row.get("parsed").and_then(Value::as_bool).unwrap_or(false),
            reviewed: row.get("reviewed").and_then(Value::as_bool).unwrap_or(false),
            admitted: row.get("admitted").and_then(Value::as_bool).unwrap_or(false),
        });
        screened += if row.get("screened").and_then(Value::as_bool).unwrap_or(false) { 1 } else { 0 };
        retained += if row.get("retained").and_then(Value::as_bool).unwrap_or(false) { 1 } else { 0 };
        verified += if row.get("verified").and_then(Value::as_bool).unwrap_or(false) { 1 } else { 0 };
        parsed += if row.get("parsed").and_then(Value::as_bool).unwrap_or(false) { 1 } else { 0 };
        reviewed += if row.get("reviewed").and_then(Value::as_bool).unwrap_or(false) { 1 } else { 0 };
        admitted += if row.get("admitted").and_then(Value::as_bool).unwrap_or(false) { 1 } else { 0 };
        count += 1;
    }

    let mut store = WorldStore::connect(config)?;
    store.ingest_wire(Cursor::new(wire))?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(WORLD_SCHEMA_SQL)?;
    let mut tx = client.transaction()?;
    for state in memberships {
        tx.execute(
            r#"INSERT INTO digital_esd.corpus_source (
                corpus_ref, source_ref, metadata_revision_ref,
                screened, screening_decision, retained, verified,
                materialised, parsed, reviewed, admitted,
                candidate_only, creates_semantic_authority
            ) VALUES (
                $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,TRUE,FALSE
            )
            ON CONFLICT (corpus_ref, source_ref) DO UPDATE SET
                metadata_revision_ref = EXCLUDED.metadata_revision_ref,
                screened = EXCLUDED.screened,
                screening_decision = EXCLUDED.screening_decision,
                retained = EXCLUDED.retained,
                verified = EXCLUDED.verified,
                materialised = EXCLUDED.materialised,
                parsed = EXCLUDED.parsed,
                reviewed = EXCLUDED.reviewed,
                admitted = EXCLUDED.admitted,
                candidate_only = TRUE,
                creates_semantic_authority = FALSE"#,
            &[
                &corpus_ref,
                &state.source_ref,
                &state.metadata_revision_ref,
                &state.screened,
                &state.screening_decision,
                &state.retained,
                &state.verified,
                &state.materialised,
                &state.parsed,
                &state.reviewed,
                &state.admitted,
            ],
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
        candidate_study_coordinate_nominations: 0,
        substrate_review_records: 0,
        study_family_hypotheses: 0,
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


fn coordinate_inspection(
    client: &mut Client,
    corpus_ref: &str,
    limit: usize,
) -> Result<(Vec<CoordinateCoverage>, Vec<InspectionCoordinateCandidate>), postgres::Error> {
    let rows = client.query(
        r#"
        SELECT coordinate_ref, COUNT(*)::BIGINT, COUNT(DISTINCT source_ref)::BIGINT
        FROM digital_esd.study_coordinate_candidate
        WHERE corpus_ref=$1
          AND candidate_only AND review_required
          AND NOT coordinate_paid AND NOT automatic_absence_inference
          AND NOT creates_semantic_authority
          AND NOT applicability_promoted AND NOT claim_truth_promoted
        GROUP BY coordinate_ref
        "#,
        &[&corpus_ref],
    )?;
    let observed = rows
        .into_iter()
        .map(|row| {
            (
                row.get::<_, String>(0),
                (row.get::<_, i64>(1), row.get::<_, i64>(2)),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let coverage = CANONICAL_19_COORDINATES
        .iter()
        .map(|coordinate| {
            let (candidate_count, distinct_source_count) =
                observed.get(*coordinate).copied().unwrap_or((0, 0));
            CoordinateCoverage {
                coordinate_ref: (*coordinate).to_owned(),
                candidate_count,
                distinct_source_count,
            }
        })
        .collect();

    let candidates = client
        .query(
            r#"
            SELECT candidate_ref, source_ref, source_revision_ref, coordinate_ref,
                   statement_ref, exact_span_ref, evidence_basis_ref
            FROM digital_esd.study_coordinate_candidate
            WHERE corpus_ref=$1
              AND candidate_only AND review_required
              AND NOT coordinate_paid AND NOT automatic_absence_inference
              AND NOT creates_semantic_authority
              AND NOT applicability_promoted AND NOT claim_truth_promoted
            ORDER BY source_ref, coordinate_ref, exact_span_ref NULLS FIRST, candidate_ref
            LIMIT $2
            "#,
            &[&corpus_ref, &(limit as i64)],
        )?
        .into_iter()
        .map(|row| InspectionCoordinateCandidate {
            candidate_ref: row.get(0),
            source_ref: row.get(1),
            source_revision_ref: row.get(2),
            coordinate_ref: row.get(3),
            statement_ref: row.get(4),
            exact_span_ref: row.get(5),
            evidence_basis_ref: row.get(6),
        })
        .collect();
    Ok((coverage, candidates))
}


fn genealogy_inspection(
    client: &mut Client,
    corpus_ref: &str,
    limit: usize,
) -> Result<Vec<InspectionStudyFamilyHypothesis>, postgres::Error> {
    client
        .query(
            r#"
            SELECT hypothesis_ref, left_source_ref, right_source_ref,
                   relation_ref, evidence_json
            FROM digital_esd.study_family_hypothesis
            WHERE corpus_ref=$1
              AND candidate_only AND review_required
              AND NOT creates_duplicate_decision
              AND NOT creates_same_empirical_study
              AND NOT creates_evidence_independence
              AND NOT creates_semantic_authority
              AND NOT claim_truth_promoted
            ORDER BY relation_ref, left_source_ref, right_source_ref, hypothesis_ref
            LIMIT $2
            "#,
            &[&corpus_ref, &(limit as i64)],
        )?
        .into_iter()
        .map(|row| InspectionStudyFamilyHypothesis {
            hypothesis_ref: row.get(0),
            left_source_ref: row.get(1),
            right_source_ref: row.get(2),
            relation_ref: row.get(3),
            evidence_json: row.get(4),
        })
        .collect()
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
    study_family_hypotheses_path: Option<&Path>,
) -> Result<DigitalEsdWorldReceipt, DigitalEsdWorldError> {
    let processing_ledger_sha256 = sha256_file(processing_ledger)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(WORLD_SCHEMA_SQL)?;

    let (denominator_rows_ingested, mut counts) =
        ingest_processing_denominator(config, processing_ledger, corpus_ref)?;

    let coordinate_receipt =
        materialize_study_coordinate_candidates(config, corpus_ref)
            .map_err(|error| DigitalEsdWorldError::Coordinate(error.to_string()))?;
    let genealogy_receipt =
        ingest_study_family_hypotheses(config, corpus_ref, study_family_hypotheses_path)
            .map_err(|error| DigitalEsdWorldError::Genealogy(error.to_string()))?;

    let (active_gaps, active_obligations, inspection) =
        active_frontier(&mut client, inspection_limit)?;
    let (coordinate_coverage, coordinate_inspection) =
        coordinate_inspection(&mut client, corpus_ref, inspection_limit)?;
    let genealogy_inspection =
        genealogy_inspection(&mut client, corpus_ref, inspection_limit)?;

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
    counts.candidate_study_coordinate_nominations = client
        .query_one(
            "SELECT COUNT(*)::BIGINT
             FROM digital_esd.study_coordinate_candidate
             WHERE corpus_ref=$1 AND candidate_only AND review_required
               AND NOT coordinate_paid AND NOT automatic_absence_inference
               AND NOT creates_semantic_authority
               AND NOT applicability_promoted AND NOT claim_truth_promoted",
            &[&corpus_ref],
        )?
        .get(0);
    debug_assert_eq!(
        coordinate_receipt.total_candidates,
        coordinate_receipt.source_level_candidates + coordinate_receipt.statement_level_candidates
    );
    counts.substrate_review_records =
        table_count(&mut client, "slr_world_v2_review")?;
    counts.study_family_hypotheses = genealogy_receipt.persisted_hypotheses;
    counts.active_gaps = active_gaps;
    counts.active_obligations = active_obligations;

    let world_revision_ref =
        world_revision_ref(corpus_ref, compiler_ref, &processing_ledger_sha256, &counts);

    client.execute(
        r#"INSERT INTO digital_esd.world_revision (
            world_revision_ref, corpus_ref, compiler_ref, processing_ledger_sha256,
            metadata_source_count, canonical_source_revision_count, exact_region_count,
            statement_count, candidate_pnf_batch_count, candidate_observation_count,
            candidate_study_coordinate_count, reviewed_world_record_count,
            study_family_hypothesis_count, active_gap_count, active_obligation_count,
            candidate_only, creates_semantic_authority, applicability_promoted,
            claim_truth_promoted
        ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,
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
            &counts.candidate_study_coordinate_nominations,
            &counts.substrate_review_records,
            &counts.study_family_hypotheses,
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
        coordinate_coverage,
        coordinate_inspection,
        genealogy_inspection,
        inspection_is_bounded: true,
        postgres_is_canonical_runtime_state: true,
        json_is_canonical_runtime_state: false,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_revision_is_deterministic_and_sensitive_to_corpus_state() {
        let counts = DigitalEsdWorldCounts {
            metadata_sources: 43_996,
            screened_sources: 200,
            retained_sources: 28,
            verified_fulltext_sources: 10,
            parsed_sources: 10,
            reviewed_sources: 0,
            admitted_sources: 0,
            substrate_source_revisions: 10,
            substrate_exact_regions: 12_980,
            substrate_statements: 1_000,
            substrate_candidate_pnf_batches: 1_000,
            substrate_candidate_observations: 1_000,
            candidate_study_coordinate_nominations: 500,
            substrate_review_records: 0,
            study_family_hypotheses: 420,
            active_gaps: 5,
            active_obligations: 5,
        };
        let a = world_revision_ref(
            "digital-esd:eric:43996",
            "compiler:v1",
            "sha256:ledger",
            &counts,
        );
        let b = world_revision_ref(
            "digital-esd:eric:43996",
            "compiler:v1",
            "sha256:ledger",
            &counts,
        );
        assert_eq!(a, b);

        let changed = world_revision_ref(
            "digital-esd:eric:43996",
            "compiler:v1",
            "sha256:other-ledger",
            &counts,
        );
        assert_ne!(a, changed);
    }
}
