//! Durable receipts for exact reviewed bounded-context source expansion.
//!
//! A source-expansion receipt says only that one exact QID manifestation was
//! parsed and its bounded outgoing context was explicitly reviewed. It neither
//! creates identity novelty nor pays any legal/proof claim.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{context_federation::CONTEXT_RECEIPT_SCHEMA_SQL, DatabaseConfig};

pub(crate) const REVIEWED_SOURCE_EXPANSION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.reviewed_source_expansion_receipt (
  receipt_sha256 TEXT PRIMARY KEY,
  source_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  review_ref TEXT NOT NULL,
  bounded_candidate_count BIGINT NOT NULL CHECK (bounded_candidate_count >= 0),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  counts_as_novel_identity BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT counts_as_novel_identity),
  pays_claim_residual BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT pays_claim_residual),
  receipt_authority TEXT NOT NULL CHECK (receipt_authority = 'reviewed_bounded_context_expansion_only'),
  UNIQUE (source_ref, source_revision_ref, review_ref)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedSourceExpansionInput {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub review_ref: String,
    pub bounded_candidate_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedSourceExpansionRow {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub review_ref: String,
    pub bounded_candidate_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub counts_as_novel_identity: bool,
    pub pays_claim_residual: bool,
    pub receipt_authority: &'static str,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedSourceExpansionMaterializationReceipt {
    pub attempted_count: usize,
    pub materialized_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub counts_as_novel_identity: bool,
    pub pays_claim_residual: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReviewedSourceExpansionError {
    #[error("source expansion must remain candidate-only")]
    ExpansionMustRemainCandidateOnly,
    #[error("source expansion may not promote authority/applicability/truth")]
    ExpansionMayNotPromote,
    #[error("source expansion coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("source revision {source_revision_ref} does not pin source {source_ref}")]
    SourceRevisionMismatch {
        source_ref: String,
        source_revision_ref: String,
    },
    #[error("bounded candidate count exceeds PostgreSQL BIGINT range")]
    CandidateCountOutOfRange,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for ReviewedSourceExpansionError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn exact_revision_matches_source(source_ref: &str, source_revision_ref: &str) -> bool {
    let fields = source_revision_ref.split(':').collect::<Vec<_>>();
    fields.len() == 4
        && fields[0] == "wikidata"
        && fields[1] == source_ref
        && fields[2] == "oldid"
        && fields[3]
            .parse::<u64>()
            .is_ok_and(|revision_id| revision_id > 0)
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn reviewed_source_expansion_row(
    input: &ReviewedSourceExpansionInput,
) -> Result<ReviewedSourceExpansionRow, ReviewedSourceExpansionError> {
    if !input.candidate_only {
        return Err(ReviewedSourceExpansionError::ExpansionMustRemainCandidateOnly);
    }
    if input.creates_semantic_authority || input.applicability_promoted || input.claim_truth_promoted {
        return Err(ReviewedSourceExpansionError::ExpansionMayNotPromote);
    }
    for (name, value) in [
        ("source_ref", input.source_ref.as_str()),
        ("source_revision_ref", input.source_revision_ref.as_str()),
        ("review_ref", input.review_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ReviewedSourceExpansionError::EmptyCoordinate(name));
        }
    }
    if !exact_revision_matches_source(&input.source_ref, &input.source_revision_ref) {
        return Err(ReviewedSourceExpansionError::SourceRevisionMismatch {
            source_ref: input.source_ref.clone(),
            source_revision_ref: input.source_revision_ref.clone(),
        });
    }
    if input.bounded_candidate_count > i64::MAX as usize {
        return Err(ReviewedSourceExpansionError::CandidateCountOutOfRange);
    }

    let mut hasher = Sha256::new();
    for value in [
        "reviewed-bounded-context-expansion:v1",
        input.source_ref.as_str(),
        input.source_revision_ref.as_str(),
        input.review_ref.as_str(),
        "reviewed_bounded_context_expansion_only",
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    hasher.update((input.bounded_candidate_count as u64).to_le_bytes());

    Ok(ReviewedSourceExpansionRow {
        source_ref: input.source_ref.clone(),
        source_revision_ref: input.source_revision_ref.clone(),
        review_ref: input.review_ref.clone(),
        bounded_candidate_count: input.bounded_candidate_count,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        counts_as_novel_identity: false,
        pays_claim_residual: false,
        receipt_authority: "reviewed_bounded_context_expansion_only",
        receipt_sha256: hex_digest(&hasher.finalize()),
    })
}

pub fn materialize_reviewed_source_expansions(
    config: &DatabaseConfig,
    inputs: &[ReviewedSourceExpansionInput],
) -> Result<ReviewedSourceExpansionMaterializationReceipt, ReviewedSourceExpansionError> {
    let rows = inputs
        .iter()
        .map(reviewed_source_expansion_row)
        .collect::<Result<Vec<_>, _>>()?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(REVIEWED_SOURCE_EXPANSION_SCHEMA_SQL)?;
    let mut materialized_count = 0usize;
    for row in &rows {
        materialized_count += tx.execute(
            "INSERT INTO context.reviewed_source_expansion_receipt (\
             receipt_sha256, source_ref, source_revision_ref, review_ref, bounded_candidate_count, \
             candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted, \
             counts_as_novel_identity, pays_claim_residual, receipt_authority) VALUES (\
             $1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE,$6) ON CONFLICT DO NOTHING",
            &[
                &row.receipt_sha256,
                &row.source_ref,
                &row.source_revision_ref,
                &row.review_ref,
                &(row.bounded_candidate_count as i64),
                &row.receipt_authority,
            ],
        )? as usize;
    }
    tx.commit()?;

    Ok(ReviewedSourceExpansionMaterializationReceipt {
        attempted_count: rows.len(),
        materialized_count,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        counts_as_novel_identity: false,
        pays_claim_residual: false,
    })
}

/// Sources with an already-reviewed outgoing relation are treated as legacy
/// expanded sources. Explicit source-expansion receipts additionally close the
/// zero-bounded-candidate case and make the expansion state restart-stable.
pub fn load_reviewed_context_expansion_sources(
    config: &DatabaseConfig,
) -> Result<BTreeSet<String>, ReviewedSourceExpansionError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CONTEXT_RECEIPT_SCHEMA_SQL)?;
    client.batch_execute(REVIEWED_SOURCE_EXPANSION_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT source_ref FROM context.reviewed_source_expansion_receipt \
         WHERE candidate_only = TRUE \
           AND creates_semantic_authority = FALSE \
           AND applicability_promoted = FALSE \
           AND claim_truth_promoted = FALSE \
           AND counts_as_novel_identity = FALSE \
           AND pays_claim_residual = FALSE \
         UNION \
         SELECT relation.left_ref AS source_ref \
         FROM algebra.relation AS relation \
         JOIN context.reviewed_relation_receipt AS receipt \
           ON receipt.relation_ref = relation.relation_ref \
         WHERE receipt.candidate_only = TRUE \
           AND receipt.creates_semantic_authority = FALSE \
           AND receipt.applicability_promoted = FALSE \
           AND receipt.claim_truth_promoted = FALSE \
           AND relation.relation_type_ref LIKE 'context:wikidata:%' \
         ORDER BY source_ref",
        &[],
    )?;
    Ok(rows
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect())
}
