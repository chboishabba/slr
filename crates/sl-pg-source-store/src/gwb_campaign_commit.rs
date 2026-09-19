//! Atomic GWB campaign transition: reviewed residual state + trajectory receipt.
//!
//! A hop is not committed unless both the ambiguity-state mutation and the
//! durable trajectory row commit in the same PostgreSQL transaction.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::gwb_ambiguity_state::{
    gwb_ambiguity_state_row, GwbAmbiguityStateError, GwbAmbiguityStateInput,
    GWB_AMBIGUITY_SCHEMA_SQL,
};
use crate::gwb_hop_ledger::{
    gwb_hop_ledger_row, GwbHopLedgerError, GwbHopLedgerInput, GWB_HOP_SCHEMA_SQL,
};
use crate::DatabaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbCampaignCommitInput {
    pub hop: GwbHopLedgerInput,
    pub close_residual_refs: Vec<String>,
    pub open_residuals: Vec<GwbAmbiguityStateInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbCampaignCommitReceipt {
    pub campaign_ref: String,
    pub hop_index: usize,
    pub hop_receipt_sha256: String,
    pub residuals_closed: usize,
    pub residuals_opened: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum GwbCampaignCommitError {
    #[error("invalid hop receipt: {0}")]
    Hop(#[from] GwbHopLedgerError),
    #[error("invalid ambiguity residual: {0}")]
    Ambiguity(#[from] GwbAmbiguityStateError),
    #[error("opened residual campaign does not match hop campaign")]
    CampaignMismatch,
    #[error("opened residual hop does not match committed hop")]
    OpenedHopMismatch,
    #[error("reviewed close references unknown/open-mismatch residual: {0}")]
    UnknownCloseResidual(String),
    #[error("GWB hop index is not consecutive: expected {expected}, observed {observed}")]
    NonConsecutiveHop { expected: usize, observed: usize },
    #[error("GWB prior receipt mismatch: expected {expected}, observed {observed:?}")]
    PriorReceiptMismatch {
        expected: String,
        observed: Option<String>,
    },
    #[error("first persisted GWB hop must be zero, observed {0}")]
    FirstHopNotZero(usize),
    #[error("existing GWB hop conflicts with supplied receipt")]
    ExistingHopConflict,
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

pub fn materialize_gwb_campaign_commit(
    config: &DatabaseConfig,
    input: &GwbCampaignCommitInput,
) -> Result<GwbCampaignCommitReceipt, GwbCampaignCommitError> {
    let hop = gwb_hop_ledger_row(&input.hop)?;
    let mut opened = input
        .open_residuals
        .iter()
        .map(gwb_ambiguity_state_row)
        .collect::<Result<Vec<_>, _>>()?;
    for row in &opened {
        if row.campaign_ref != hop.campaign_ref {
            return Err(GwbCampaignCommitError::CampaignMismatch);
        }
        if row.opened_by_hop != Some(hop.hop_index) {
            return Err(GwbCampaignCommitError::OpenedHopMismatch);
        }
    }
    opened.sort_by(|left, right| left.residual_ref.cmp(&right.residual_ref));
    opened.dedup_by(|left, right| left.residual_ref == right.residual_ref);

    let mut closes = input.close_residual_refs.clone();
    closes.sort();
    closes.dedup();

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(GWB_AMBIGUITY_SCHEMA_SQL)?;
    tx.batch_execute(GWB_HOP_SCHEMA_SQL)?;

    let latest = tx.query_opt(
        "SELECT hop_index, receipt_sha256 FROM context.gwb_adaptive_hop_receipt          WHERE campaign_ref=$1 AND receipt_authority='gwb_adaptive_runtime_review_only'          ORDER BY hop_index DESC LIMIT 1 FOR UPDATE",
        &[&hop.campaign_ref],
    )?;

    if let Some(latest) = latest {
        let latest_i64: i64 = latest.get(0);
        let latest_index =
            usize::try_from(latest_i64).map_err(|_| GwbHopLedgerError::HopIndexOutOfRange)?;
        let latest_receipt: String = latest.get(1);

        if hop.hop_index == latest_index {
            if hop.receipt_sha256 == latest_receipt {
                tx.commit()?;
                return Ok(GwbCampaignCommitReceipt {
                    campaign_ref: hop.campaign_ref,
                    hop_index: hop.hop_index,
                    hop_receipt_sha256: hop.receipt_sha256,
                    residuals_closed: 0,
                    residuals_opened: 0,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                });
            }
            return Err(GwbCampaignCommitError::ExistingHopConflict);
        }
        let expected = latest_index.saturating_add(1);
        if hop.hop_index != expected {
            return Err(GwbCampaignCommitError::NonConsecutiveHop {
                expected,
                observed: hop.hop_index,
            });
        }
        if hop.prior_receipt_sha256.as_deref() != Some(latest_receipt.as_str()) {
            return Err(GwbCampaignCommitError::PriorReceiptMismatch {
                expected: latest_receipt,
                observed: hop.prior_receipt_sha256.clone(),
            });
        }
    } else if hop.hop_index != 0 {
        return Err(GwbCampaignCommitError::FirstHopNotZero(hop.hop_index));
    }

    let hop_i64 = i64::try_from(hop.hop_index)
        .map_err(|_| GwbHopLedgerError::HopIndexOutOfRange)?;
    let mut residuals_closed = 0usize;
    for residual_ref in closes {
        let updated = tx.execute(
            "UPDATE context.gwb_ambiguity_residual              SET status_ref='closed-reviewed',closed_by_hop=$1,close_review_ref=$2              WHERE campaign_ref=$3 AND residual_ref=$4 AND status_ref='open'",
            &[&hop_i64, &hop.review_ref, &hop.campaign_ref, &residual_ref],
        )? as usize;
        if updated == 0 {
            return Err(GwbCampaignCommitError::UnknownCloseResidual(residual_ref));
        }
        residuals_closed = residuals_closed.saturating_add(updated);
    }

    let mut residuals_opened = 0usize;
    for row in &opened {
        let opened_by_hop = row
            .opened_by_hop
            .map(|value| i64::try_from(value).map_err(|_| GwbHopLedgerError::HopIndexOutOfRange))
            .transpose()?;
        residuals_opened += tx.execute(
            "INSERT INTO context.gwb_ambiguity_residual (             campaign_ref,residual_ref,subject_ref,proposition_ref,kind_ref,root_qid,salience,             dependency_refs,status_ref,opened_by_hop,candidate_only,creates_semantic_authority,             applicability_promoted,claim_truth_promoted) VALUES (             $1,$2,$3,$4,$5,$6,$7,$8,'open',$9,TRUE,FALSE,FALSE,FALSE)              ON CONFLICT (campaign_ref,residual_ref) DO NOTHING",
            &[
                &row.campaign_ref,
                &row.residual_ref,
                &row.subject_ref,
                &row.proposition_ref,
                &row.kind_ref,
                &row.root_qid,
                &(row.salience as i64),
                &row.dependency_refs,
                &opened_by_hop,
            ],
        )? as usize;
    }

    let inserted = tx.execute(
        "INSERT INTO context.gwb_adaptive_hop_receipt (         receipt_sha256,campaign_ref,hop_index,prior_receipt_sha256,world_before_sha256,         frontier_sha256,selected_move_ref,investigation_kind_ref,source_revision_ref,         evidence_digest_ref,review_ref,outcome_ref,residual_effect_ref,world_after_sha256,         closed_residual_refs,opened_residual_refs,candidate_only,creates_semantic_authority,         applicability_promoted,claim_truth_promoted,receipt_authority) VALUES (         $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,TRUE,FALSE,FALSE,FALSE,$17)",
        &[
            &hop.receipt_sha256,
            &hop.campaign_ref,
            &hop_i64,
            &hop.prior_receipt_sha256,
            &hop.world_before_sha256,
            &hop.frontier_sha256,
            &hop.selected_move_ref,
            &hop.investigation_kind_ref,
            &hop.source_revision_ref,
            &hop.evidence_digest_ref,
            &hop.review_ref,
            &hop.outcome_ref,
            &hop.residual_effect_ref,
            &hop.world_after_sha256,
            &hop.closed_residual_refs,
            &hop.opened_residual_refs,
            &hop.receipt_authority,
        ],
    )? as usize;
    if inserted != 1 {
        return Err(GwbCampaignCommitError::ExistingHopConflict);
    }
    tx.commit()?;

    Ok(GwbCampaignCommitReceipt {
        campaign_ref: hop.campaign_ref,
        hop_index: hop.hop_index,
        hop_receipt_sha256: hop.receipt_sha256,
        residuals_closed,
        residuals_opened,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
