//! Reviewed PostgreSQL evidence for consumer-relative representation factorisation.
//!
//! This is the durable owner for the factorisation/admission coordinates named by
//! `ConsumerAdequacyReceipt` and `ConsumerInformativenessReceipt`. It does not infer
//! adequacy from wording, persona, complexity, or UI state. Review is explicit and
//! all persisted rows remain non-authoritative/non-adjudicative.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{
    install_legal_controversy_schema, load_legal_controversy_matter, DatabaseConfig,
    LegalControversyStoreError,
};

pub const REVIEWED_REPRESENTATION_FACTORISATION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS consumer_projection;

CREATE TABLE IF NOT EXISTS consumer_projection.reviewed_representation_factorisation (
  factorisation_receipt_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref),
  representation_ref TEXT NOT NULL,
  consumer_ref TEXT NOT NULL,
  query_family_ref TEXT NOT NULL,
  admissibility_receipt_ref TEXT NOT NULL,
  preserved_distinction_refs TEXT[] NOT NULL,
  reviewer_ref TEXT NOT NULL,
  review_decision_ref TEXT NOT NULL,
  accepted BOOLEAN NOT NULL CHECK (accepted),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  determines_credibility BOOLEAN NOT NULL CHECK (NOT determines_credibility),
  enters_final_judgment BOOLEAN NOT NULL CHECK (NOT enters_final_judgment),
  promotes_claim_truth BOOLEAN NOT NULL CHECK (NOT promotes_claim_truth)
);

CREATE TABLE IF NOT EXISTS consumer_projection.reviewed_informativeness_factorisation (
  reviewed_factorisation_receipt_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref),
  left_factorisation_receipt_ref TEXT NOT NULL
    REFERENCES consumer_projection.reviewed_representation_factorisation(factorisation_receipt_ref),
  right_factorisation_receipt_ref TEXT NOT NULL
    REFERENCES consumer_projection.reviewed_representation_factorisation(factorisation_receipt_ref),
  consumer_ref TEXT NOT NULL,
  query_family_ref TEXT NOT NULL,
  right_preserved_distinction_refs TEXT[] NOT NULL,
  left_preserved_distinction_refs TEXT[] NOT NULL,
  reviewer_ref TEXT NOT NULL,
  review_decision_ref TEXT NOT NULL,
  accepted BOOLEAN NOT NULL CHECK (accepted),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_universal_superiority BOOLEAN NOT NULL CHECK (NOT creates_universal_superiority),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  determines_credibility BOOLEAN NOT NULL CHECK (NOT determines_credibility),
  enters_final_judgment BOOLEAN NOT NULL CHECK (NOT enters_final_judgment),
  promotes_claim_truth BOOLEAN NOT NULL CHECK (NOT promotes_claim_truth)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedRepresentationFactorisation {
    pub factorisation_receipt_ref: String,
    pub matter_ref: String,
    pub controversy_ref: String,
    pub representation_ref: String,
    pub consumer_ref: String,
    pub query_family_ref: String,
    pub admissibility_receipt_ref: String,
    pub preserved_distinction_refs: BTreeSet<String>,
    pub reviewer_ref: String,
    pub review_decision_ref: String,
    pub accepted: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub determines_credibility: bool,
    pub enters_final_judgment: bool,
    pub promotes_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedInformativenessFactorisation {
    pub reviewed_factorisation_receipt_ref: String,
    pub matter_ref: String,
    pub controversy_ref: String,
    pub left_factorisation_receipt_ref: String,
    pub right_factorisation_receipt_ref: String,
    pub consumer_ref: String,
    pub query_family_ref: String,
    pub right_preserved_distinction_refs: BTreeSet<String>,
    pub left_preserved_distinction_refs: BTreeSet<String>,
    pub reviewer_ref: String,
    pub review_decision_ref: String,
    pub accepted: bool,
    pub candidate_only: bool,
    pub creates_universal_superiority: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub determines_credibility: bool,
    pub enters_final_judgment: bool,
    pub promotes_claim_truth: bool,
}

#[derive(Debug, Error)]
pub enum ReviewedRepresentationFactorisationStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("legal controversy prerequisite failed: {0}")]
    LegalControversy(#[from] LegalControversyStoreError),
    #[error("empty reviewed factorisation coordinate: {0}")]
    EmptyCoordinate(&'static str),
    #[error("reviewed factorisation must remain explicitly accepted/candidate-only and non-promoting")]
    InvalidReviewBoundary,
    #[error("reviewed factorisation matter_ref does not match persisted controversy Matter")]
    MatterBindingMismatch,
    #[error("reviewed informativeness factorisation has mismatched consumer/query coordinates")]
    ConsumerQueryMismatch,
    #[error("reviewed informativeness factorisation does not have left >= right distinction coverage")]
    CoverageMismatch,
    #[error("persisted reviewed factorisation conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("reviewed factorisation not found: {0}")]
    NotFound(String),
}

fn require_nonempty(
    name: &'static str,
    value: &str,
) -> Result<(), ReviewedRepresentationFactorisationStoreError> {
    if value.trim().is_empty() {
        Err(ReviewedRepresentationFactorisationStoreError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn require_matter_binding(
    config: &DatabaseConfig,
    matter_ref: &str,
    controversy_ref: &str,
) -> Result<(), ReviewedRepresentationFactorisationStoreError> {
    let persisted = load_legal_controversy_matter(config, controversy_ref)?;
    if persisted.matter_ref != matter_ref {
        return Err(ReviewedRepresentationFactorisationStoreError::MatterBindingMismatch);
    }
    Ok(())
}

fn vec_from_set(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
}

fn set_from_vec(values: Vec<String>) -> BTreeSet<String> {
    values.into_iter().collect()
}

impl ReviewedRepresentationFactorisation {
    pub fn validate(&self) -> Result<(), ReviewedRepresentationFactorisationStoreError> {
        for (name, value) in [
            ("factorisation_receipt_ref", self.factorisation_receipt_ref.as_str()),
            ("matter_ref", self.matter_ref.as_str()),
            ("controversy_ref", self.controversy_ref.as_str()),
            ("representation_ref", self.representation_ref.as_str()),
            ("consumer_ref", self.consumer_ref.as_str()),
            ("query_family_ref", self.query_family_ref.as_str()),
            ("admissibility_receipt_ref", self.admissibility_receipt_ref.as_str()),
            ("reviewer_ref", self.reviewer_ref.as_str()),
            ("review_decision_ref", self.review_decision_ref.as_str()),
        ] {
            require_nonempty(name, value)?;
        }
        if self.preserved_distinction_refs.is_empty()
            || self.preserved_distinction_refs.iter().any(|value| value.trim().is_empty())
        {
            return Err(ReviewedRepresentationFactorisationStoreError::EmptyCoordinate(
                "preserved_distinction_refs",
            ));
        }
        if !self.accepted
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_legal_authority
            || self.determines_credibility
            || self.enters_final_judgment
            || self.promotes_claim_truth
        {
            return Err(ReviewedRepresentationFactorisationStoreError::InvalidReviewBoundary);
        }
        Ok(())
    }
}

impl ReviewedInformativenessFactorisation {
    pub fn validate(&self) -> Result<(), ReviewedRepresentationFactorisationStoreError> {
        for (name, value) in [
            (
                "reviewed_factorisation_receipt_ref",
                self.reviewed_factorisation_receipt_ref.as_str(),
            ),
            ("matter_ref", self.matter_ref.as_str()),
            ("controversy_ref", self.controversy_ref.as_str()),
            (
                "left_factorisation_receipt_ref",
                self.left_factorisation_receipt_ref.as_str(),
            ),
            (
                "right_factorisation_receipt_ref",
                self.right_factorisation_receipt_ref.as_str(),
            ),
            ("consumer_ref", self.consumer_ref.as_str()),
            ("query_family_ref", self.query_family_ref.as_str()),
            ("reviewer_ref", self.reviewer_ref.as_str()),
            ("review_decision_ref", self.review_decision_ref.as_str()),
        ] {
            require_nonempty(name, value)?;
        }
        if self.right_preserved_distinction_refs.is_empty()
            || self
                .right_preserved_distinction_refs
                .iter()
                .chain(self.left_preserved_distinction_refs.iter())
                .any(|value| value.trim().is_empty())
        {
            return Err(ReviewedRepresentationFactorisationStoreError::EmptyCoordinate(
                "preserved_distinction_refs",
            ));
        }
        if !self
            .right_preserved_distinction_refs
            .is_subset(&self.left_preserved_distinction_refs)
        {
            return Err(ReviewedRepresentationFactorisationStoreError::CoverageMismatch);
        }
        if !self.accepted
            || !self.candidate_only
            || self.creates_universal_superiority
            || self.creates_semantic_authority
            || self.creates_legal_authority
            || self.determines_credibility
            || self.enters_final_judgment
            || self.promotes_claim_truth
        {
            return Err(ReviewedRepresentationFactorisationStoreError::InvalidReviewBoundary);
        }
        Ok(())
    }
}

pub fn install_reviewed_representation_factorisation_schema(
    config: &DatabaseConfig,
) -> Result<(), ReviewedRepresentationFactorisationStoreError> {
    install_legal_controversy_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REVIEWED_REPRESENTATION_FACTORISATION_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_reviewed_representation_factorisation(
    config: &DatabaseConfig,
    value: &ReviewedRepresentationFactorisation,
) -> Result<ReviewedRepresentationFactorisation, ReviewedRepresentationFactorisationStoreError> {
    value.validate()?;
    install_reviewed_representation_factorisation_schema(config)?;
    require_matter_binding(config, &value.matter_ref, &value.controversy_ref)?;
    let distinctions = vec_from_set(&value.preserved_distinction_refs);
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO consumer_projection.reviewed_representation_factorisation
           (factorisation_receipt_ref,matter_ref,controversy_ref,representation_ref,
            consumer_ref,query_family_ref,admissibility_receipt_ref,preserved_distinction_refs,
            reviewer_ref,review_decision_ref,accepted,candidate_only,creates_semantic_authority,
            creates_legal_authority,determines_credibility,enters_final_judgment,promotes_claim_truth)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(factorisation_receipt_ref) DO NOTHING"#,
        &[
            &value.factorisation_receipt_ref,
            &value.matter_ref,
            &value.controversy_ref,
            &value.representation_ref,
            &value.consumer_ref,
            &value.query_family_ref,
            &value.admissibility_receipt_ref,
            &distinctions,
            &value.reviewer_ref,
            &value.review_decision_ref,
        ],
    )?;
    let reopened = load_reviewed_representation_factorisation(config, &value.factorisation_receipt_ref)?;
    if reopened != *value {
        return Err(ReviewedRepresentationFactorisationStoreError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_reviewed_representation_factorisation(
    config: &DatabaseConfig,
    factorisation_receipt_ref: &str,
) -> Result<ReviewedRepresentationFactorisation, ReviewedRepresentationFactorisationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT matter_ref,controversy_ref,representation_ref,consumer_ref,query_family_ref,
                      admissibility_receipt_ref,preserved_distinction_refs,reviewer_ref,
                      review_decision_ref,accepted,candidate_only,creates_semantic_authority,
                      creates_legal_authority,determines_credibility,enters_final_judgment,
                      promotes_claim_truth
               FROM consumer_projection.reviewed_representation_factorisation
               WHERE factorisation_receipt_ref=$1"#,
            &[&factorisation_receipt_ref],
        )?
        .ok_or_else(|| {
            ReviewedRepresentationFactorisationStoreError::NotFound(
                factorisation_receipt_ref.to_owned(),
            )
        })?;
    let value = ReviewedRepresentationFactorisation {
        factorisation_receipt_ref: factorisation_receipt_ref.to_owned(),
        matter_ref: row.get(0),
        controversy_ref: row.get(1),
        representation_ref: row.get(2),
        consumer_ref: row.get(3),
        query_family_ref: row.get(4),
        admissibility_receipt_ref: row.get(5),
        preserved_distinction_refs: set_from_vec(row.get(6)),
        reviewer_ref: row.get(7),
        review_decision_ref: row.get(8),
        accepted: row.get(9),
        candidate_only: row.get(10),
        creates_semantic_authority: row.get(11),
        creates_legal_authority: row.get(12),
        determines_credibility: row.get(13),
        enters_final_judgment: row.get(14),
        promotes_claim_truth: row.get(15),
    };
    value.validate()?;
    require_matter_binding(config, &value.matter_ref, &value.controversy_ref)?;
    Ok(value)
}

pub fn persist_reviewed_informativeness_factorisation(
    config: &DatabaseConfig,
    value: &ReviewedInformativenessFactorisation,
) -> Result<ReviewedInformativenessFactorisation, ReviewedRepresentationFactorisationStoreError> {
    value.validate()?;
    install_reviewed_representation_factorisation_schema(config)?;
    require_matter_binding(config, &value.matter_ref, &value.controversy_ref)?;
    let left = load_reviewed_representation_factorisation(config, &value.left_factorisation_receipt_ref)?;
    let right = load_reviewed_representation_factorisation(config, &value.right_factorisation_receipt_ref)?;
    if left.matter_ref != value.matter_ref
        || right.matter_ref != value.matter_ref
        || left.controversy_ref != value.controversy_ref
        || right.controversy_ref != value.controversy_ref
        || left.consumer_ref != value.consumer_ref
        || right.consumer_ref != value.consumer_ref
        || left.query_family_ref != value.query_family_ref
        || right.query_family_ref != value.query_family_ref
    {
        return Err(ReviewedRepresentationFactorisationStoreError::ConsumerQueryMismatch);
    }
    if left.preserved_distinction_refs != value.left_preserved_distinction_refs
        || right.preserved_distinction_refs != value.right_preserved_distinction_refs
    {
        return Err(ReviewedRepresentationFactorisationStoreError::CoverageMismatch);
    }

    let right_distinctions = vec_from_set(&value.right_preserved_distinction_refs);
    let left_distinctions = vec_from_set(&value.left_preserved_distinction_refs);
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO consumer_projection.reviewed_informativeness_factorisation
           (reviewed_factorisation_receipt_ref,matter_ref,controversy_ref,
            left_factorisation_receipt_ref,right_factorisation_receipt_ref,consumer_ref,
            query_family_ref,right_preserved_distinction_refs,left_preserved_distinction_refs,
            reviewer_ref,review_decision_ref,accepted,candidate_only,
            creates_universal_superiority,creates_semantic_authority,creates_legal_authority,
            determines_credibility,enters_final_judgment,promotes_claim_truth)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,TRUE,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(reviewed_factorisation_receipt_ref) DO NOTHING"#,
        &[
            &value.reviewed_factorisation_receipt_ref,
            &value.matter_ref,
            &value.controversy_ref,
            &value.left_factorisation_receipt_ref,
            &value.right_factorisation_receipt_ref,
            &value.consumer_ref,
            &value.query_family_ref,
            &right_distinctions,
            &left_distinctions,
            &value.reviewer_ref,
            &value.review_decision_ref,
        ],
    )?;
    let reopened = load_reviewed_informativeness_factorisation(
        config,
        &value.reviewed_factorisation_receipt_ref,
    )?;
    if reopened != *value {
        return Err(ReviewedRepresentationFactorisationStoreError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_reviewed_informativeness_factorisation(
    config: &DatabaseConfig,
    reviewed_factorisation_receipt_ref: &str,
) -> Result<ReviewedInformativenessFactorisation, ReviewedRepresentationFactorisationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT matter_ref,controversy_ref,left_factorisation_receipt_ref,
                      right_factorisation_receipt_ref,consumer_ref,query_family_ref,
                      right_preserved_distinction_refs,left_preserved_distinction_refs,
                      reviewer_ref,review_decision_ref,accepted,candidate_only,
                      creates_universal_superiority,creates_semantic_authority,
                      creates_legal_authority,determines_credibility,enters_final_judgment,
                      promotes_claim_truth
               FROM consumer_projection.reviewed_informativeness_factorisation
               WHERE reviewed_factorisation_receipt_ref=$1"#,
            &[&reviewed_factorisation_receipt_ref],
        )?
        .ok_or_else(|| {
            ReviewedRepresentationFactorisationStoreError::NotFound(
                reviewed_factorisation_receipt_ref.to_owned(),
            )
        })?;
    let value = ReviewedInformativenessFactorisation {
        reviewed_factorisation_receipt_ref: reviewed_factorisation_receipt_ref.to_owned(),
        matter_ref: row.get(0),
        controversy_ref: row.get(1),
        left_factorisation_receipt_ref: row.get(2),
        right_factorisation_receipt_ref: row.get(3),
        consumer_ref: row.get(4),
        query_family_ref: row.get(5),
        right_preserved_distinction_refs: set_from_vec(row.get(6)),
        left_preserved_distinction_refs: set_from_vec(row.get(7)),
        reviewer_ref: row.get(8),
        review_decision_ref: row.get(9),
        accepted: row.get(10),
        candidate_only: row.get(11),
        creates_universal_superiority: row.get(12),
        creates_semantic_authority: row.get(13),
        creates_legal_authority: row.get(14),
        determines_credibility: row.get(15),
        enters_final_judgment: row.get(16),
        promotes_claim_truth: row.get(17),
    };
    value.validate()?;
    require_matter_binding(config, &value.matter_ref, &value.controversy_ref)?;
    let left = load_reviewed_representation_factorisation(config, &value.left_factorisation_receipt_ref)?;
    let right = load_reviewed_representation_factorisation(config, &value.right_factorisation_receipt_ref)?;
    if left.consumer_ref != value.consumer_ref
        || right.consumer_ref != value.consumer_ref
        || left.query_family_ref != value.query_family_ref
        || right.query_family_ref != value.query_family_ref
        || left.preserved_distinction_refs != value.left_preserved_distinction_refs
        || right.preserved_distinction_refs != value.right_preserved_distinction_refs
    {
        return Err(ReviewedRepresentationFactorisationStoreError::ExistingRowConflict);
    }
    Ok(value)
}
