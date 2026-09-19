//! Durable open/closed ambiguity state for the GWB adaptive campaign.
//!
//! This store contains consumer-relative research residuals only. It is not an
//! ontology, a truth store, or a publication surface.

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::DatabaseConfig;

const GWB_AMBIGUITY_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS context;
CREATE TABLE IF NOT EXISTS context.gwb_ambiguity_residual (
  campaign_ref TEXT NOT NULL,
  residual_ref TEXT NOT NULL,
  subject_ref TEXT NOT NULL,
  proposition_ref TEXT NOT NULL,
  kind_ref TEXT NOT NULL,
  root_qid TEXT,
  salience BIGINT NOT NULL CHECK (salience >= 0),
  dependency_refs TEXT[] NOT NULL DEFAULT '{}',
  status_ref TEXT NOT NULL CHECK (status_ref IN ('open','closed-reviewed')),
  opened_by_hop BIGINT,
  closed_by_hop BIGINT,
  close_review_ref TEXT,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT creates_semantic_authority),
  applicability_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL DEFAULT FALSE CHECK (NOT claim_truth_promoted),
  PRIMARY KEY (campaign_ref, residual_ref)
);
"#;

const ALLOWED_KINDS: &[&str] = &[
    "identity",
    "source-work-identity",
    "type-class",
    "superclass",
    "subclass",
    "property-support",
    "cross-language-gap",
    "unsupported-dependency",
    "provenance",
    "competing-alternatives",
    "consumer-semantic-gap",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbAmbiguityStateInput {
    pub campaign_ref: String,
    pub residual_ref: String,
    pub subject_ref: String,
    pub proposition_ref: String,
    pub kind_ref: String,
    pub root_qid: Option<String>,
    pub salience: u64,
    pub dependency_refs: Vec<String>,
    pub opened_by_hop: Option<usize>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbAmbiguityStateRow {
    pub campaign_ref: String,
    pub residual_ref: String,
    pub subject_ref: String,
    pub proposition_ref: String,
    pub kind_ref: String,
    pub root_qid: Option<String>,
    pub salience: u64,
    pub dependency_refs: Vec<String>,
    pub status_ref: &'static str,
    pub opened_by_hop: Option<usize>,
    pub closed_by_hop: Option<usize>,
    pub close_review_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbAmbiguityMaterializationReceipt {
    pub attempted_count: usize,
    pub inserted_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GwbAmbiguityStateError {
    #[error("GWB ambiguity coordinate must not be empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("invalid GWB ambiguity kind: {0}")]
    InvalidKind(String),
    #[error("GWB ambiguity state must remain candidate-only")]
    StateMustRemainCandidateOnly,
    #[error("GWB ambiguity state may not promote authority/applicability/truth")]
    StateMayNotPromote,
    #[error("GWB salience exceeds PostgreSQL BIGINT range")]
    SalienceOutOfRange,
    #[error("GWB hop index exceeds PostgreSQL BIGINT range")]
    HopIndexOutOfRange,
    #[error("cannot close unknown GWB residual: {0}")]
    UnknownResidual(String),
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for GwbAmbiguityStateError {
    fn from(value: postgres::Error) -> Self {
        Self::Postgres(value.to_string())
    }
}

fn valid_kind(value: &str) -> bool {
    ALLOWED_KINDS.contains(&value)
}

pub fn gwb_ambiguity_state_row(
    input: &GwbAmbiguityStateInput,
) -> Result<GwbAmbiguityStateRow, GwbAmbiguityStateError> {
    for (name, value) in [
        ("campaign_ref", input.campaign_ref.as_str()),
        ("residual_ref", input.residual_ref.as_str()),
        ("subject_ref", input.subject_ref.as_str()),
        ("proposition_ref", input.proposition_ref.as_str()),
        ("kind_ref", input.kind_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(GwbAmbiguityStateError::EmptyCoordinate(name));
        }
    }
    if !valid_kind(&input.kind_ref) {
        return Err(GwbAmbiguityStateError::InvalidKind(input.kind_ref.clone()));
    }
    if !input.candidate_only {
        return Err(GwbAmbiguityStateError::StateMustRemainCandidateOnly);
    }
    if input.creates_semantic_authority
        || input.applicability_promoted
        || input.claim_truth_promoted
    {
        return Err(GwbAmbiguityStateError::StateMayNotPromote);
    }
    if input.salience > i64::MAX as u64 {
        return Err(GwbAmbiguityStateError::SalienceOutOfRange);
    }
    if input
        .opened_by_hop
        .is_some_and(|index| index > i64::MAX as usize)
    {
        return Err(GwbAmbiguityStateError::HopIndexOutOfRange);
    }
    let mut dependencies = input.dependency_refs.clone();
    dependencies.sort();
    dependencies.dedup();

    Ok(GwbAmbiguityStateRow {
        campaign_ref: input.campaign_ref.clone(),
        residual_ref: input.residual_ref.clone(),
        subject_ref: input.subject_ref.clone(),
        proposition_ref: input.proposition_ref.clone(),
        kind_ref: input.kind_ref.clone(),
        root_qid: input.root_qid.clone(),
        salience: input.salience,
        dependency_refs: dependencies,
        status_ref: "open",
        opened_by_hop: input.opened_by_hop,
        closed_by_hop: None,
        close_review_ref: None,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn materialize_gwb_ambiguity_residuals(
    config: &DatabaseConfig,
    inputs: &[GwbAmbiguityStateInput],
) -> Result<GwbAmbiguityMaterializationReceipt, GwbAmbiguityStateError> {
    let rows = inputs
        .iter()
        .map(gwb_ambiguity_state_row)
        .collect::<Result<Vec<_>, _>>()?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(GWB_AMBIGUITY_SCHEMA_SQL)?;
    let mut inserted_count = 0usize;
    for row in &rows {
        let opened_by_hop = row
            .opened_by_hop
            .map(|value| i64::try_from(value).map_err(|_| GwbAmbiguityStateError::HopIndexOutOfRange))
            .transpose()?;
        inserted_count += tx.execute(
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
    tx.commit()?;
    Ok(GwbAmbiguityMaterializationReceipt {
        attempted_count: rows.len(),
        inserted_count,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn load_open_gwb_ambiguity_residuals(
    config: &DatabaseConfig,
    campaign_ref: &str,
) -> Result<Vec<GwbAmbiguityStateRow>, GwbAmbiguityStateError> {
    if campaign_ref.trim().is_empty() {
        return Err(GwbAmbiguityStateError::EmptyCoordinate("campaign_ref"));
    }
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(GWB_AMBIGUITY_SCHEMA_SQL)?;
    let rows = client.query(
        "SELECT campaign_ref,residual_ref,subject_ref,proposition_ref,kind_ref,root_qid,salience,                dependency_refs,opened_by_hop          FROM context.gwb_ambiguity_residual          WHERE campaign_ref=$1 AND status_ref='open'            AND candidate_only=TRUE AND creates_semantic_authority=FALSE            AND applicability_promoted=FALSE AND claim_truth_promoted=FALSE          ORDER BY salience DESC,residual_ref",
        &[&campaign_ref],
    )?;
    rows.into_iter()
        .map(|row| {
            let salience: i64 = row.get(6);
            let opened: Option<i64> = row.get(8);
            Ok(GwbAmbiguityStateRow {
                campaign_ref: row.get(0),
                residual_ref: row.get(1),
                subject_ref: row.get(2),
                proposition_ref: row.get(3),
                kind_ref: row.get(4),
                root_qid: row.get(5),
                salience: u64::try_from(salience)
                    .map_err(|_| GwbAmbiguityStateError::SalienceOutOfRange)?,
                dependency_refs: row.get(7),
                status_ref: "open",
                opened_by_hop: opened
                    .map(|value| usize::try_from(value)
                        .map_err(|_| GwbAmbiguityStateError::HopIndexOutOfRange))
                    .transpose()?,
                closed_by_hop: None,
                close_review_ref: None,
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            })
        })
        .collect()
}

pub fn close_gwb_ambiguity_residuals(
    config: &DatabaseConfig,
    campaign_ref: &str,
    residual_refs: &[String],
    hop_index: usize,
    review_ref: &str,
) -> Result<usize, GwbAmbiguityStateError> {
    if campaign_ref.trim().is_empty() {
        return Err(GwbAmbiguityStateError::EmptyCoordinate("campaign_ref"));
    }
    if review_ref.trim().is_empty() {
        return Err(GwbAmbiguityStateError::EmptyCoordinate("review_ref"));
    }
    let hop = i64::try_from(hop_index).map_err(|_| GwbAmbiguityStateError::HopIndexOutOfRange)?;
    let mut refs = residual_refs.to_vec();
    refs.sort();
    refs.dedup();

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(GWB_AMBIGUITY_SCHEMA_SQL)?;
    let mut updated = 0usize;
    for residual_ref in refs {
        let count = tx.execute(
            "UPDATE context.gwb_ambiguity_residual              SET status_ref='closed-reviewed',closed_by_hop=$1,close_review_ref=$2              WHERE campaign_ref=$3 AND residual_ref=$4 AND status_ref='open'",
            &[&hop, &review_ref, &campaign_ref, &residual_ref],
        )? as usize;
        if count == 0 {
            return Err(GwbAmbiguityStateError::UnknownResidual(residual_ref));
        }
        updated = updated.saturating_add(count);
    }
    tx.commit()?;
    Ok(updated)
}
