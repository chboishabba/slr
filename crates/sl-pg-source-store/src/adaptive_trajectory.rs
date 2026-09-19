//! Durable adaptive trajectory receipts for restart-stable campaigns.
//!
//! The trajectory table records which current-world/current-frontier selection
//! actually committed. It is runtime provenance only: no receipt here creates
//! legal authority, applicability, claim truth, or review authority.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

const ADAPTIVE_TRAJECTORY_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.adaptive_trajectory_receipt (
  receipt_sha256 TEXT PRIMARY KEY,
  campaign_ref TEXT NOT NULL,
  cycle_index BIGINT NOT NULL CHECK (cycle_index >= 0),
  world_digest TEXT NOT NULL,
  frontier_digest TEXT NOT NULL,
  selected_residual_ref TEXT NOT NULL,
  selected_move_ref TEXT NOT NULL,
  selected_producer_lane_ref TEXT NOT NULL,
  prior_commit_ref TEXT,
  commit_ref TEXT NOT NULL,
  review_or_payment_ref TEXT NOT NULL,
  world_delta_ref TEXT NOT NULL,
  selection_origin TEXT NOT NULL CHECK (selection_origin = 'post-commit-current-world-diagnosis'),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_authority TEXT NOT NULL CHECK (receipt_authority = 'adaptive_trajectory_runtime_only'),
  UNIQUE (campaign_ref, cycle_index),
  UNIQUE (campaign_ref, commit_ref)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveTrajectoryInput {
    pub campaign_ref: String,
    pub cycle_index: usize,
    pub world_digest: String,
    pub frontier_digest: String,
    pub selected_residual_ref: String,
    pub selected_move_ref: String,
    pub selected_producer_lane_ref: String,
    pub prior_commit_ref: Option<String>,
    pub commit_ref: String,
    pub review_or_payment_ref: String,
    pub world_delta_ref: String,
    pub selection_origin: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveTrajectoryRow {
    pub campaign_ref: String,
    pub cycle_index: usize,
    pub world_digest: String,
    pub frontier_digest: String,
    pub selected_residual_ref: String,
    pub selected_move_ref: String,
    pub selected_producer_lane_ref: String,
    pub prior_commit_ref: Option<String>,
    pub commit_ref: String,
    pub review_or_payment_ref: String,
    pub world_delta_ref: String,
    pub selection_origin: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_authority: &'static str,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveTrajectoryMaterializationReceipt {
    pub campaign_ref: String,
    pub cycle_index: usize,
    pub receipt_sha256: String,
    pub materialized: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdaptiveTrajectoryError {
    #[error("adaptive trajectory coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("adaptive trajectory may not promote authority/applicability/truth")]
    TrajectoryMayNotPromote,
    #[error("adaptive trajectory must remain candidate-only")]
    TrajectoryMustRemainCandidateOnly,
    #[error("adaptive trajectory selection origin must be post-commit current-world diagnosis")]
    InvalidSelectionOrigin,
    #[error("cycle zero may not name a prior commit")]
    UnexpectedPriorCommitAtCycleZero,
    #[error("non-zero adaptive cycle must name its prior committed transition")]
    MissingPriorCommit,
    #[error("adaptive cycle index exceeds PostgreSQL BIGINT range")]
    CycleIndexOutOfRange,
    #[error("adaptive trajectory must extend latest cycle: expected {expected}, observed {observed}")]
    NonConsecutiveCycle { expected: usize, observed: usize },
    #[error("adaptive trajectory prior commit mismatch: expected {expected}, observed {observed:?}")]
    PriorCommitMismatch {
        expected: String,
        observed: Option<String>,
    },
    #[error("first persisted adaptive trajectory cycle must be zero, observed {0}")]
    FirstCycleNotZero(usize),
    #[error("existing adaptive trajectory row conflicts with supplied cycle")]
    ExistingCycleConflict,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for AdaptiveTrajectoryError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hash_field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value.as_bytes());
}

pub fn adaptive_trajectory_row(
    input: &AdaptiveTrajectoryInput,
) -> Result<AdaptiveTrajectoryRow, AdaptiveTrajectoryError> {
    for (name, value) in [
        ("campaign_ref", input.campaign_ref.as_str()),
        ("world_digest", input.world_digest.as_str()),
        ("frontier_digest", input.frontier_digest.as_str()),
        ("selected_residual_ref", input.selected_residual_ref.as_str()),
        ("selected_move_ref", input.selected_move_ref.as_str()),
        (
            "selected_producer_lane_ref",
            input.selected_producer_lane_ref.as_str(),
        ),
        ("commit_ref", input.commit_ref.as_str()),
        ("review_or_payment_ref", input.review_or_payment_ref.as_str()),
        ("world_delta_ref", input.world_delta_ref.as_str()),
        ("selection_origin", input.selection_origin.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AdaptiveTrajectoryError::EmptyCoordinate(name));
        }
    }
    if !input.candidate_only {
        return Err(AdaptiveTrajectoryError::TrajectoryMustRemainCandidateOnly);
    }
    if input.creates_semantic_authority
        || input.applicability_promoted
        || input.claim_truth_promoted
    {
        return Err(AdaptiveTrajectoryError::TrajectoryMayNotPromote);
    }
    if input.selection_origin != "post-commit-current-world-diagnosis" {
        return Err(AdaptiveTrajectoryError::InvalidSelectionOrigin);
    }
    if input.cycle_index == 0 && input.prior_commit_ref.is_some() {
        return Err(AdaptiveTrajectoryError::UnexpectedPriorCommitAtCycleZero);
    }
    if input.cycle_index > 0
        && input
            .prior_commit_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AdaptiveTrajectoryError::MissingPriorCommit);
    }
    if input.cycle_index > i64::MAX as usize {
        return Err(AdaptiveTrajectoryError::CycleIndexOutOfRange);
    }

    let mut hasher = Sha256::new();
    for value in [
        "mabo-adaptive-trajectory:v1",
        input.campaign_ref.as_str(),
        &input.cycle_index.to_string(),
        input.world_digest.as_str(),
        input.frontier_digest.as_str(),
        input.selected_residual_ref.as_str(),
        input.selected_move_ref.as_str(),
        input.selected_producer_lane_ref.as_str(),
        input.prior_commit_ref.as_deref().unwrap_or(""),
        input.commit_ref.as_str(),
        input.review_or_payment_ref.as_str(),
        input.world_delta_ref.as_str(),
        input.selection_origin.as_str(),
        "adaptive_trajectory_runtime_only",
    ] {
        hash_field(&mut hasher, value);
    }

    Ok(AdaptiveTrajectoryRow {
        campaign_ref: input.campaign_ref.clone(),
        cycle_index: input.cycle_index,
        world_digest: input.world_digest.clone(),
        frontier_digest: input.frontier_digest.clone(),
        selected_residual_ref: input.selected_residual_ref.clone(),
        selected_move_ref: input.selected_move_ref.clone(),
        selected_producer_lane_ref: input.selected_producer_lane_ref.clone(),
        prior_commit_ref: input.prior_commit_ref.clone(),
        commit_ref: input.commit_ref.clone(),
        review_or_payment_ref: input.review_or_payment_ref.clone(),
        world_delta_ref: input.world_delta_ref.clone(),
        selection_origin: input.selection_origin.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "adaptive_trajectory_runtime_only",
        receipt_sha256: hex_digest(&hasher.finalize()),
    })
}

pub fn load_latest_adaptive_trajectory(
    config: &DatabaseConfig,
    campaign_ref: &str,
) -> Result<Option<AdaptiveTrajectoryRow>, AdaptiveTrajectoryError> {
    if campaign_ref.trim().is_empty() {
        return Err(AdaptiveTrajectoryError::EmptyCoordinate("campaign_ref"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(ADAPTIVE_TRAJECTORY_SCHEMA_SQL)?;
    let row = client.query_opt(
        "SELECT campaign_ref, cycle_index, world_digest, frontier_digest,                 selected_residual_ref, selected_move_ref, selected_producer_lane_ref,                 prior_commit_ref, commit_ref, review_or_payment_ref, world_delta_ref,                 selection_origin, candidate_only, creates_semantic_authority,                 applicability_promoted, claim_truth_promoted, receipt_sha256          FROM context.adaptive_trajectory_receipt          WHERE campaign_ref = $1          ORDER BY cycle_index DESC LIMIT 1",
        &[&campaign_ref],
    )?;
    let Some(row) = row else {
        return Ok(None);
    };
    let cycle_index_i64: i64 = row.get(1);
    Ok(Some(AdaptiveTrajectoryRow {
        campaign_ref: row.get(0),
        cycle_index: usize::try_from(cycle_index_i64)
            .map_err(|_| AdaptiveTrajectoryError::CycleIndexOutOfRange)?,
        world_digest: row.get(2),
        frontier_digest: row.get(3),
        selected_residual_ref: row.get(4),
        selected_move_ref: row.get(5),
        selected_producer_lane_ref: row.get(6),
        prior_commit_ref: row.get(7),
        commit_ref: row.get(8),
        review_or_payment_ref: row.get(9),
        world_delta_ref: row.get(10),
        selection_origin: row.get(11),
        candidate_only: row.get(12),
        creates_semantic_authority: row.get(13),
        applicability_promoted: row.get(14),
        claim_truth_promoted: row.get(15),
        receipt_authority: "adaptive_trajectory_runtime_only",
        receipt_sha256: row.get(16),
    }))
}

pub fn materialize_adaptive_trajectory(
    config: &DatabaseConfig,
    input: &AdaptiveTrajectoryInput,
) -> Result<AdaptiveTrajectoryMaterializationReceipt, AdaptiveTrajectoryError> {
    let prepared = adaptive_trajectory_row(input)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(ADAPTIVE_TRAJECTORY_SCHEMA_SQL)?;

    let latest = tx.query_opt(
        "SELECT cycle_index, commit_ref, receipt_sha256          FROM context.adaptive_trajectory_receipt          WHERE campaign_ref = $1          ORDER BY cycle_index DESC LIMIT 1 FOR UPDATE",
        &[&prepared.campaign_ref],
    )?;

    if let Some(latest) = latest {
        let latest_index_i64: i64 = latest.get(0);
        let latest_index = usize::try_from(latest_index_i64)
            .map_err(|_| AdaptiveTrajectoryError::CycleIndexOutOfRange)?;
        let latest_commit: String = latest.get(1);
        let latest_hash: String = latest.get(2);

        if prepared.cycle_index == latest_index {
            if prepared.receipt_sha256 == latest_hash {
                tx.commit()?;
                return Ok(AdaptiveTrajectoryMaterializationReceipt {
                    campaign_ref: prepared.campaign_ref,
                    cycle_index: prepared.cycle_index,
                    receipt_sha256: prepared.receipt_sha256,
                    materialized: false,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                });
            }
            return Err(AdaptiveTrajectoryError::ExistingCycleConflict);
        }

        let expected = latest_index.saturating_add(1);
        if prepared.cycle_index != expected {
            return Err(AdaptiveTrajectoryError::NonConsecutiveCycle {
                expected,
                observed: prepared.cycle_index,
            });
        }
        if prepared.prior_commit_ref.as_deref() != Some(latest_commit.as_str()) {
            return Err(AdaptiveTrajectoryError::PriorCommitMismatch {
                expected: latest_commit,
                observed: prepared.prior_commit_ref.clone(),
            });
        }
    } else if prepared.cycle_index != 0 {
        return Err(AdaptiveTrajectoryError::FirstCycleNotZero(
            prepared.cycle_index,
        ));
    }

    let inserted = tx.execute(
        "INSERT INTO context.adaptive_trajectory_receipt (         receipt_sha256, campaign_ref, cycle_index, world_digest, frontier_digest,          selected_residual_ref, selected_move_ref, selected_producer_lane_ref,          prior_commit_ref, commit_ref, review_or_payment_ref, world_delta_ref,          selection_origin, candidate_only, creates_semantic_authority,          applicability_promoted, claim_truth_promoted, receipt_authority) VALUES (         $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,TRUE,FALSE,FALSE,FALSE,$14)          ON CONFLICT DO NOTHING",
        &[
            &prepared.receipt_sha256,
            &prepared.campaign_ref,
            &(prepared.cycle_index as i64),
            &prepared.world_digest,
            &prepared.frontier_digest,
            &prepared.selected_residual_ref,
            &prepared.selected_move_ref,
            &prepared.selected_producer_lane_ref,
            &prepared.prior_commit_ref,
            &prepared.commit_ref,
            &prepared.review_or_payment_ref,
            &prepared.world_delta_ref,
            &prepared.selection_origin,
            &prepared.receipt_authority,
        ],
    )? as usize;

    if inserted == 0 {
        return Err(AdaptiveTrajectoryError::ExistingCycleConflict);
    }
    tx.commit()?;

    Ok(AdaptiveTrajectoryMaterializationReceipt {
        campaign_ref: prepared.campaign_ref,
        cycle_index: prepared.cycle_index,
        receipt_sha256: prepared.receipt_sha256,
        materialized: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
