use postgres::{Client, NoTls};
use sensiblaw_core::chronology_contestation::{
    ClaimLeaf, ClaimLeafKind, ClaimReviewState, ContestationRelation, ContestationRelationKind,
    PropositionRoot, TemporalAssertion, TemporalForm,
};
use thiserror::Error;

use crate::DatabaseConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventTemporalLink {
    pub event_ref: String,
    pub temporal_ref: String,
    pub assembly_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventClaimLink {
    pub event_ref: String,
    pub claim_ref: String,
    pub assembly_receipt_ref: String,
}

#[derive(Debug, Error)]
pub enum ChronologyContestationStoreError {
    #[error("invalid chronology/contestation object")]
    InvalidDomainObject,
    #[error("required persistence coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("persisted row conflicts with requested immutable coordinates")]
    ExistingRowConflict,
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

fn require(name: &'static str, value: &str) -> Result<(), ChronologyContestationStoreError> {
    if value.trim().is_empty() {
        Err(ChronologyContestationStoreError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn temporal_form_row(form: &TemporalForm) -> (&'static str, Option<&str>, Option<&str>) {
    match form {
        TemporalForm::ExactInstant { instant_ref } => ("exact_instant", Some(instant_ref), None),
        TemporalForm::ExactDate { date_ref } => ("exact_date", Some(date_ref), None),
        TemporalForm::Interval { start_ref, end_ref } => {
            ("interval", Some(start_ref), Some(end_ref))
        }
        TemporalForm::Approximate { label } => ("approximate", Some(label), None),
        TemporalForm::RelativeBefore { event_ref } => ("relative_before", Some(event_ref), None),
        TemporalForm::RelativeAfter { event_ref } => ("relative_after", Some(event_ref), None),
        TemporalForm::Contemporaneous { event_ref } => {
            ("contemporaneous", Some(event_ref), None)
        }
        TemporalForm::Undated => ("undated", None, None),
        TemporalForm::Unknown => ("unknown", None, None),
    }
}

fn temporal_form_from_row(
    kind: &str,
    a: Option<String>,
    b: Option<String>,
) -> Result<TemporalForm, ChronologyContestationStoreError> {
    let need = |value: Option<String>| {
        value.ok_or(ChronologyContestationStoreError::ExistingRowConflict)
    };
    Ok(match kind {
        "exact_instant" => TemporalForm::ExactInstant {
            instant_ref: need(a)?,
        },
        "exact_date" => TemporalForm::ExactDate { date_ref: need(a)? },
        "interval" => TemporalForm::Interval {
            start_ref: need(a)?,
            end_ref: need(b)?,
        },
        "approximate" => TemporalForm::Approximate { label: need(a)? },
        "relative_before" => TemporalForm::RelativeBefore {
            event_ref: need(a)?,
        },
        "relative_after" => TemporalForm::RelativeAfter {
            event_ref: need(a)?,
        },
        "contemporaneous" => TemporalForm::Contemporaneous {
            event_ref: need(a)?,
        },
        "undated" => TemporalForm::Undated,
        "unknown" => TemporalForm::Unknown,
        _ => return Err(ChronologyContestationStoreError::ExistingRowConflict),
    })
}

fn claim_kind_db(kind: ClaimLeafKind) -> &'static str {
    match kind {
        ClaimLeafKind::Affirmation => "affirmation",
        ClaimLeafKind::Denial => "denial",
        ClaimLeafKind::Qualification => "qualification",
        ClaimLeafKind::AlternativeAccount => "alternative_account",
    }
}

fn claim_kind_from_db(value: &str) -> Result<ClaimLeafKind, ChronologyContestationStoreError> {
    match value {
        "affirmation" => Ok(ClaimLeafKind::Affirmation),
        "denial" => Ok(ClaimLeafKind::Denial),
        "qualification" => Ok(ClaimLeafKind::Qualification),
        "alternative_account" => Ok(ClaimLeafKind::AlternativeAccount),
        _ => Err(ChronologyContestationStoreError::ExistingRowConflict),
    }
}

fn review_state_db(state: ClaimReviewState) -> &'static str {
    match state {
        ClaimReviewState::Unreviewed => "unreviewed",
        ClaimReviewState::Accepted => "accepted",
        ClaimReviewState::Rejected => "rejected",
        ClaimReviewState::Abstained => "abstained",
        ClaimReviewState::Qualified => "qualified",
        ClaimReviewState::Superseded => "superseded",
    }
}

fn review_state_from_db(value: &str) -> Result<ClaimReviewState, ChronologyContestationStoreError> {
    match value {
        "unreviewed" => Ok(ClaimReviewState::Unreviewed),
        "accepted" => Ok(ClaimReviewState::Accepted),
        "rejected" => Ok(ClaimReviewState::Rejected),
        "abstained" => Ok(ClaimReviewState::Abstained),
        "qualified" => Ok(ClaimReviewState::Qualified),
        "superseded" => Ok(ClaimReviewState::Superseded),
        _ => Err(ChronologyContestationStoreError::ExistingRowConflict),
    }
}

fn relation_kind_db(kind: ContestationRelationKind) -> &'static str {
    match kind {
        ContestationRelationKind::Supports => "supports",
        ContestationRelationKind::Qualifies => "qualifies",
        ContestationRelationKind::Denies => "denies",
        ContestationRelationKind::Contradicts => "contradicts",
        ContestationRelationKind::Adjacent => "adjacent",
        ContestationRelationKind::Supersedes => "supersedes",
        ContestationRelationKind::SameIncidentDifferentAccount => "same_incident_different_account",
        ContestationRelationKind::UnresolvedRelation => "unresolved_relation",
    }
}

fn relation_kind_from_db(
    value: &str,
) -> Result<ContestationRelationKind, ChronologyContestationStoreError> {
    match value {
        "supports" => Ok(ContestationRelationKind::Supports),
        "qualifies" => Ok(ContestationRelationKind::Qualifies),
        "denies" => Ok(ContestationRelationKind::Denies),
        "contradicts" => Ok(ContestationRelationKind::Contradicts),
        "adjacent" => Ok(ContestationRelationKind::Adjacent),
        "supersedes" => Ok(ContestationRelationKind::Supersedes),
        "same_incident_different_account" => {
            Ok(ContestationRelationKind::SameIncidentDifferentAccount)
        }
        "unresolved_relation" => Ok(ContestationRelationKind::UnresolvedRelation),
        _ => Err(ChronologyContestationStoreError::ExistingRowConflict),
    }
}

pub fn install_chronology_contestation_schema(
    config: &DatabaseConfig,
) -> Result<(), ChronologyContestationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(
        r#"
        CREATE SCHEMA IF NOT EXISTS semantic;

        CREATE TABLE IF NOT EXISTS semantic.temporal_assertion (
          temporal_ref TEXT PRIMARY KEY,
          form_ref TEXT NOT NULL,
          coordinate_a TEXT NULL,
          coordinate_b TEXT NULL,
          review_ref TEXT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE TABLE IF NOT EXISTS semantic.temporal_statement (
          temporal_ref TEXT NOT NULL REFERENCES semantic.temporal_assertion(temporal_ref),
          statement_ref TEXT NOT NULL,
          PRIMARY KEY (temporal_ref, statement_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.temporal_observation (
          temporal_ref TEXT NOT NULL REFERENCES semantic.temporal_assertion(temporal_ref),
          observation_ref TEXT NOT NULL,
          PRIMARY KEY (temporal_ref, observation_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.event_temporal (
          event_ref TEXT NOT NULL,
          temporal_ref TEXT NOT NULL REFERENCES semantic.temporal_assertion(temporal_ref),
          assembly_receipt_ref TEXT NOT NULL,
          PRIMARY KEY (event_ref, temporal_ref)
        );

        CREATE INDEX IF NOT EXISTS event_temporal_event_idx
          ON semantic.event_temporal(event_ref);

        CREATE TABLE IF NOT EXISTS semantic.proposition_root (
          proposition_ref TEXT PRIMARY KEY,
          label TEXT NOT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE TABLE IF NOT EXISTS semantic.claim_leaf (
          claim_ref TEXT PRIMARY KEY,
          proposition_ref TEXT NOT NULL REFERENCES semantic.proposition_root(proposition_ref),
          kind_ref TEXT NOT NULL,
          speaker_ref TEXT NULL,
          review_state_ref TEXT NOT NULL,
          review_ref TEXT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE TABLE IF NOT EXISTS semantic.claim_statement (
          claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          statement_ref TEXT NOT NULL,
          PRIMARY KEY (claim_ref, statement_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.claim_observation (
          claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          observation_ref TEXT NOT NULL,
          PRIMARY KEY (claim_ref, observation_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.claim_temporal (
          claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          temporal_ref TEXT NOT NULL,
          PRIMARY KEY (claim_ref, temporal_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.claim_scope (
          claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          scope_ref TEXT NOT NULL,
          PRIMARY KEY (claim_ref, scope_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.event_claim (
          event_ref TEXT NOT NULL,
          claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          assembly_receipt_ref TEXT NOT NULL,
          PRIMARY KEY (event_ref, claim_ref)
        );

        CREATE INDEX IF NOT EXISTS event_claim_event_idx
          ON semantic.event_claim(event_ref);

        CREATE TABLE IF NOT EXISTS semantic.contestation_relation (
          relation_ref TEXT PRIMARY KEY,
          from_claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          to_claim_ref TEXT NOT NULL REFERENCES semantic.claim_leaf(claim_ref),
          kind_ref TEXT NOT NULL,
          review_ref TEXT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE TABLE IF NOT EXISTS semantic.contestation_statement (
          relation_ref TEXT NOT NULL REFERENCES semantic.contestation_relation(relation_ref),
          statement_ref TEXT NOT NULL,
          PRIMARY KEY (relation_ref, statement_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.contestation_observation (
          relation_ref TEXT NOT NULL REFERENCES semantic.contestation_relation(relation_ref),
          observation_ref TEXT NOT NULL,
          PRIMARY KEY (relation_ref, observation_ref)
        );
        "#,
    )?;
    Ok(())
}

pub fn persist_temporal_assertion(
    config: &DatabaseConfig,
    value: &TemporalAssertion,
) -> Result<TemporalAssertion, ChronologyContestationStoreError> {
    value
        .validate()
        .map_err(|_| ChronologyContestationStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let (form_ref, a, b) = temporal_form_row(&value.form);
    client.execute(
        r#"
        INSERT INTO semantic.temporal_assertion
          (temporal_ref, form_ref, coordinate_a, coordinate_b, review_ref,
           candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,true,false,false,false)
        ON CONFLICT (temporal_ref) DO NOTHING
        "#,
        &[&value.temporal_ref, &form_ref, &a, &b, &value.review_ref],
    )?;
    persist_refs(
        &mut client,
        "semantic.temporal_statement",
        "temporal_ref",
        &value.temporal_ref,
        "statement_ref",
        &value.statement_refs,
    )?;
    persist_refs(
        &mut client,
        "semantic.temporal_observation",
        "temporal_ref",
        &value.temporal_ref,
        "observation_ref",
        &value.observation_refs,
    )?;
    let loaded = load_temporal_assertion_with_client(&mut client, &value.temporal_ref)?
        .ok_or(ChronologyContestationStoreError::ExistingRowConflict)?;
    if &loaded != value {
        return Err(ChronologyContestationStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_proposition_root(
    config: &DatabaseConfig,
    value: &PropositionRoot,
) -> Result<PropositionRoot, ChronologyContestationStoreError> {
    value
        .validate()
        .map_err(|_| ChronologyContestationStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"
        INSERT INTO semantic.proposition_root
          (proposition_ref, label, candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,true,false,false,false)
        ON CONFLICT (proposition_ref) DO NOTHING
        "#,
        &[&value.proposition_ref, &value.label],
    )?;
    let row = client.query_one(
        "SELECT label, candidate_only, creates_semantic_authority,          applicability_promoted, claim_truth_promoted          FROM semantic.proposition_root WHERE proposition_ref=$1",
        &[&value.proposition_ref],
    )?;
    let loaded = PropositionRoot {
        proposition_ref: value.proposition_ref.clone(),
        label: row.get(0),
        candidate_only: row.get(1),
        creates_semantic_authority: row.get(2),
        applicability_promoted: row.get(3),
        claim_truth_promoted: row.get(4),
    };
    if &loaded != value {
        return Err(ChronologyContestationStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_claim_leaf(
    config: &DatabaseConfig,
    value: &ClaimLeaf,
) -> Result<ClaimLeaf, ChronologyContestationStoreError> {
    value
        .validate()
        .map_err(|_| ChronologyContestationStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"
        INSERT INTO semantic.claim_leaf
          (claim_ref, proposition_ref, kind_ref, speaker_ref,
           review_state_ref, review_ref, candidate_only,
           creates_semantic_authority, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,true,false,false,false)
        ON CONFLICT (claim_ref) DO NOTHING
        "#,
        &[
            &value.claim_ref,
            &value.proposition_ref,
            &claim_kind_db(value.kind),
            &value.speaker_ref,
            &review_state_db(value.review_state),
            &value.review_ref,
        ],
    )?;
    persist_refs(
        &mut client,
        "semantic.claim_statement",
        "claim_ref",
        &value.claim_ref,
        "statement_ref",
        &value.statement_refs,
    )?;
    persist_refs(
        &mut client,
        "semantic.claim_observation",
        "claim_ref",
        &value.claim_ref,
        "observation_ref",
        &value.observation_refs,
    )?;
    persist_refs(
        &mut client,
        "semantic.claim_temporal",
        "claim_ref",
        &value.claim_ref,
        "temporal_ref",
        &value.temporal_refs,
    )?;
    persist_refs(
        &mut client,
        "semantic.claim_scope",
        "claim_ref",
        &value.claim_ref,
        "scope_ref",
        &value.scope_refs,
    )?;
    let loaded = load_claim_leaf_with_client(&mut client, &value.claim_ref)?
        .ok_or(ChronologyContestationStoreError::ExistingRowConflict)?;
    if &loaded != value {
        return Err(ChronologyContestationStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_contestation_relation(
    config: &DatabaseConfig,
    value: &ContestationRelation,
) -> Result<ContestationRelation, ChronologyContestationStoreError> {
    value
        .validate()
        .map_err(|_| ChronologyContestationStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"
        INSERT INTO semantic.contestation_relation
          (relation_ref, from_claim_ref, to_claim_ref, kind_ref, review_ref,
           candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,true,false,false,false)
        ON CONFLICT (relation_ref) DO NOTHING
        "#,
        &[
            &value.relation_ref,
            &value.from_claim_ref,
            &value.to_claim_ref,
            &relation_kind_db(value.kind),
            &value.review_ref,
        ],
    )?;
    persist_refs(
        &mut client,
        "semantic.contestation_statement",
        "relation_ref",
        &value.relation_ref,
        "statement_ref",
        &value.statement_refs,
    )?;
    persist_refs(
        &mut client,
        "semantic.contestation_observation",
        "relation_ref",
        &value.relation_ref,
        "observation_ref",
        &value.observation_refs,
    )?;
    let loaded = load_contestation_relation_with_client(&mut client, &value.relation_ref)?
        .ok_or(ChronologyContestationStoreError::ExistingRowConflict)?;
    if &loaded != value {
        return Err(ChronologyContestationStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_event_temporal_link(
    config: &DatabaseConfig,
    value: &EventTemporalLink,
) -> Result<(), ChronologyContestationStoreError> {
    require("event_ref", &value.event_ref)?;
    require("temporal_ref", &value.temporal_ref)?;
    require("assembly_receipt_ref", &value.assembly_receipt_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"
        INSERT INTO semantic.event_temporal
          (event_ref, temporal_ref, assembly_receipt_ref)
        VALUES ($1,$2,$3)
        ON CONFLICT (event_ref, temporal_ref) DO NOTHING
        "#,
        &[&value.event_ref, &value.temporal_ref, &value.assembly_receipt_ref],
    )?;
    Ok(())
}

pub fn persist_event_claim_link(
    config: &DatabaseConfig,
    value: &EventClaimLink,
) -> Result<(), ChronologyContestationStoreError> {
    require("event_ref", &value.event_ref)?;
    require("claim_ref", &value.claim_ref)?;
    require("assembly_receipt_ref", &value.assembly_receipt_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"
        INSERT INTO semantic.event_claim
          (event_ref, claim_ref, assembly_receipt_ref)
        VALUES ($1,$2,$3)
        ON CONFLICT (event_ref, claim_ref) DO NOTHING
        "#,
        &[&value.event_ref, &value.claim_ref, &value.assembly_receipt_ref],
    )?;
    Ok(())
}

pub fn load_temporal_assertions_for_event(
    config: &DatabaseConfig,
    event_ref: &str,
) -> Result<Vec<TemporalAssertion>, ChronologyContestationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client.query(
        "SELECT temporal_ref FROM semantic.event_temporal          WHERE event_ref=$1 ORDER BY temporal_ref",
        &[&event_ref],
    )?;
    refs.into_iter()
        .map(|row| {
            let reference: String = row.get(0);
            load_temporal_assertion_with_client(&mut client, &reference)?
                .ok_or(ChronologyContestationStoreError::ExistingRowConflict)
        })
        .collect()
}

pub fn load_claims_for_event(
    config: &DatabaseConfig,
    event_ref: &str,
) -> Result<Vec<ClaimLeaf>, ChronologyContestationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client.query(
        "SELECT claim_ref FROM semantic.event_claim          WHERE event_ref=$1 ORDER BY claim_ref",
        &[&event_ref],
    )?;
    refs.into_iter()
        .map(|row| {
            let reference: String = row.get(0);
            load_claim_leaf_with_client(&mut client, &reference)?
                .ok_or(ChronologyContestationStoreError::ExistingRowConflict)
        })
        .collect()
}

pub fn load_contestation_relations_for_claims(
    config: &DatabaseConfig,
    claim_refs: &[String],
) -> Result<Vec<ContestationRelation>, ChronologyContestationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut relation_refs = std::collections::BTreeSet::new();
    for claim_ref in claim_refs {
        for row in client.query(
            "SELECT relation_ref FROM semantic.contestation_relation              WHERE from_claim_ref=$1 OR to_claim_ref=$1 ORDER BY relation_ref",
            &[&claim_ref],
        )? {
            relation_refs.insert(row.get::<_, String>(0));
        }
    }
    relation_refs
        .into_iter()
        .map(|reference| {
            load_contestation_relation_with_client(&mut client, &reference)?
                .ok_or(ChronologyContestationStoreError::ExistingRowConflict)
        })
        .collect()
}

fn persist_refs(
    client: &mut Client,
    table: &str,
    owner_column: &str,
    owner_ref: &str,
    value_column: &str,
    values: &[String],
) -> Result<(), ChronologyContestationStoreError> {
    let sql = format!(
        "INSERT INTO {table} ({owner_column}, {value_column}) VALUES ($1,$2) ON CONFLICT DO NOTHING"
    );
    for value in values {
        client.execute(&sql, &[&owner_ref, value])?;
    }
    Ok(())
}

fn load_refs(
    client: &mut Client,
    table: &str,
    owner_column: &str,
    owner_ref: &str,
    value_column: &str,
) -> Result<Vec<String>, ChronologyContestationStoreError> {
    let sql = format!(
        "SELECT {value_column} FROM {table} WHERE {owner_column}=$1 ORDER BY {value_column}"
    );
    Ok(client
        .query(&sql, &[&owner_ref])?
        .into_iter()
        .map(|row| row.get(0))
        .collect())
}

fn load_temporal_assertion_with_client(
    client: &mut Client,
    temporal_ref: &str,
) -> Result<Option<TemporalAssertion>, ChronologyContestationStoreError> {
    let row = client.query_opt(
        "SELECT form_ref, coordinate_a, coordinate_b, review_ref,          candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted          FROM semantic.temporal_assertion WHERE temporal_ref=$1",
        &[&temporal_ref],
    )?;
    let Some(row) = row else { return Ok(None); };
    let form = temporal_form_from_row(
        row.get::<_, String>(0).as_str(),
        row.get(1),
        row.get(2),
    )?;
    Ok(Some(TemporalAssertion {
        temporal_ref: temporal_ref.to_owned(),
        form,
        statement_refs: load_refs(
            client,
            "semantic.temporal_statement",
            "temporal_ref",
            temporal_ref,
            "statement_ref",
        )?,
        observation_refs: load_refs(
            client,
            "semantic.temporal_observation",
            "temporal_ref",
            temporal_ref,
            "observation_ref",
        )?,
        review_ref: row.get(3),
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    }))
}

fn load_claim_leaf_with_client(
    client: &mut Client,
    claim_ref: &str,
) -> Result<Option<ClaimLeaf>, ChronologyContestationStoreError> {
    let row = client.query_opt(
        "SELECT proposition_ref, kind_ref, speaker_ref, review_state_ref, review_ref,          candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted          FROM semantic.claim_leaf WHERE claim_ref=$1",
        &[&claim_ref],
    )?;
    let Some(row) = row else { return Ok(None); };
    Ok(Some(ClaimLeaf {
        claim_ref: claim_ref.to_owned(),
        proposition_ref: row.get(0),
        kind: claim_kind_from_db(row.get::<_, String>(1).as_str())?,
        speaker_ref: row.get(2),
        statement_refs: load_refs(
            client,
            "semantic.claim_statement",
            "claim_ref",
            claim_ref,
            "statement_ref",
        )?,
        observation_refs: load_refs(
            client,
            "semantic.claim_observation",
            "claim_ref",
            claim_ref,
            "observation_ref",
        )?,
        temporal_refs: load_refs(
            client,
            "semantic.claim_temporal",
            "claim_ref",
            claim_ref,
            "temporal_ref",
        )?,
        scope_refs: load_refs(
            client,
            "semantic.claim_scope",
            "claim_ref",
            claim_ref,
            "scope_ref",
        )?,
        review_state: review_state_from_db(row.get::<_, String>(3).as_str())?,
        review_ref: row.get(4),
        candidate_only: row.get(5),
        creates_semantic_authority: row.get(6),
        applicability_promoted: row.get(7),
        claim_truth_promoted: row.get(8),
    }))
}

fn load_contestation_relation_with_client(
    client: &mut Client,
    relation_ref: &str,
) -> Result<Option<ContestationRelation>, ChronologyContestationStoreError> {
    let row = client.query_opt(
        "SELECT from_claim_ref, to_claim_ref, kind_ref, review_ref,          candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted          FROM semantic.contestation_relation WHERE relation_ref=$1",
        &[&relation_ref],
    )?;
    let Some(row) = row else { return Ok(None); };
    Ok(Some(ContestationRelation {
        relation_ref: relation_ref.to_owned(),
        from_claim_ref: row.get(0),
        to_claim_ref: row.get(1),
        kind: relation_kind_from_db(row.get::<_, String>(2).as_str())?,
        statement_refs: load_refs(
            client,
            "semantic.contestation_statement",
            "relation_ref",
            relation_ref,
            "statement_ref",
        )?,
        observation_refs: load_refs(
            client,
            "semantic.contestation_observation",
            "relation_ref",
            relation_ref,
            "observation_ref",
        )?,
        review_ref: row.get(3),
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_temporal_forms_have_distinct_storage_tags() {
        let forms = [
            TemporalForm::ExactInstant { instant_ref: "t".into() },
            TemporalForm::ExactDate { date_ref: "d".into() },
            TemporalForm::Interval { start_ref: "a".into(), end_ref: "b".into() },
            TemporalForm::Approximate { label: "~mid".into() },
            TemporalForm::RelativeBefore { event_ref: "e".into() },
            TemporalForm::RelativeAfter { event_ref: "e".into() },
            TemporalForm::Contemporaneous { event_ref: "e".into() },
            TemporalForm::Undated,
            TemporalForm::Unknown,
        ];
        let tags = forms
            .iter()
            .map(|form| temporal_form_row(form).0)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(tags.len(), forms.len());
    }

    #[test]
    fn claim_and_relation_kinds_round_trip_through_storage_tags() {
        for kind in [
            ClaimLeafKind::Affirmation,
            ClaimLeafKind::Denial,
            ClaimLeafKind::Qualification,
            ClaimLeafKind::AlternativeAccount,
        ] {
            assert_eq!(claim_kind_from_db(claim_kind_db(kind)).unwrap(), kind);
        }
        for kind in [
            ContestationRelationKind::Supports,
            ContestationRelationKind::Qualifies,
            ContestationRelationKind::Denies,
            ContestationRelationKind::Contradicts,
            ContestationRelationKind::Adjacent,
            ContestationRelationKind::Supersedes,
            ContestationRelationKind::SameIncidentDifferentAccount,
            ContestationRelationKind::UnresolvedRelation,
        ] {
            assert_eq!(relation_kind_from_db(relation_kind_db(kind)).unwrap(), kind);
        }
    }
}
