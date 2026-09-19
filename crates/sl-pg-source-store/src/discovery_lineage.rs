//! Durable append-only lineage for residual-driven world discovery.
//!
//! Persistence records why an object representation was acquired/admitted, the
//! reviewed world-identity class it belongs to, and what observed PNF/world
//! delta followed. It is intentionally provider-neutral and does not depend on
//! proof-search runtime types.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

const DISCOVERY_LINEAGE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.discovery_lineage_receipt (
  receipt_sha256 TEXT PRIMARY KEY,
  object_ref TEXT NOT NULL,
  identity_class_ref TEXT,
  discovery_parent_ref TEXT NOT NULL,
  triggering_residual_ref TEXT NOT NULL,
  selected_candidate_ref TEXT NOT NULL,
  producer_lane_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  pnf_world_disambiguation_ref TEXT NOT NULL,
  expected_residual_contraction BIGINT NOT NULL CHECK (expected_residual_contraction >= 0),
  observed_residual_contraction BIGINT NOT NULL CHECK (observed_residual_contraction >= 0),
  new_residual_refs TEXT[] NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_authority TEXT NOT NULL
);
ALTER TABLE context.discovery_lineage_receipt
  ADD COLUMN IF NOT EXISTS identity_class_ref TEXT;
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryLineageInput {
    pub object_ref: String,
    pub identity_class_ref: String,
    pub discovery_parent_ref: String,
    pub triggering_residual_ref: String,
    pub selected_candidate_ref: String,
    pub producer_lane_ref: String,
    pub source_revision_ref: String,
    pub pnf_world_disambiguation_ref: String,
    pub expected_residual_contraction: u64,
    pub observed_residual_contraction: u64,
    pub new_residual_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_authority: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryLineageRow {
    pub object_ref: String,
    pub identity_class_ref: String,
    pub discovery_parent_ref: String,
    pub triggering_residual_ref: String,
    pub selected_candidate_ref: String,
    pub producer_lane_ref: String,
    pub source_revision_ref: String,
    pub pnf_world_disambiguation_ref: String,
    pub expected_residual_contraction: u64,
    pub observed_residual_contraction: u64,
    pub new_residual_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_authority: String,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryLineageMaterializationReceipt {
    pub attempted_count: usize,
    pub materialized_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DiscoveryLineageError {
    #[error("lineage must remain candidate-only")]
    LineageMustRemainCandidateOnly,
    #[error("lineage persistence may not promote authority/applicability/truth")]
    LineageMayNotPromote,
    #[error("lineage coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("lineage receipt authority is invalid")]
    InvalidReceiptAuthority,
    #[error("lineage contraction exceeds PostgreSQL BIGINT range")]
    ContractionOutOfRange,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for DiscoveryLineageError {
    fn from(value: postgres::Error) -> Self { Self::Postgres(value.to_string()) }
}

fn hex_digest(bytes: &[u8]) -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() }

pub fn discovery_lineage_row(lineage: &DiscoveryLineageInput) -> Result<DiscoveryLineageRow, DiscoveryLineageError> {
    if !lineage.candidate_only { return Err(DiscoveryLineageError::LineageMustRemainCandidateOnly); }
    if lineage.creates_semantic_authority || lineage.applicability_promoted || lineage.claim_truth_promoted {
        return Err(DiscoveryLineageError::LineageMayNotPromote);
    }
    if lineage.receipt_authority != "candidate_world_expansion_only" { return Err(DiscoveryLineageError::InvalidReceiptAuthority); }
    for (name, value) in [
        ("object_ref", lineage.object_ref.as_str()),
        ("identity_class_ref", lineage.identity_class_ref.as_str()),
        ("discovery_parent_ref", lineage.discovery_parent_ref.as_str()),
        ("triggering_residual_ref", lineage.triggering_residual_ref.as_str()),
        ("selected_candidate_ref", lineage.selected_candidate_ref.as_str()),
        ("producer_lane_ref", lineage.producer_lane_ref.as_str()),
        ("source_revision_ref", lineage.source_revision_ref.as_str()),
        ("pnf_world_disambiguation_ref", lineage.pnf_world_disambiguation_ref.as_str()),
    ] {
        if value.trim().is_empty() { return Err(DiscoveryLineageError::EmptyCoordinate(name)); }
    }
    if lineage.expected_residual_contraction > i64::MAX as u64 || lineage.observed_residual_contraction > i64::MAX as u64 {
        return Err(DiscoveryLineageError::ContractionOutOfRange);
    }

    let mut hasher = Sha256::new();
    for value in [
        "mabo-discovery-lineage:v2",
        lineage.object_ref.as_str(),
        lineage.identity_class_ref.as_str(),
        lineage.discovery_parent_ref.as_str(),
        lineage.triggering_residual_ref.as_str(),
        lineage.selected_candidate_ref.as_str(),
        lineage.producer_lane_ref.as_str(),
        lineage.source_revision_ref.as_str(),
        lineage.pnf_world_disambiguation_ref.as_str(),
        lineage.receipt_authority.as_str(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    hasher.update(lineage.expected_residual_contraction.to_le_bytes());
    hasher.update(lineage.observed_residual_contraction.to_le_bytes());
    for residual_ref in &lineage.new_residual_refs { hasher.update(residual_ref.as_bytes()); hasher.update([0]); }
    let receipt_sha256 = hex_digest(&hasher.finalize());

    Ok(DiscoveryLineageRow {
        object_ref: lineage.object_ref.clone(),
        identity_class_ref: lineage.identity_class_ref.clone(),
        discovery_parent_ref: lineage.discovery_parent_ref.clone(),
        triggering_residual_ref: lineage.triggering_residual_ref.clone(),
        selected_candidate_ref: lineage.selected_candidate_ref.clone(),
        producer_lane_ref: lineage.producer_lane_ref.clone(),
        source_revision_ref: lineage.source_revision_ref.clone(),
        pnf_world_disambiguation_ref: lineage.pnf_world_disambiguation_ref.clone(),
        expected_residual_contraction: lineage.expected_residual_contraction,
        observed_residual_contraction: lineage.observed_residual_contraction,
        new_residual_refs: lineage.new_residual_refs.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: lineage.receipt_authority.clone(),
        receipt_sha256,
    })
}

pub fn materialize_discovery_lineage(
    config: &DatabaseConfig,
    lineages: &[DiscoveryLineageInput],
) -> Result<DiscoveryLineageMaterializationReceipt, DiscoveryLineageError> {
    let rows = lineages.iter().map(discovery_lineage_row).collect::<Result<Vec<_>, _>>()?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(DISCOVERY_LINEAGE_SCHEMA_SQL)?;
    let mut materialized_count = 0usize;
    for row in &rows {
        materialized_count += tx.execute(
            "INSERT INTO context.discovery_lineage_receipt (\
             receipt_sha256, object_ref, identity_class_ref, discovery_parent_ref, triggering_residual_ref, \
             selected_candidate_ref, producer_lane_ref, source_revision_ref, pnf_world_disambiguation_ref, \
             expected_residual_contraction, observed_residual_contraction, new_residual_refs, candidate_only, \
             creates_semantic_authority, applicability_promoted, claim_truth_promoted, receipt_authority) VALUES (\
             $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,TRUE,FALSE,FALSE,FALSE,$13) ON CONFLICT DO NOTHING",
            &[
                &row.receipt_sha256, &row.object_ref, &row.identity_class_ref, &row.discovery_parent_ref,
                &row.triggering_residual_ref, &row.selected_candidate_ref, &row.producer_lane_ref,
                &row.source_revision_ref, &row.pnf_world_disambiguation_ref,
                &(row.expected_residual_contraction as i64), &(row.observed_residual_contraction as i64),
                &row.new_residual_refs, &row.receipt_authority,
            ],
        )? as usize;
    }
    tx.commit()?;

    Ok(DiscoveryLineageMaterializationReceipt {
        attempted_count: rows.len(), materialized_count, candidate_only: true,
        creates_semantic_authority: false, applicability_promoted: false, claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lineage() -> DiscoveryLineageInput {
        DiscoveryLineageInput {
            object_ref: "case:[1992]-HCA-23".into(), identity_class_ref: "world-object:mabo-case-1992-hca-23".into(),
            discovery_parent_ref: "Q1501525".into(), triggering_residual_ref: "residual:mabo:authority-source".into(),
            selected_candidate_ref: "oalc:case:[1992]-HCA-23".into(), producer_lane_ref: "governed-legal".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(), pnf_world_disambiguation_ref: "pnf-world:mabo:5".into(),
            expected_residual_contraction: 4, observed_residual_contraction: 2,
            new_residual_refs: vec!["residual:mabo:case-follow".into()], candidate_only: true,
            creates_semantic_authority: false, applicability_promoted: false, claim_truth_promoted: false,
            receipt_authority: "candidate_world_expansion_only".into(),
        }
    }

    #[test]
    fn lineage_row_preserves_identity_discovery_and_observed_delta_coordinates() {
        let row = discovery_lineage_row(&lineage()).unwrap();
        assert_eq!(row.object_ref, "case:[1992]-HCA-23");
        assert_eq!(row.identity_class_ref, "world-object:mabo-case-1992-hca-23");
        assert_eq!(row.discovery_parent_ref, "Q1501525");
        assert_eq!(row.triggering_residual_ref, "residual:mabo:authority-source");
        assert_eq!(row.producer_lane_ref, "governed-legal");
        assert_eq!(row.source_revision_ref, "oalc:[1992]-HCA-23:sha256:abc");
        assert_eq!(row.expected_residual_contraction, 4);
        assert_eq!(row.observed_residual_contraction, 2);
        assert_eq!(row.new_residual_refs, vec!["residual:mabo:case-follow"]);
        assert_eq!(row.receipt_sha256.len(), 64);
    }

    #[test]
    fn same_lineage_is_hash_stable_and_changed_identity_changes_receipt() {
        let first = discovery_lineage_row(&lineage()).unwrap();
        let second = discovery_lineage_row(&lineage()).unwrap();
        assert_eq!(first.receipt_sha256, second.receipt_sha256);
        let mut changed = lineage();
        changed.identity_class_ref = "world-object:other".into();
        assert_ne!(first.receipt_sha256, discovery_lineage_row(&changed).unwrap().receipt_sha256);
    }

    #[test]
    fn promoted_or_non_candidate_lineage_fails_closed() {
        let mut bad = lineage(); bad.candidate_only = false;
        assert_eq!(discovery_lineage_row(&bad), Err(DiscoveryLineageError::LineageMustRemainCandidateOnly));
        bad.candidate_only = true; bad.creates_semantic_authority = true;
        assert_eq!(discovery_lineage_row(&bad), Err(DiscoveryLineageError::LineageMayNotPromote));
    }
}
