//! Revision-attributed candidate context edges for the latent-world walker.
//!
//! The materialiser deliberately owns only durable candidate context. It does
//! not fetch providers, infer legal authority, or pay a reader/proof claim.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub(crate) const CONTEXT_RECEIPT_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.reviewed_relation_receipt (
  relation_ref TEXT NOT NULL REFERENCES algebra.relation(relation_ref),
  source_family_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  receipt_sha256 BYTEA NOT NULL UNIQUE,
  PRIMARY KEY (relation_ref, source_revision_ref)
)
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceFamily {
    Wikidata,
    Wikipedia,
    Oalc,
}

impl SourceFamily {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wikidata => "wikidata",
            Self::Wikipedia => "wikipedia",
            Self::Oalc => "oalc",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedContextEdge {
    pub source_family: SourceFamily,
    pub source_revision_ref: String,
    pub relation_ref: String,
    pub relation_type_ref: String,
    pub left_ref: String,
    pub right_ref: String,
    pub relation_sha256: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMaterializationReceipt {
    pub materialized_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum ContextFederationError {
    #[error("coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("invalid sha256 hex: {0}")]
    InvalidHex(String),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

/// Build one reviewed, revision-pinned external context edge. Different source
/// revisions necessarily produce different durable relation identities.
pub fn reviewed_context_edge(
    source_family: SourceFamily,
    source_revision_ref: impl Into<String>,
    left_ref: impl Into<String>,
    right_ref: impl Into<String>,
    relation_type_ref: impl Into<String>,
) -> Result<ReviewedContextEdge, ContextFederationError> {
    let source_revision_ref = source_revision_ref.into();
    let left_ref = left_ref.into();
    let right_ref = right_ref.into();
    let relation_type_ref = relation_type_ref.into();

    for (name, value) in [
        ("source_revision_ref", source_revision_ref.as_str()),
        ("left_ref", left_ref.as_str()),
        ("right_ref", right_ref.as_str()),
        ("relation_type_ref", relation_type_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ContextFederationError::EmptyCoordinate(name));
        }
    }

    let mut hasher = Sha256::new();
    for value in [
        "reviewed-context-edge:v1",
        source_family.as_str(),
        source_revision_ref.as_str(),
        relation_type_ref.as_str(),
        left_ref.as_str(),
        right_ref.as_str(),
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    let digest = hasher.finalize();
    let relation_sha256 = crate::hex(&digest);
    let relation_ref = format!(
        "relation:context:{}:{}",
        source_family.as_str(),
        &relation_sha256[..16]
    );

    Ok(ReviewedContextEdge {
        source_family,
        source_revision_ref,
        relation_ref,
        relation_type_ref,
        left_ref,
        right_ref,
        relation_sha256,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

/// Persist reviewed context as `algebra.relation` plus an immutable provenance
/// receipt. The receipt makes source family/revision available to the walker
/// without converting context navigation into legal-IR proof or authority.
pub fn materialize_reviewed_context_edges(
    config: &DatabaseConfig,
    edges: &[ReviewedContextEdge],
) -> Result<ContextMaterializationReceipt, ContextFederationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(CONTEXT_RECEIPT_SCHEMA_SQL)?;

    let mut materialized_count = 0;
    for edge in edges {
        let digest_bytes = crate::decode_hex_32(&edge.relation_sha256)
            .ok_or_else(|| ContextFederationError::InvalidHex(edge.relation_sha256.clone()))?;
        tx.execute(
            "INSERT INTO algebra.relation \
             (relation_ref, relation_type_ref, left_ref, right_ref, relation_sha256) \
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
            &[
                &edge.relation_ref,
                &edge.relation_type_ref,
                &edge.left_ref,
                &edge.right_ref,
                &&digest_bytes[..],
            ],
        )?;
        materialized_count += tx.execute(
            "INSERT INTO context.reviewed_relation_receipt \
             (relation_ref, source_family_ref, source_revision_ref, candidate_only, \
              creates_semantic_authority, applicability_promoted, claim_truth_promoted, receipt_sha256) \
             VALUES ($1, $2, $3, TRUE, FALSE, FALSE, FALSE, $4) \
             ON CONFLICT DO NOTHING",
            &[
                &edge.relation_ref,
                &edge.source_family.as_str(),
                &edge.source_revision_ref,
                &&digest_bytes[..],
            ],
        )?;
    }
    tx.commit()?;

    Ok(ContextMaterializationReceipt {
        materialized_count: materialized_count as usize,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
