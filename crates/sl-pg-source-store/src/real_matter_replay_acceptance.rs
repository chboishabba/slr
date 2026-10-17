//! Durable acceptance receipt for repeated replay-strict REAL-MATTER materialisation.
//!
//! This does not create or promote a controversy. It runs the existing strict
//! materialiser repeatedly, requires identical receipts, and persists the exact
//! refs as execution evidence. The same owner can freeze Smith, Mabo, or later
//! real matters without case-specific storage semantics.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    install_legal_controversy_schema, materialize_real_matter_controversy_strict,
    DatabaseConfig, RealMatterControversyDraft, RealMatterControversyMaterializationError,
    RealMatterControversyReceipt,
};

pub const REAL_MATTER_REPLAY_ACCEPTANCE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS acceptance;

CREATE TABLE IF NOT EXISTS acceptance.real_matter_replay_receipt (
  acceptance_ref TEXT PRIMARY KEY,
  fixture_ref TEXT NOT NULL,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref),
  applicant_reviewed_evidence_ref TEXT NOT NULL,
  respondent_reviewed_evidence_ref TEXT NOT NULL,
  applicant_fibre_ref TEXT NOT NULL,
  respondent_fibre_ref TEXT NOT NULL,
  response_ref TEXT NOT NULL,
  residual_ref TEXT NOT NULL,
  obligation_ref TEXT NOT NULL,
  reverse_ref TEXT NOT NULL,
  applicant_normative_order_ref TEXT NOT NULL,
  respondent_normative_order_ref TEXT NOT NULL,
  replay_count INTEGER NOT NULL CHECK (replay_count >= 2),
  receipt_fingerprint_sha256 TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  replay_stable BOOLEAN NOT NULL CHECK (replay_stable),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE(fixture_ref,matter_ref,controversy_ref)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealMatterReplayAcceptanceReceipt {
    pub acceptance_ref: String,
    pub fixture_ref: String,
    pub matter_ref: String,
    pub controversy_ref: String,
    pub applicant_reviewed_evidence_ref: String,
    pub respondent_reviewed_evidence_ref: String,
    pub applicant_fibre_ref: String,
    pub respondent_fibre_ref: String,
    pub response_ref: String,
    pub residual_ref: String,
    pub obligation_ref: String,
    pub reverse_ref: String,
    pub applicant_normative_order_ref: String,
    pub respondent_normative_order_ref: String,
    pub replay_count: i32,
    pub receipt_fingerprint_sha256: String,
    pub candidate_only: bool,
    pub replay_stable: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum RealMatterReplayAcceptanceError {
    #[error("real-Matter strict materialisation failed: {0}")]
    Materialization(#[from] RealMatterControversyMaterializationError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("fixture_ref is empty")]
    EmptyFixtureRef,
    #[error("replay count must be at least two")]
    InsufficientReplayCount,
    #[error("replay count exceeds PostgreSQL INTEGER")]
    ReplayCountOverflow,
    #[error("repeated strict materialisation produced different receipts")]
    ReplayMismatch,
    #[error("persisted replay acceptance receipt conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("replay acceptance receipt not found: {0}")]
    NotFound(String),
}

#[must_use]
pub fn replay_count_satisfies(stored: i32, requested: i32) -> bool {
    stored >= requested && requested >= 2
}

pub fn checked_replay_count(
    replay_count: usize,
) -> Result<i32, RealMatterReplayAcceptanceError> {
    if replay_count < 2 {
        return Err(RealMatterReplayAcceptanceError::InsufficientReplayCount);
    }
    i32::try_from(replay_count).map_err(|_| RealMatterReplayAcceptanceError::ReplayCountOverflow)
}

fn fingerprint(receipt: &RealMatterControversyReceipt) -> String {
    let mut hasher = Sha256::new();
    for value in [
        receipt.matter_ref.as_str(),
        receipt.controversy_ref.as_str(),
        receipt.applicant_fibre_ref.as_str(),
        receipt.respondent_fibre_ref.as_str(),
        receipt.response_ref.as_str(),
        receipt.residual_ref.as_str(),
        receipt.obligation_ref.as_str(),
        receipt.reverse_ref.as_str(),
        receipt.applicant_normative_order_ref.as_str(),
        receipt.respondent_normative_order_ref.as_str(),
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn stable_acceptance_ref(fixture_ref: &str, fingerprint: &str) -> String {
    let mut hasher = Sha256::new();
    for value in [fixture_ref, fingerprint] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("real-matter-replay-acceptance:sha256:{:x}", hasher.finalize())
}

fn same_immutable_coordinates(
    left: &RealMatterReplayAcceptanceReceipt,
    right: &RealMatterReplayAcceptanceReceipt,
) -> bool {
    left.acceptance_ref == right.acceptance_ref
        && left.fixture_ref == right.fixture_ref
        && left.matter_ref == right.matter_ref
        && left.controversy_ref == right.controversy_ref
        && left.applicant_reviewed_evidence_ref == right.applicant_reviewed_evidence_ref
        && left.respondent_reviewed_evidence_ref == right.respondent_reviewed_evidence_ref
        && left.applicant_fibre_ref == right.applicant_fibre_ref
        && left.respondent_fibre_ref == right.respondent_fibre_ref
        && left.response_ref == right.response_ref
        && left.residual_ref == right.residual_ref
        && left.obligation_ref == right.obligation_ref
        && left.reverse_ref == right.reverse_ref
        && left.applicant_normative_order_ref == right.applicant_normative_order_ref
        && left.respondent_normative_order_ref == right.respondent_normative_order_ref
        && left.receipt_fingerprint_sha256 == right.receipt_fingerprint_sha256
        && left.candidate_only == right.candidate_only
        && left.replay_stable == right.replay_stable
        && left.creates_semantic_authority == right.creates_semantic_authority
        && left.creates_legal_authority == right.creates_legal_authority
        && left.applicability_promoted == right.applicability_promoted
        && left.claim_truth_promoted == right.claim_truth_promoted
}

pub fn install_real_matter_replay_acceptance_schema(
    config: &DatabaseConfig,
) -> Result<(), RealMatterReplayAcceptanceError> {
    install_legal_controversy_schema(config).map_err(|error| {
        RealMatterReplayAcceptanceError::Materialization(
            RealMatterControversyMaterializationError::Store(error),
        )
    })?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REAL_MATTER_REPLAY_ACCEPTANCE_SCHEMA_SQL)?;
    Ok(())
}

pub fn materialize_and_persist_real_matter_replay_acceptance(
    config: &DatabaseConfig,
    fixture_ref: &str,
    draft: &RealMatterControversyDraft,
    replay_count: usize,
) -> Result<RealMatterReplayAcceptanceReceipt, RealMatterReplayAcceptanceError> {
    if fixture_ref.trim().is_empty() {
        return Err(RealMatterReplayAcceptanceError::EmptyFixtureRef);
    }
    let replay_count = checked_replay_count(replay_count)?;
    install_real_matter_replay_acceptance_schema(config)?;

    let first = materialize_real_matter_controversy_strict(config, draft)?;
    for _ in 1..replay_count {
        let replayed = materialize_real_matter_controversy_strict(config, draft)?;
        if replayed != first {
            return Err(RealMatterReplayAcceptanceError::ReplayMismatch);
        }
    }
    let receipt_fingerprint_sha256 = fingerprint(&first);
    let acceptance_ref = stable_acceptance_ref(fixture_ref, &receipt_fingerprint_sha256);
    let expected = RealMatterReplayAcceptanceReceipt {
        acceptance_ref: acceptance_ref.clone(),
        fixture_ref: fixture_ref.to_owned(),
        matter_ref: first.matter_ref.clone(),
        controversy_ref: first.controversy_ref.clone(),
        applicant_reviewed_evidence_ref: draft.applicant_reviewed_evidence_ref.clone(),
        respondent_reviewed_evidence_ref: draft.respondent_reviewed_evidence_ref.clone(),
        applicant_fibre_ref: first.applicant_fibre_ref,
        respondent_fibre_ref: first.respondent_fibre_ref,
        response_ref: first.response_ref,
        residual_ref: first.residual_ref,
        obligation_ref: first.obligation_ref,
        reverse_ref: first.reverse_ref,
        applicant_normative_order_ref: first.applicant_normative_order_ref,
        respondent_normative_order_ref: first.respondent_normative_order_ref,
        replay_count,
        receipt_fingerprint_sha256,
        candidate_only: true,
        replay_stable: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO acceptance.real_matter_replay_receipt
           (acceptance_ref,fixture_ref,matter_ref,controversy_ref,
            applicant_reviewed_evidence_ref,respondent_reviewed_evidence_ref,
            applicant_fibre_ref,respondent_fibre_ref,response_ref,residual_ref,obligation_ref,
            reverse_ref,applicant_normative_order_ref,respondent_normative_order_ref,replay_count,
            receipt_fingerprint_sha256,candidate_only,replay_stable,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                  TRUE,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(acceptance_ref) DO UPDATE SET replay_count = GREATEST(
             acceptance.real_matter_replay_receipt.replay_count,
             EXCLUDED.replay_count
           )"#,
        &[
            &expected.acceptance_ref,
            &expected.fixture_ref,
            &expected.matter_ref,
            &expected.controversy_ref,
            &expected.applicant_reviewed_evidence_ref,
            &expected.respondent_reviewed_evidence_ref,
            &expected.applicant_fibre_ref,
            &expected.respondent_fibre_ref,
            &expected.response_ref,
            &expected.residual_ref,
            &expected.obligation_ref,
            &expected.reverse_ref,
            &expected.applicant_normative_order_ref,
            &expected.respondent_normative_order_ref,
            &expected.replay_count,
            &expected.receipt_fingerprint_sha256,
        ],
    )?;
    let reopened = load_real_matter_replay_acceptance(config, &acceptance_ref)?;
    if !same_immutable_coordinates(&reopened, &expected)
        || !replay_count_satisfies(reopened.replay_count, expected.replay_count)
    {
        return Err(RealMatterReplayAcceptanceError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_real_matter_replay_acceptance(
    config: &DatabaseConfig,
    acceptance_ref: &str,
) -> Result<RealMatterReplayAcceptanceReceipt, RealMatterReplayAcceptanceError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT fixture_ref,matter_ref,controversy_ref,applicant_reviewed_evidence_ref,
                      respondent_reviewed_evidence_ref,applicant_fibre_ref,respondent_fibre_ref,
                      response_ref,residual_ref,obligation_ref,reverse_ref,
                      applicant_normative_order_ref,respondent_normative_order_ref,replay_count,
                      receipt_fingerprint_sha256,candidate_only,replay_stable,
                      creates_semantic_authority,creates_legal_authority,applicability_promoted,
                      claim_truth_promoted
               FROM acceptance.real_matter_replay_receipt WHERE acceptance_ref=$1"#,
            &[&acceptance_ref],
        )?
        .ok_or_else(|| RealMatterReplayAcceptanceError::NotFound(acceptance_ref.to_owned()))?;
    let value = RealMatterReplayAcceptanceReceipt {
        acceptance_ref: acceptance_ref.to_owned(),
        fixture_ref: row.get(0),
        matter_ref: row.get(1),
        controversy_ref: row.get(2),
        applicant_reviewed_evidence_ref: row.get(3),
        respondent_reviewed_evidence_ref: row.get(4),
        applicant_fibre_ref: row.get(5),
        respondent_fibre_ref: row.get(6),
        response_ref: row.get(7),
        residual_ref: row.get(8),
        obligation_ref: row.get(9),
        reverse_ref: row.get(10),
        applicant_normative_order_ref: row.get(11),
        respondent_normative_order_ref: row.get(12),
        replay_count: row.get(13),
        receipt_fingerprint_sha256: row.get(14),
        candidate_only: row.get(15),
        replay_stable: row.get(16),
        creates_semantic_authority: row.get(17),
        creates_legal_authority: row.get(18),
        applicability_promoted: row.get(19),
        claim_truth_promoted: row.get(20),
    };
    if value.replay_count < 2
        || !value.candidate_only
        || !value.replay_stable
        || value.creates_semantic_authority
        || value.creates_legal_authority
        || value.applicability_promoted
        || value.claim_truth_promoted
    {
        return Err(RealMatterReplayAcceptanceError::ExistingRowConflict);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceptance_identity_is_fixture_plus_exact_controversy_receipt() {
        let receipt = RealMatterControversyReceipt {
            matter_ref: "matter".into(),
            controversy_ref: "controversy".into(),
            applicant_fibre_ref: "a".into(),
            respondent_fibre_ref: "b".into(),
            response_ref: "response".into(),
            residual_ref: "residual".into(),
            obligation_ref: "obligation".into(),
            reverse_ref: "reverse".into(),
            applicant_normative_order_ref: "order:a".into(),
            respondent_normative_order_ref: "order:b".into(),
            response_mode: crate::ResponseMode::AdmitOccurrenceDisputeCharacterisation,
            disagreement_kind: crate::DisagreementKind::LegalConsequence,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_legal_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        let fp = fingerprint(&receipt);
        assert_eq!(fp.len(), 64);
        assert!(stable_acceptance_ref("fixture:smith", &fp)
            .starts_with("real-matter-replay-acceptance:sha256:"));
    }
}
