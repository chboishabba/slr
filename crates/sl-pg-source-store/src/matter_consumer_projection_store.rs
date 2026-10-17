//! PostgreSQL persistence for consumer-relative representations of one canonical Matter.
//!
//! This store persists already-reviewed adequacy/informativeness receipts. It never
//! derives adequacy from UI wording, persona labels, description length, or source text.
//! Stable refs are immutable: every idempotent write is reopened and compared exactly.

use std::collections::BTreeSet;

use postgres::{Client, NoTls};
use thiserror::Error;

use sensiblaw_core::canonical_evidence::{
    ConsumerAdequacyReceipt, ConsumerInformativenessReceipt, MatterPersona,
    MatterRepresentationProjection, SameMatterInformativenessReceipt,
};

use crate::{
    install_legal_controversy_schema, install_reviewed_representation_factorisation_schema,
    load_legal_controversy_matter, load_reviewed_informativeness_factorisation,
    load_reviewed_representation_factorisation, DatabaseConfig, LegalControversyStoreError,
    ReviewedRepresentationFactorisationStoreError,
};

pub const MATTER_CONSUMER_PROJECTION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS consumer_projection;

CREATE TABLE IF NOT EXISTS consumer_projection.matter_representation (
  projection_receipt_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref),
  persona_ref TEXT NOT NULL,
  representation_ref TEXT NOT NULL,
  consumer_ref TEXT NOT NULL,
  query_family_ref TEXT NOT NULL,
  admissibility_receipt_ref TEXT NOT NULL,
  factorisation_receipt_refs TEXT[] NOT NULL,
  admissible BOOLEAN NOT NULL,
  consumer_adequate BOOLEAN NOT NULL,
  description_length_rank BIGINT NULL CHECK (description_length_rank IS NULL OR description_length_rank >= 0),
  creates_new_matter BOOLEAN NOT NULL CHECK (NOT creates_new_matter),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  determines_credibility BOOLEAN NOT NULL CHECK (NOT determines_credibility),
  enters_final_judgment BOOLEAN NOT NULL CHECK (NOT enters_final_judgment)
);

CREATE INDEX IF NOT EXISTS matter_representation_matter_idx
  ON consumer_projection.matter_representation(matter_ref, controversy_ref);

CREATE TABLE IF NOT EXISTS consumer_projection.same_matter_informativeness (
  receipt_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  controversy_ref TEXT NOT NULL REFERENCES legal_controversy.matter(controversy_ref),
  left_projection_receipt_ref TEXT NOT NULL
    REFERENCES consumer_projection.matter_representation(projection_receipt_ref),
  right_projection_receipt_ref TEXT NOT NULL
    REFERENCES consumer_projection.matter_representation(projection_receipt_ref),
  right_preserved_distinction_refs TEXT[] NOT NULL,
  left_preserved_distinction_refs TEXT[] NOT NULL,
  reviewed_factorisation_receipt_ref TEXT NOT NULL
    REFERENCES consumer_projection.reviewed_informativeness_factorisation(reviewed_factorisation_receipt_ref),
  creates_universal_superiority BOOLEAN NOT NULL CHECK (NOT creates_universal_superiority),
  promotes_claim_truth BOOLEAN NOT NULL CHECK (NOT promotes_claim_truth),
  creates_new_matter BOOLEAN NOT NULL CHECK (NOT creates_new_matter),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  determines_credibility BOOLEAN NOT NULL CHECK (NOT determines_credibility),
  enters_final_judgment BOOLEAN NOT NULL CHECK (NOT enters_final_judgment)
);
"#;

#[derive(Debug, Error)]
pub enum MatterConsumerProjectionStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("legal controversy prerequisite failed: {0}")]
    LegalControversy(#[from] LegalControversyStoreError),
    #[error("reviewed factorisation prerequisite failed: {0}")]
    ReviewedFactorisation(#[from] ReviewedRepresentationFactorisationStoreError),
    #[error("invalid matter consumer projection receipt")]
    InvalidProjection,
    #[error("projection matter_ref does not match persisted controversy Matter")]
    MatterBindingMismatch,
    #[error("reviewed factorisation evidence does not match the projection/informativeness coordinates")]
    ReviewedFactorisationMismatch,
    #[error("persisted matter consumer projection row conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("matter consumer projection row not found: {0}")]
    NotFound(String),
    #[error("unknown matter persona: {0}")]
    UnknownPersona(String),
    #[error("description length rank exceeds PostgreSQL BIGINT")]
    RankOverflow,
}

fn persona_db(value: MatterPersona) -> &'static str {
    match value {
        MatterPersona::AffectedPerson => "affected_person",
        MatterPersona::SolicitorCounsel => "solicitor_counsel",
        MatterPersona::CourtAssociate => "court_associate",
        MatterPersona::Novice => "novice",
        MatterPersona::Expert => "expert",
        MatterPersona::DomainSpecialist => "domain_specialist",
    }
}

fn persona_from_db(value: &str) -> Result<MatterPersona, MatterConsumerProjectionStoreError> {
    match value {
        "affected_person" => Ok(MatterPersona::AffectedPerson),
        "solicitor_counsel" => Ok(MatterPersona::SolicitorCounsel),
        "court_associate" => Ok(MatterPersona::CourtAssociate),
        "novice" => Ok(MatterPersona::Novice),
        "expert" => Ok(MatterPersona::Expert),
        "domain_specialist" => Ok(MatterPersona::DomainSpecialist),
        other => Err(MatterConsumerProjectionStoreError::UnknownPersona(other.to_owned())),
    }
}

fn vec_from_set(values: &BTreeSet<String>) -> Vec<String> {
    values.iter().cloned().collect()
}

fn rank_to_db(value: Option<u64>) -> Result<Option<i64>, MatterConsumerProjectionStoreError> {
    value
        .map(|rank| i64::try_from(rank).map_err(|_| MatterConsumerProjectionStoreError::RankOverflow))
        .transpose()
}

fn rank_from_db(value: Option<i64>) -> Result<Option<u64>, MatterConsumerProjectionStoreError> {
    value
        .map(|rank| u64::try_from(rank).map_err(|_| MatterConsumerProjectionStoreError::RankOverflow))
        .transpose()
}

fn require_persisted_matter_binding(
    config: &DatabaseConfig,
    matter_ref: &str,
    controversy_ref: &str,
) -> Result<(), MatterConsumerProjectionStoreError> {
    let persisted = load_legal_controversy_matter(config, controversy_ref)?;
    if persisted.matter_ref != matter_ref {
        return Err(MatterConsumerProjectionStoreError::MatterBindingMismatch);
    }
    Ok(())
}

fn require_reviewed_projection_factorisations(
    config: &DatabaseConfig,
    value: &MatterRepresentationProjection,
) -> Result<(), MatterConsumerProjectionStoreError> {
    if value.adequacy.factorisation_receipt_refs.is_empty() {
        return Err(MatterConsumerProjectionStoreError::ReviewedFactorisationMismatch);
    }
    for factorisation_ref in &value.adequacy.factorisation_receipt_refs {
        let reviewed = load_reviewed_representation_factorisation(config, factorisation_ref)?;
        if reviewed.matter_ref != value.matter_ref
            || reviewed.controversy_ref != value.controversy_ref
            || reviewed.representation_ref != value.representation_ref
            || reviewed.consumer_ref != value.adequacy.consumer_ref
            || reviewed.query_family_ref != value.adequacy.query_family_ref
            || reviewed.admissibility_receipt_ref != value.adequacy.admissibility_receipt_ref
            || !reviewed.accepted
            || !reviewed.candidate_only
            || reviewed.creates_semantic_authority
            || reviewed.creates_legal_authority
            || reviewed.determines_credibility
            || reviewed.enters_final_judgment
            || reviewed.promotes_claim_truth
        {
            return Err(MatterConsumerProjectionStoreError::ReviewedFactorisationMismatch);
        }
    }
    Ok(())
}

fn require_reviewed_informativeness_factorisation(
    config: &DatabaseConfig,
    value: &SameMatterInformativenessReceipt,
) -> Result<(), MatterConsumerProjectionStoreError> {
    let reviewed = load_reviewed_informativeness_factorisation(
        config,
        &value.informativeness.reviewed_factorisation_receipt_ref,
    )?;
    if reviewed.matter_ref != value.left.matter_ref
        || reviewed.controversy_ref != value.left.controversy_ref
        || reviewed.consumer_ref != value.informativeness.left.consumer_ref
        || reviewed.query_family_ref != value.informativeness.left.query_family_ref
        || reviewed.left_preserved_distinction_refs
            != value.informativeness.left_preserved_distinction_refs
        || reviewed.right_preserved_distinction_refs
            != value.informativeness.right_preserved_distinction_refs
        || !value
            .left
            .adequacy
            .factorisation_receipt_refs
            .contains(&reviewed.left_factorisation_receipt_ref)
        || !value
            .right
            .adequacy
            .factorisation_receipt_refs
            .contains(&reviewed.right_factorisation_receipt_ref)
        || !reviewed.accepted
        || !reviewed.candidate_only
        || reviewed.creates_universal_superiority
        || reviewed.creates_semantic_authority
        || reviewed.creates_legal_authority
        || reviewed.determines_credibility
        || reviewed.enters_final_judgment
        || reviewed.promotes_claim_truth
    {
        return Err(MatterConsumerProjectionStoreError::ReviewedFactorisationMismatch);
    }
    Ok(())
}

pub fn install_matter_consumer_projection_schema(
    config: &DatabaseConfig,
) -> Result<(), MatterConsumerProjectionStoreError> {
    install_legal_controversy_schema(config)?;
    install_reviewed_representation_factorisation_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(MATTER_CONSUMER_PROJECTION_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_matter_representation_projection(
    config: &DatabaseConfig,
    value: &MatterRepresentationProjection,
) -> Result<MatterRepresentationProjection, MatterConsumerProjectionStoreError> {
    value
        .validate()
        .map_err(|_| MatterConsumerProjectionStoreError::InvalidProjection)?;
    install_matter_consumer_projection_schema(config)?;
    require_persisted_matter_binding(config, &value.matter_ref, &value.controversy_ref)?;
    require_reviewed_projection_factorisations(config, value)?;

    let factorisation_refs = vec_from_set(&value.adequacy.factorisation_receipt_refs);
    let rank = rank_to_db(value.adequacy.description_length_rank)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO consumer_projection.matter_representation
           (projection_receipt_ref,matter_ref,controversy_ref,persona_ref,representation_ref,
            consumer_ref,query_family_ref,admissibility_receipt_ref,factorisation_receipt_refs,
            admissible,consumer_adequate,description_length_rank,creates_new_matter,
            creates_semantic_authority,determines_credibility,enters_final_judgment)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(projection_receipt_ref) DO NOTHING"#,
        &[
            &value.projection_receipt_ref,
            &value.matter_ref,
            &value.controversy_ref,
            &persona_db(value.persona),
            &value.representation_ref,
            &value.adequacy.consumer_ref,
            &value.adequacy.query_family_ref,
            &value.adequacy.admissibility_receipt_ref,
            &factorisation_refs,
            &value.adequacy.admissible,
            &value.adequacy.consumer_adequate,
            &rank,
        ],
    )?;

    let reopened = load_matter_representation_projection(config, &value.projection_receipt_ref)?;
    if reopened != *value {
        return Err(MatterConsumerProjectionStoreError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_matter_representation_projection(
    config: &DatabaseConfig,
    projection_receipt_ref: &str,
) -> Result<MatterRepresentationProjection, MatterConsumerProjectionStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT matter_ref,controversy_ref,persona_ref,representation_ref,consumer_ref,
                      query_family_ref,admissibility_receipt_ref,factorisation_receipt_refs,
                      admissible,consumer_adequate,description_length_rank,
                      creates_new_matter,creates_semantic_authority,determines_credibility,
                      enters_final_judgment
               FROM consumer_projection.matter_representation
               WHERE projection_receipt_ref=$1"#,
            &[&projection_receipt_ref],
        )?
        .ok_or_else(|| MatterConsumerProjectionStoreError::NotFound(projection_receipt_ref.to_owned()))?;

    let matter_ref: String = row.get(0);
    let controversy_ref: String = row.get(1);
    require_persisted_matter_binding(config, &matter_ref, &controversy_ref)?;
    let persona_ref: String = row.get(2);
    let factorisation_refs: Vec<String> = row.get(7);
    let rank: Option<i64> = row.get(10);
    let adequacy = ConsumerAdequacyReceipt::new(
        row.get::<_, String>(3),
        row.get::<_, String>(4),
        row.get::<_, String>(5),
        row.get::<_, String>(6),
        factorisation_refs,
        row.get(8),
        row.get(9),
        rank_from_db(rank)?,
    )
    .map_err(|_| MatterConsumerProjectionStoreError::InvalidProjection)?;

    let projection = MatterRepresentationProjection::new(
        matter_ref,
        controversy_ref,
        persona_from_db(&persona_ref)?,
        adequacy.representation_ref.clone(),
        adequacy,
        projection_receipt_ref.to_owned(),
    )
    .map_err(|_| MatterConsumerProjectionStoreError::InvalidProjection)?;

    if row.get::<_, bool>(11)
        || row.get::<_, bool>(12)
        || row.get::<_, bool>(13)
        || row.get::<_, bool>(14)
    {
        return Err(MatterConsumerProjectionStoreError::ExistingRowConflict);
    }
    require_reviewed_projection_factorisations(config, &projection)?;
    Ok(projection)
}

pub fn persist_same_matter_informativeness(
    config: &DatabaseConfig,
    value: &SameMatterInformativenessReceipt,
) -> Result<SameMatterInformativenessReceipt, MatterConsumerProjectionStoreError> {
    SameMatterInformativenessReceipt::new(
        value.left.clone(),
        value.right.clone(),
        value.informativeness.clone(),
        value.receipt_ref.clone(),
    )
    .map_err(|_| MatterConsumerProjectionStoreError::InvalidProjection)?;

    require_reviewed_informativeness_factorisation(config, value)?;
    let left = persist_matter_representation_projection(config, &value.left)?;
    let right = persist_matter_representation_projection(config, &value.right)?;
    let right_distinctions = vec_from_set(&value.informativeness.right_preserved_distinction_refs);
    let left_distinctions = vec_from_set(&value.informativeness.left_preserved_distinction_refs);

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO consumer_projection.same_matter_informativeness
           (receipt_ref,matter_ref,controversy_ref,left_projection_receipt_ref,
            right_projection_receipt_ref,right_preserved_distinction_refs,
            left_preserved_distinction_refs,reviewed_factorisation_receipt_ref,
            creates_universal_superiority,promotes_claim_truth,creates_new_matter,
            creates_semantic_authority,determines_credibility,enters_final_judgment)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(receipt_ref) DO NOTHING"#,
        &[
            &value.receipt_ref,
            &left.matter_ref,
            &left.controversy_ref,
            &left.projection_receipt_ref,
            &right.projection_receipt_ref,
            &right_distinctions,
            &left_distinctions,
            &value.informativeness.reviewed_factorisation_receipt_ref,
        ],
    )?;

    let reopened = load_same_matter_informativeness(config, &value.receipt_ref)?;
    if reopened != *value {
        return Err(MatterConsumerProjectionStoreError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_same_matter_informativeness(
    config: &DatabaseConfig,
    receipt_ref: &str,
) -> Result<SameMatterInformativenessReceipt, MatterConsumerProjectionStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT matter_ref,controversy_ref,left_projection_receipt_ref,
                      right_projection_receipt_ref,right_preserved_distinction_refs,
                      left_preserved_distinction_refs,reviewed_factorisation_receipt_ref,
                      creates_universal_superiority,promotes_claim_truth,creates_new_matter,
                      creates_semantic_authority,determines_credibility,enters_final_judgment
               FROM consumer_projection.same_matter_informativeness
               WHERE receipt_ref=$1"#,
            &[&receipt_ref],
        )?
        .ok_or_else(|| MatterConsumerProjectionStoreError::NotFound(receipt_ref.to_owned()))?;

    let matter_ref: String = row.get(0);
    let controversy_ref: String = row.get(1);
    require_persisted_matter_binding(config, &matter_ref, &controversy_ref)?;
    let left_ref: String = row.get(2);
    let right_ref: String = row.get(3);
    let left = load_matter_representation_projection(config, &left_ref)?;
    let right = load_matter_representation_projection(config, &right_ref)?;
    if left.matter_ref != matter_ref
        || right.matter_ref != matter_ref
        || left.controversy_ref != controversy_ref
        || right.controversy_ref != controversy_ref
    {
        return Err(MatterConsumerProjectionStoreError::ExistingRowConflict);
    }

    let informativeness = ConsumerInformativenessReceipt::new(
        left.adequacy.clone(),
        right.adequacy.clone(),
        row.get::<_, Vec<String>>(4),
        row.get::<_, Vec<String>>(5),
        row.get::<_, String>(6),
    )
    .map_err(|_| MatterConsumerProjectionStoreError::InvalidProjection)?;

    if row.get::<_, bool>(7)
        || row.get::<_, bool>(8)
        || row.get::<_, bool>(9)
        || row.get::<_, bool>(10)
        || row.get::<_, bool>(11)
        || row.get::<_, bool>(12)
    {
        return Err(MatterConsumerProjectionStoreError::ExistingRowConflict);
    }

    let value = SameMatterInformativenessReceipt::new(left, right, informativeness, receipt_ref.to_owned())
        .map_err(|_| MatterConsumerProjectionStoreError::InvalidProjection)?;
    require_reviewed_informativeness_factorisation(config, &value)?;
    Ok(value)
}
