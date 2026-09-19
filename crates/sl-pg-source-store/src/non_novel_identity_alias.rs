//! Durable reviewed representation aliases for already-known world identities.
//!
//! An alias receipt records `representation -> existing identity class` so a
//! restart can quotient the representation without pretending that a new world
//! object was discovered. It is deliberately separate from discovery lineage.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub(crate) const NON_NOVEL_IDENTITY_ALIAS_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.reviewed_identity_alias_receipt (
  receipt_sha256 TEXT PRIMARY KEY,
  representation_ref TEXT NOT NULL,
  identity_class_ref TEXT NOT NULL,
  review_ref TEXT NOT NULL,
  source_revision_ref TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  counts_as_novel_discovery BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT counts_as_novel_discovery),
  creates_discovery_lineage BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_discovery_lineage),
  receipt_authority TEXT NOT NULL CHECK (receipt_authority = 'reviewed_non_novel_identity_alias_only'),
  UNIQUE (representation_ref, identity_class_ref, review_ref, source_revision_ref)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonNovelIdentityAliasInput {
    pub representation_ref: String,
    pub identity_class_ref: String,
    pub review_ref: String,
    pub source_revision_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonNovelIdentityAliasRow {
    pub representation_ref: String,
    pub identity_class_ref: String,
    pub review_ref: String,
    pub source_revision_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub counts_as_novel_discovery: bool,
    pub creates_discovery_lineage: bool,
    pub receipt_authority: &'static str,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonNovelIdentityAliasMaterializationReceipt {
    pub attempted_count: usize,
    pub materialized_count: usize,
    pub candidate_only: bool,
    pub counts_as_novel_discovery: bool,
    pub creates_discovery_lineage: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NonNovelIdentityAliasError {
    #[error("alias must remain candidate-only")]
    AliasMustRemainCandidateOnly,
    #[error("alias persistence may not promote authority/applicability/truth")]
    AliasMayNotPromote,
    #[error("identity alias coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for NonNovelIdentityAliasError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn identity_alias_row(
    alias: &NonNovelIdentityAliasInput,
) -> Result<NonNovelIdentityAliasRow, NonNovelIdentityAliasError> {
    if !alias.candidate_only {
        return Err(NonNovelIdentityAliasError::AliasMustRemainCandidateOnly);
    }
    if alias.creates_semantic_authority || alias.applicability_promoted || alias.claim_truth_promoted {
        return Err(NonNovelIdentityAliasError::AliasMayNotPromote);
    }
    for (name, value) in [
        ("representation_ref", alias.representation_ref.as_str()),
        ("identity_class_ref", alias.identity_class_ref.as_str()),
        ("review_ref", alias.review_ref.as_str()),
        ("source_revision_ref", alias.source_revision_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(NonNovelIdentityAliasError::EmptyCoordinate(name));
        }
    }

    let mut hasher = Sha256::new();
    for value in [
        "reviewed-non-novel-identity-alias:v1",
        alias.representation_ref.as_str(),
        alias.identity_class_ref.as_str(),
        alias.review_ref.as_str(),
        alias.source_revision_ref.as_str(),
        "reviewed_non_novel_identity_alias_only",
    ] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }

    Ok(NonNovelIdentityAliasRow {
        representation_ref: alias.representation_ref.clone(),
        identity_class_ref: alias.identity_class_ref.clone(),
        review_ref: alias.review_ref.clone(),
        source_revision_ref: alias.source_revision_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        counts_as_novel_discovery: false,
        creates_discovery_lineage: false,
        receipt_authority: "reviewed_non_novel_identity_alias_only",
        receipt_sha256: hex_digest(&hasher.finalize()),
    })
}

pub fn materialize_non_novel_identity_aliases(
    config: &DatabaseConfig,
    aliases: &[NonNovelIdentityAliasInput],
) -> Result<NonNovelIdentityAliasMaterializationReceipt, NonNovelIdentityAliasError> {
    let rows = aliases
        .iter()
        .map(identity_alias_row)
        .collect::<Result<Vec<_>, _>>()?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(NON_NOVEL_IDENTITY_ALIAS_SCHEMA_SQL)?;
    let mut materialized_count = 0usize;
    for row in &rows {
        materialized_count += tx.execute(
            "INSERT INTO context.reviewed_identity_alias_receipt (\
             receipt_sha256, representation_ref, identity_class_ref, review_ref, source_revision_ref, \
             candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted, \
             counts_as_novel_discovery, creates_discovery_lineage, receipt_authority) VALUES (\
             $1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE,$6) ON CONFLICT DO NOTHING",
            &[
                &row.receipt_sha256,
                &row.representation_ref,
                &row.identity_class_ref,
                &row.review_ref,
                &row.source_revision_ref,
                &row.receipt_authority,
            ],
        )? as usize;
    }
    tx.commit()?;

    Ok(NonNovelIdentityAliasMaterializationReceipt {
        attempted_count: rows.len(),
        materialized_count,
        candidate_only: true,
        counts_as_novel_discovery: false,
        creates_discovery_lineage: false,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
