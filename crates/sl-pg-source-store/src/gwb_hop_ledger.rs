//! Durable trajectory ledger for the GWB ambiguity-directed campaign.
//!
//! Each committed reviewed hop extends exactly one previous receipt. The ledger
//! records runtime/review provenance only and cannot create semantic authority,
//! applicability, or claim truth.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub(crate) const GWB_HOP_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.gwb_adaptive_hop_receipt (
  receipt_sha256 TEXT PRIMARY KEY,
  campaign_ref TEXT NOT NULL,
  hop_index BIGINT NOT NULL CHECK (hop_index >= 0),
  prior_receipt_sha256 TEXT,
  world_before_sha256 TEXT NOT NULL,
  frontier_sha256 TEXT NOT NULL,
  selected_move_ref TEXT NOT NULL,
  investigation_kind_ref TEXT NOT NULL,
  producer_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  evidence_digest_ref TEXT NOT NULL,
  review_ref TEXT NOT NULL,
  outcome_ref TEXT NOT NULL,
  residual_effect_ref TEXT NOT NULL,
  world_after_sha256 TEXT NOT NULL,
  closed_residual_refs TEXT[] NOT NULL DEFAULT '{}',
  opened_residual_refs TEXT[] NOT NULL DEFAULT '{}',
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_authority TEXT NOT NULL CHECK (receipt_authority = 'gwb_adaptive_runtime_review_only'),
  UNIQUE (campaign_ref, hop_index)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbHopLedgerInput {
    pub campaign_ref: String,
    pub hop_index: usize,
    pub prior_receipt_sha256: Option<String>,
    pub world_before_sha256: String,
    pub frontier_sha256: String,
    pub selected_move_ref: String,
    pub investigation_kind_ref: String,
    pub producer_ref: String,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
    pub review_ref: String,
    pub outcome_ref: String,
    pub residual_effect_ref: String,
    pub world_after_sha256: String,
    pub closed_residual_refs: Vec<String>,
    pub opened_residual_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbHopLedgerRow {
    pub campaign_ref: String,
    pub hop_index: usize,
    pub prior_receipt_sha256: Option<String>,
    pub world_before_sha256: String,
    pub frontier_sha256: String,
    pub selected_move_ref: String,
    pub investigation_kind_ref: String,
    pub producer_ref: String,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
    pub review_ref: String,
    pub outcome_ref: String,
    pub residual_effect_ref: String,
    pub world_after_sha256: String,
    pub closed_residual_refs: Vec<String>,
    pub opened_residual_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_authority: &'static str,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbHopLedgerMaterializationReceipt {
    pub campaign_ref: String,
    pub hop_index: usize,
    pub receipt_sha256: String,
    pub materialized: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GwbHopLedgerError {
    #[error("GWB hop ledger coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("invalid sha256 reference: {0}")]
    InvalidSha256(String),
    #[error("GWB hop ledger must remain candidate-only")]
    LedgerMustRemainCandidateOnly,
    #[error("GWB hop ledger may not promote authority/applicability/truth")]
    LedgerMayNotPromote,
    #[error("hop zero may not name a prior receipt")]
    UnexpectedPriorAtHopZero,
    #[error("non-zero GWB hop must name its prior receipt")]
    MissingPriorReceipt,
    #[error("GWB hop index exceeds PostgreSQL BIGINT range")]
    HopIndexOutOfRange,
    #[error("first persisted GWB hop must be zero, observed {0}")]
    FirstHopNotZero(usize),
    #[error("GWB hop index is not consecutive: expected {expected}, observed {observed}")]
    NonConsecutiveHop { expected: usize, observed: usize },
    #[error("GWB prior receipt mismatch: expected {expected}, observed {observed:?}")]
    PriorReceiptMismatch {
        expected: String,
        observed: Option<String>,
    },
    #[error("existing GWB hop conflicts with supplied receipt")]
    ExistingHopConflict,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for GwbHopLedgerError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn valid_sha256_ref(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn valid_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn gwb_hop_ledger_row(
    input: &GwbHopLedgerInput,
) -> Result<GwbHopLedgerRow, GwbHopLedgerError> {
    for (name, value) in [
        ("campaign_ref", input.campaign_ref.as_str()),
        ("world_before_sha256", input.world_before_sha256.as_str()),
        ("frontier_sha256", input.frontier_sha256.as_str()),
        ("selected_move_ref", input.selected_move_ref.as_str()),
        ("investigation_kind_ref", input.investigation_kind_ref.as_str()),
        ("producer_ref", input.producer_ref.as_str()),
        ("source_revision_ref", input.source_revision_ref.as_str()),
        ("evidence_digest_ref", input.evidence_digest_ref.as_str()),
        ("review_ref", input.review_ref.as_str()),
        ("outcome_ref", input.outcome_ref.as_str()),
        ("residual_effect_ref", input.residual_effect_ref.as_str()),
        ("world_after_sha256", input.world_after_sha256.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(GwbHopLedgerError::EmptyCoordinate(name));
        }
    }
    for digest in [
        input.world_before_sha256.as_str(),
        input.frontier_sha256.as_str(),
        input.evidence_digest_ref.as_str(),
        input.world_after_sha256.as_str(),
    ] {
        if !valid_sha256_ref(digest) {
            return Err(GwbHopLedgerError::InvalidSha256(digest.to_owned()));
        }
    }
    if !input.candidate_only {
        return Err(GwbHopLedgerError::LedgerMustRemainCandidateOnly);
    }
    if input.creates_semantic_authority
        || input.applicability_promoted
        || input.claim_truth_promoted
    {
        return Err(GwbHopLedgerError::LedgerMayNotPromote);
    }
    match (input.hop_index, input.prior_receipt_sha256.as_deref()) {
        (0, Some(_)) => return Err(GwbHopLedgerError::UnexpectedPriorAtHopZero),
        (0, None) => {}
        (_, None) => return Err(GwbHopLedgerError::MissingPriorReceipt),
        (_, Some(value)) if !valid_sha256_hex(value) => {
            return Err(GwbHopLedgerError::InvalidSha256(value.to_owned()))
        }
        (_, Some(_)) => {}
    }
    if input.hop_index > i64::MAX as usize {
        return Err(GwbHopLedgerError::HopIndexOutOfRange);
    }

    let mut closed = input.closed_residual_refs.clone();
    closed.sort();
    closed.dedup();
    let mut opened = input.opened_residual_refs.clone();
    opened.sort();
    opened.dedup();

    let mut hasher = Sha256::new();
    for value in [
        "gwb-adaptive-hop:v1",
        input.campaign_ref.as_str(),
        &input.hop_index.to_string(),
        input.prior_receipt_sha256.as_deref().unwrap_or(""),
        input.world_before_sha256.as_str(),
        input.frontier_sha256.as_str(),
        input.selected_move_ref.as_str(),
        input.investigation_kind_ref.as_str(),
        input.producer_ref.as_str(),
        input.source_revision_ref.as_str(),
        input.evidence_digest_ref.as_str(),
        input.review_ref.as_str(),
        input.outcome_ref.as_str(),
        input.residual_effect_ref.as_str(),
        input.world_after_sha256.as_str(),
    ] {
        hash_field(&mut hasher, value);
    }
    for residual in &closed {
        hash_field(&mut hasher, "closed");
        hash_field(&mut hasher, residual);
    }
    for residual in &opened {
        hash_field(&mut hasher, "opened");
        hash_field(&mut hasher, residual);
    }

    Ok(GwbHopLedgerRow {
        campaign_ref: input.campaign_ref.clone(),
        hop_index: input.hop_index,
        prior_receipt_sha256: input.prior_receipt_sha256.clone(),
        world_before_sha256: input.world_before_sha256.clone(),
        frontier_sha256: input.frontier_sha256.clone(),
        selected_move_ref: input.selected_move_ref.clone(),
        investigation_kind_ref: input.investigation_kind_ref.clone(),
        producer_ref: input.producer_ref.clone(),
        source_revision_ref: input.source_revision_ref.clone(),
        evidence_digest_ref: input.evidence_digest_ref.clone(),
        review_ref: input.review_ref.clone(),
        outcome_ref: input.outcome_ref.clone(),
        residual_effect_ref: input.residual_effect_ref.clone(),
        world_after_sha256: input.world_after_sha256.clone(),
        closed_residual_refs: closed,
        opened_residual_refs: opened,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "gwb_adaptive_runtime_review_only",
        receipt_sha256: hex(&hasher.finalize()),
    })
}

pub fn load_gwb_reviewed_move_refs(
    config: &DatabaseConfig,
    campaign_ref: &str,
) -> Result<std::collections::BTreeSet<String>, GwbHopLedgerError> {
    if campaign_ref.trim().is_empty() {
        return Err(GwbHopLedgerError::EmptyCoordinate("campaign_ref"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(GWB_HOP_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT selected_move_ref FROM context.gwb_adaptive_hop_receipt \
         WHERE campaign_ref = $1 \
           AND receipt_authority = 'gwb_adaptive_runtime_review_only' \
         ORDER BY hop_index",
        &[&campaign_ref],
    )?;
    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

pub fn load_gwb_hops(
    config: &DatabaseConfig,
    campaign_ref: &str,
) -> Result<Vec<GwbHopLedgerRow>, GwbHopLedgerError> {
    if campaign_ref.trim().is_empty() {
        return Err(GwbHopLedgerError::EmptyCoordinate("campaign_ref"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(GWB_HOP_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT campaign_ref, hop_index, prior_receipt_sha256, world_before_sha256, \
                frontier_sha256, selected_move_ref, investigation_kind_ref, producer_ref, \
                source_revision_ref, evidence_digest_ref, review_ref, outcome_ref, \
                residual_effect_ref, world_after_sha256, closed_residual_refs, \
                opened_residual_refs, candidate_only, creates_semantic_authority, \
                applicability_promoted, claim_truth_promoted, receipt_sha256 \
         FROM context.gwb_adaptive_hop_receipt \
         WHERE campaign_ref = $1 \
           AND receipt_authority = 'gwb_adaptive_runtime_review_only' \
         ORDER BY hop_index",
        &[&campaign_ref],
    )?;
    rows.into_iter()
        .map(|row| {
            let index: i64 = row.get(1);
            Ok(GwbHopLedgerRow {
                campaign_ref: row.get(0),
                hop_index: usize::try_from(index)
                    .map_err(|_| GwbHopLedgerError::HopIndexOutOfRange)?,
                prior_receipt_sha256: row.get(2),
                world_before_sha256: row.get(3),
                frontier_sha256: row.get(4),
                selected_move_ref: row.get(5),
                investigation_kind_ref: row.get(6),
                producer_ref: row.get(7),
                source_revision_ref: row.get(8),
                evidence_digest_ref: row.get(9),
                review_ref: row.get(10),
                outcome_ref: row.get(11),
                residual_effect_ref: row.get(12),
                world_after_sha256: row.get(13),
                closed_residual_refs: row.get(14),
                opened_residual_refs: row.get(15),
                candidate_only: row.get(16),
                creates_semantic_authority: row.get(17),
                applicability_promoted: row.get(18),
                claim_truth_promoted: row.get(19),
                receipt_authority: "gwb_adaptive_runtime_review_only",
                receipt_sha256: row.get(20),
            })
        })
        .collect()
}

pub fn load_latest_gwb_hop(
    config: &DatabaseConfig,
    campaign_ref: &str,
) -> Result<Option<GwbHopLedgerRow>, GwbHopLedgerError> {
    if campaign_ref.trim().is_empty() {
        return Err(GwbHopLedgerError::EmptyCoordinate("campaign_ref"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(GWB_HOP_SCHEMA_SQL)?;
    let row = client.query_opt(
        "SELECT campaign_ref, hop_index, prior_receipt_sha256, world_before_sha256,                 frontier_sha256, selected_move_ref, investigation_kind_ref, producer_ref,                 source_revision_ref, evidence_digest_ref, review_ref, outcome_ref,                 residual_effect_ref, world_after_sha256, closed_residual_refs,                 opened_residual_refs, candidate_only, creates_semantic_authority,                 applicability_promoted, claim_truth_promoted, receipt_sha256          FROM context.gwb_adaptive_hop_receipt          WHERE campaign_ref = $1            AND receipt_authority = 'gwb_adaptive_runtime_review_only'          ORDER BY hop_index DESC LIMIT 1",
        &[&campaign_ref],
    )?;
    let Some(row) = row else {
        return Ok(None);
    };
    let index: i64 = row.get(1);
    Ok(Some(GwbHopLedgerRow {
        campaign_ref: row.get(0),
        hop_index: usize::try_from(index).map_err(|_| GwbHopLedgerError::HopIndexOutOfRange)?,
        prior_receipt_sha256: row.get(2),
        world_before_sha256: row.get(3),
        frontier_sha256: row.get(4),
        selected_move_ref: row.get(5),
        investigation_kind_ref: row.get(6),
        producer_ref: row.get(7),
        source_revision_ref: row.get(8),
        evidence_digest_ref: row.get(9),
        review_ref: row.get(10),
        outcome_ref: row.get(11),
        residual_effect_ref: row.get(12),
        world_after_sha256: row.get(13),
        closed_residual_refs: row.get(14),
        opened_residual_refs: row.get(15),
        candidate_only: row.get(16),
        creates_semantic_authority: row.get(17),
        applicability_promoted: row.get(18),
        claim_truth_promoted: row.get(19),
        receipt_authority: "gwb_adaptive_runtime_review_only",
        receipt_sha256: row.get(20),
    }))
}

pub fn materialize_gwb_hop(
    config: &DatabaseConfig,
    input: &GwbHopLedgerInput,
) -> Result<GwbHopLedgerMaterializationReceipt, GwbHopLedgerError> {
    let prepared = gwb_hop_ledger_row(input)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(GWB_HOP_SCHEMA_SQL)?;

    let latest = tx.query_opt(
        "SELECT hop_index, receipt_sha256          FROM context.gwb_adaptive_hop_receipt          WHERE campaign_ref = $1            AND receipt_authority = 'gwb_adaptive_runtime_review_only'          ORDER BY hop_index DESC LIMIT 1 FOR UPDATE",
        &[&prepared.campaign_ref],
    )?;

    if let Some(latest) = latest {
        let latest_index_i64: i64 = latest.get(0);
        let latest_index =
            usize::try_from(latest_index_i64).map_err(|_| GwbHopLedgerError::HopIndexOutOfRange)?;
        let latest_receipt: String = latest.get(1);

        if prepared.hop_index == latest_index {
            if prepared.receipt_sha256 == latest_receipt {
                tx.commit()?;
                return Ok(GwbHopLedgerMaterializationReceipt {
                    campaign_ref: prepared.campaign_ref,
                    hop_index: prepared.hop_index,
                    receipt_sha256: prepared.receipt_sha256,
                    materialized: false,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                });
            }
            return Err(GwbHopLedgerError::ExistingHopConflict);
        }

        let expected = latest_index.saturating_add(1);
        if prepared.hop_index != expected {
            return Err(GwbHopLedgerError::NonConsecutiveHop {
                expected,
                observed: prepared.hop_index,
            });
        }
        if prepared.prior_receipt_sha256.as_deref() != Some(latest_receipt.as_str()) {
            return Err(GwbHopLedgerError::PriorReceiptMismatch {
                expected: latest_receipt,
                observed: prepared.prior_receipt_sha256.clone(),
            });
        }
    } else if prepared.hop_index != 0 {
        return Err(GwbHopLedgerError::FirstHopNotZero(prepared.hop_index));
    }

    let inserted = tx.execute(
        "INSERT INTO context.gwb_adaptive_hop_receipt (         receipt_sha256, campaign_ref, hop_index, prior_receipt_sha256, world_before_sha256,          frontier_sha256, selected_move_ref, investigation_kind_ref, producer_ref, source_revision_ref,          evidence_digest_ref, review_ref, outcome_ref, residual_effect_ref, world_after_sha256,          closed_residual_refs, opened_residual_refs, candidate_only, creates_semantic_authority,          applicability_promoted, claim_truth_promoted, receipt_authority) VALUES (         $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,TRUE,FALSE,FALSE,FALSE,$18)          ON CONFLICT DO NOTHING",
        &[
            &prepared.receipt_sha256,
            &prepared.campaign_ref,
            &(prepared.hop_index as i64),
            &prepared.prior_receipt_sha256,
            &prepared.world_before_sha256,
            &prepared.frontier_sha256,
            &prepared.selected_move_ref,
            &prepared.investigation_kind_ref,
            &prepared.producer_ref,
            &prepared.source_revision_ref,
            &prepared.evidence_digest_ref,
            &prepared.review_ref,
            &prepared.outcome_ref,
            &prepared.residual_effect_ref,
            &prepared.world_after_sha256,
            &prepared.closed_residual_refs,
            &prepared.opened_residual_refs,
            &prepared.receipt_authority,
        ],
    )? as usize;
    if inserted == 0 {
        return Err(GwbHopLedgerError::ExistingHopConflict);
    }
    tx.commit()?;

    Ok(GwbHopLedgerMaterializationReceipt {
        campaign_ref: prepared.campaign_ref,
        hop_index: prepared.hop_index,
        receipt_sha256: prepared.receipt_sha256,
        materialized: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
