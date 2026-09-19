//! Durable reviewed negative knowledge for adaptive proof-search.
//!
//! These receipts constrain one exact candidate move for one exact residual.
//! They do not satisfy that residual and they never promote semantic/legal
//! authority, applicability, or claim truth.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

const ADAPTIVE_NEGATIVE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.adaptive_negative_assessment_receipt (
  receipt_sha256 TEXT PRIMARY KEY,
  residual_ref TEXT NOT NULL,
  move_ref TEXT NOT NULL,
  kind_ref TEXT NOT NULL,
  assessment_ref TEXT NOT NULL,
  source_revision_ref TEXT,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  makes_move_inadmissible BOOLEAN NOT NULL CHECK (makes_move_inadmissible),
  satisfies_residual BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT satisfies_residual),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_authority TEXT NOT NULL CHECK (receipt_authority = 'reviewed_negative_search_constraint_only'),
  UNIQUE (residual_ref, move_ref, kind_ref, assessment_ref, source_revision_ref)
);
"#;

const ALLOWED_KINDS: &[&str] = &[
    "wrong-type",
    "duplicate",
    "irrelevant-to-residual",
    "failed-factors-through",
    "inadmissible",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveNegativeAssessmentInput {
    pub residual_ref: String,
    pub move_ref: String,
    pub kind_ref: String,
    pub assessment_ref: String,
    pub source_revision_ref: Option<String>,
    pub candidate_only: bool,
    pub makes_move_inadmissible: bool,
    pub satisfies_residual: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveNegativeAssessmentRow {
    pub residual_ref: String,
    pub move_ref: String,
    pub kind_ref: String,
    pub assessment_ref: String,
    pub source_revision_ref: Option<String>,
    pub candidate_only: bool,
    pub makes_move_inadmissible: bool,
    pub satisfies_residual: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_authority: &'static str,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdaptiveNegativeAssessmentMaterializationReceipt {
    pub attempted_count: usize,
    pub materialized_count: usize,
    pub candidate_only: bool,
    pub makes_move_inadmissible: bool,
    pub satisfies_residual: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdaptiveNegativeAssessmentError {
    #[error("negative assessment coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("negative assessment kind is invalid: {0}")]
    InvalidKind(String),
    #[error("negative assessment must remain candidate-only")]
    NegativeMustRemainCandidateOnly,
    #[error("negative assessment must make its exact move inadmissible")]
    NegativeMustConstrainMove,
    #[error("negative assessment may not satisfy the residual")]
    NegativeMayNotSatisfyResidual,
    #[error("negative assessment may not promote authority/applicability/truth")]
    NegativeMayNotPromote,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for AdaptiveNegativeAssessmentError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_kind(kind: &str) -> bool {
    ALLOWED_KINDS.contains(&kind)
}

pub fn adaptive_negative_assessment_row(
    input: &AdaptiveNegativeAssessmentInput,
) -> Result<AdaptiveNegativeAssessmentRow, AdaptiveNegativeAssessmentError> {
    for (name, value) in [
        ("residual_ref", input.residual_ref.as_str()),
        ("move_ref", input.move_ref.as_str()),
        ("kind_ref", input.kind_ref.as_str()),
        ("assessment_ref", input.assessment_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(AdaptiveNegativeAssessmentError::EmptyCoordinate(name));
        }
    }
    if input
        .source_revision_ref
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(AdaptiveNegativeAssessmentError::EmptyCoordinate(
            "source_revision_ref",
        ));
    }
    if !valid_kind(&input.kind_ref) {
        return Err(AdaptiveNegativeAssessmentError::InvalidKind(
            input.kind_ref.clone(),
        ));
    }
    if !input.candidate_only {
        return Err(AdaptiveNegativeAssessmentError::NegativeMustRemainCandidateOnly);
    }
    if !input.makes_move_inadmissible {
        return Err(AdaptiveNegativeAssessmentError::NegativeMustConstrainMove);
    }
    if input.satisfies_residual {
        return Err(AdaptiveNegativeAssessmentError::NegativeMayNotSatisfyResidual);
    }
    if input.creates_semantic_authority
        || input.applicability_promoted
        || input.claim_truth_promoted
    {
        return Err(AdaptiveNegativeAssessmentError::NegativeMayNotPromote);
    }

    let mut hasher = Sha256::new();
    for value in [
        "adaptive-negative-search-constraint:v1",
        input.residual_ref.as_str(),
        input.move_ref.as_str(),
        input.kind_ref.as_str(),
        input.assessment_ref.as_str(),
        input.source_revision_ref.as_deref().unwrap_or(""),
        "reviewed_negative_search_constraint_only",
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }

    Ok(AdaptiveNegativeAssessmentRow {
        residual_ref: input.residual_ref.clone(),
        move_ref: input.move_ref.clone(),
        kind_ref: input.kind_ref.clone(),
        assessment_ref: input.assessment_ref.clone(),
        source_revision_ref: input.source_revision_ref.clone(),
        candidate_only: true,
        makes_move_inadmissible: true,
        satisfies_residual: false,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "reviewed_negative_search_constraint_only",
        receipt_sha256: hex_digest(&hasher.finalize()),
    })
}

pub fn materialize_adaptive_negative_assessments(
    config: &DatabaseConfig,
    inputs: &[AdaptiveNegativeAssessmentInput],
) -> Result<AdaptiveNegativeAssessmentMaterializationReceipt, AdaptiveNegativeAssessmentError> {
    let rows = inputs
        .iter()
        .map(adaptive_negative_assessment_row)
        .collect::<Result<Vec<_>, _>>()?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(ADAPTIVE_NEGATIVE_SCHEMA_SQL)?;
    let mut materialized_count = 0usize;
    for row in &rows {
        materialized_count += tx.execute(
            "INSERT INTO context.adaptive_negative_assessment_receipt (             receipt_sha256, residual_ref, move_ref, kind_ref, assessment_ref, source_revision_ref,              candidate_only, makes_move_inadmissible, satisfies_residual, creates_semantic_authority,              applicability_promoted, claim_truth_promoted, receipt_authority) VALUES (             $1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,$7) ON CONFLICT DO NOTHING",
            &[
                &row.receipt_sha256,
                &row.residual_ref,
                &row.move_ref,
                &row.kind_ref,
                &row.assessment_ref,
                &row.source_revision_ref,
                &row.receipt_authority,
            ],
        )? as usize;
    }
    tx.commit()?;

    Ok(AdaptiveNegativeAssessmentMaterializationReceipt {
        attempted_count: rows.len(),
        materialized_count,
        candidate_only: true,
        makes_move_inadmissible: true,
        satisfies_residual: false,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn load_adaptive_negative_assessments(
    config: &DatabaseConfig,
) -> Result<Vec<AdaptiveNegativeAssessmentRow>, AdaptiveNegativeAssessmentError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(ADAPTIVE_NEGATIVE_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT residual_ref, move_ref, kind_ref, assessment_ref, source_revision_ref,                 candidate_only, makes_move_inadmissible, satisfies_residual,                 creates_semantic_authority, applicability_promoted, claim_truth_promoted,                 receipt_authority, receipt_sha256          FROM context.adaptive_negative_assessment_receipt          WHERE candidate_only = TRUE            AND makes_move_inadmissible = TRUE            AND satisfies_residual = FALSE            AND creates_semantic_authority = FALSE            AND applicability_promoted = FALSE            AND claim_truth_promoted = FALSE          ORDER BY residual_ref, move_ref, kind_ref, assessment_ref, source_revision_ref",
        &[],
    )?;

    Ok(rows
        .into_iter()
        .map(|row| AdaptiveNegativeAssessmentRow {
            residual_ref: row.get(0),
            move_ref: row.get(1),
            kind_ref: row.get(2),
            assessment_ref: row.get(3),
            source_revision_ref: row.get(4),
            candidate_only: row.get(5),
            makes_move_inadmissible: row.get(6),
            satisfies_residual: row.get(7),
            creates_semantic_authority: row.get(8),
            applicability_promoted: row.get(9),
            claim_truth_promoted: row.get(10),
            receipt_authority: "reviewed_negative_search_constraint_only",
            receipt_sha256: row.get(12),
        })
        .collect())
}
