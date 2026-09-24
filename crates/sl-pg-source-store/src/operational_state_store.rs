use std::{fs, path::Path};

use postgres::{Client, NoTls};
use serde::Deserialize;
use thiserror::Error;

use sensiblaw_core::operational_state::{
    OperationalEvent, OperationalEventKind, OperationalSemanticLink,
    OperationalSemanticRelationKind, OperationalTargetKind,
};

use crate::DatabaseConfig;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StatiBakerActivityLedger {
    pub activity_events: Vec<StatiBakerActivityEvent>,
    pub provenance: StatiBakerLedgerProvenance,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct StatiBakerLedgerProvenance {
    pub algorithm: String,
    pub input_hash: String,
    #[serde(default)]
    pub policy_receipt: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct StatiBakerActivityEvent {
    pub id: String,
    pub t_start: String,
    pub t_end: String,
    #[serde(default)]
    pub snapshot_ids: Vec<String>,
    #[serde(default)]
    pub primary_app: String,
    pub title: String,
    #[serde(default)]
    pub key_text: Vec<String>,
    #[serde(default)]
    pub thumb_snapshot_id: String,
    pub confidence: f64,
    #[serde(default)]
    pub policy_flags: Vec<String>,
    #[serde(default)]
    pub derived_from: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatiBakerOperationalImportReceipt {
    pub state_date: String,
    pub producer_algorithm: String,
    pub producer_input_hash: String,
    pub event_count: usize,
    pub operational_event_refs: Vec<String>,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum OperationalStateStoreError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid operational event")]
    InvalidOperationalEvent,
    #[error("invalid operational-semantic link")]
    InvalidOperationalLink,
    #[error("invalid StatiBaker activity ledger")]
    InvalidStatiBakerLedger,
    #[error("existing operational event conflicts with producer event")]
    ExistingOperationalEventConflict,
    #[error("missing operational event for semantic link")]
    MissingOperationalEvent,
}

fn operational_kind_as_db(kind: OperationalEventKind) -> &'static str {
    match kind {
        OperationalEventKind::Session => "session",
        OperationalEventKind::ToolActivity => "tool_activity",
        OperationalEventKind::ResearchActivity => "research_activity",
        OperationalEventKind::ReviewAction => "review_action",
        OperationalEventKind::GitCommit => "git_commit",
        OperationalEventKind::PullRequest => "pull_request",
        OperationalEventKind::TaskTransition => "task_transition",
        OperationalEventKind::Interruption => "interruption",
        OperationalEventKind::BrowserActivity => "browser_activity",
        OperationalEventKind::EditorActivity => "editor_activity",
        OperationalEventKind::CommunicationActivity => "communication_activity",
        OperationalEventKind::Other => "other",
    }
}

fn operational_kind_from_db(value: &str) -> Result<OperationalEventKind, OperationalStateStoreError> {
    match value {
        "session" => Ok(OperationalEventKind::Session),
        "tool_activity" => Ok(OperationalEventKind::ToolActivity),
        "research_activity" => Ok(OperationalEventKind::ResearchActivity),
        "review_action" => Ok(OperationalEventKind::ReviewAction),
        "git_commit" => Ok(OperationalEventKind::GitCommit),
        "pull_request" => Ok(OperationalEventKind::PullRequest),
        "task_transition" => Ok(OperationalEventKind::TaskTransition),
        "interruption" => Ok(OperationalEventKind::Interruption),
        "browser_activity" => Ok(OperationalEventKind::BrowserActivity),
        "editor_activity" => Ok(OperationalEventKind::EditorActivity),
        "communication_activity" => Ok(OperationalEventKind::CommunicationActivity),
        "other" => Ok(OperationalEventKind::Other),
        _ => Err(OperationalStateStoreError::InvalidOperationalEvent),
    }
}

fn target_kind_as_db(kind: OperationalTargetKind) -> &'static str {
    match kind {
        OperationalTargetKind::Matter => "matter",
        OperationalTargetKind::Source => "source",
        OperationalTargetKind::Statement => "statement",
        OperationalTargetKind::ParserReceipt => "parser_receipt",
        OperationalTargetKind::Observation => "observation",
        OperationalTargetKind::SemanticEvent => "semantic_event",
        OperationalTargetKind::Proposition => "proposition",
        OperationalTargetKind::Claim => "claim",
        OperationalTargetKind::Authority => "authority",
        OperationalTargetKind::ReviewItem => "review_item",
        OperationalTargetKind::Artifact => "artifact",
        OperationalTargetKind::ResearchResidual => "research_residual",
    }
}

fn target_kind_from_db(value: &str) -> Result<OperationalTargetKind, OperationalStateStoreError> {
    match value {
        "matter" => Ok(OperationalTargetKind::Matter),
        "source" => Ok(OperationalTargetKind::Source),
        "statement" => Ok(OperationalTargetKind::Statement),
        "parser_receipt" => Ok(OperationalTargetKind::ParserReceipt),
        "observation" => Ok(OperationalTargetKind::Observation),
        "semantic_event" => Ok(OperationalTargetKind::SemanticEvent),
        "proposition" => Ok(OperationalTargetKind::Proposition),
        "claim" => Ok(OperationalTargetKind::Claim),
        "authority" => Ok(OperationalTargetKind::Authority),
        "review_item" => Ok(OperationalTargetKind::ReviewItem),
        "artifact" => Ok(OperationalTargetKind::Artifact),
        "research_residual" => Ok(OperationalTargetKind::ResearchResidual),
        _ => Err(OperationalStateStoreError::InvalidOperationalLink),
    }
}

fn relation_kind_as_db(kind: OperationalSemanticRelationKind) -> &'static str {
    match kind {
        OperationalSemanticRelationKind::Opened => "opened",
        OperationalSemanticRelationKind::Observed => "observed",
        OperationalSemanticRelationKind::Edited => "edited",
        OperationalSemanticRelationKind::Parsed => "parsed",
        OperationalSemanticRelationKind::Reviewed => "reviewed",
        OperationalSemanticRelationKind::Researched => "researched",
        OperationalSemanticRelationKind::FollowedAuthority => "followed_authority",
        OperationalSemanticRelationKind::AcceptedReview => "accepted_review",
        OperationalSemanticRelationKind::RejectedReview => "rejected_review",
        OperationalSemanticRelationKind::QualifiedReview => "qualified_review",
        OperationalSemanticRelationKind::RequestedEvidence => "requested_evidence",
        OperationalSemanticRelationKind::ProducedArtifact => "produced_artifact",
        OperationalSemanticRelationKind::CommittedChange => "committed_change",
        OperationalSemanticRelationKind::Affected => "affected",
    }
}

fn relation_kind_from_db(
    value: &str,
) -> Result<OperationalSemanticRelationKind, OperationalStateStoreError> {
    match value {
        "opened" => Ok(OperationalSemanticRelationKind::Opened),
        "observed" => Ok(OperationalSemanticRelationKind::Observed),
        "edited" => Ok(OperationalSemanticRelationKind::Edited),
        "parsed" => Ok(OperationalSemanticRelationKind::Parsed),
        "reviewed" => Ok(OperationalSemanticRelationKind::Reviewed),
        "researched" => Ok(OperationalSemanticRelationKind::Researched),
        "followed_authority" => Ok(OperationalSemanticRelationKind::FollowedAuthority),
        "accepted_review" => Ok(OperationalSemanticRelationKind::AcceptedReview),
        "rejected_review" => Ok(OperationalSemanticRelationKind::RejectedReview),
        "qualified_review" => Ok(OperationalSemanticRelationKind::QualifiedReview),
        "requested_evidence" => Ok(OperationalSemanticRelationKind::RequestedEvidence),
        "produced_artifact" => Ok(OperationalSemanticRelationKind::ProducedArtifact),
        "committed_change" => Ok(OperationalSemanticRelationKind::CommittedChange),
        "affected" => Ok(OperationalSemanticRelationKind::Affected),
        _ => Err(OperationalStateStoreError::InvalidOperationalLink),
    }
}

pub fn load_statibaker_activity_ledger(
    path: impl AsRef<Path>,
) -> Result<StatiBakerActivityLedger, OperationalStateStoreError> {
    let raw = fs::read_to_string(path)?;
    let ledger = serde_json::from_str::<StatiBakerActivityLedger>(&raw)?;
    validate_statibaker_ledger(&ledger)?;
    Ok(ledger)
}

pub fn validate_statibaker_ledger(
    ledger: &StatiBakerActivityLedger,
) -> Result<(), OperationalStateStoreError> {
    if ledger.provenance.algorithm.trim().is_empty()
        || ledger.provenance.input_hash.trim().is_empty()
        || !ledger.provenance.algorithm.starts_with("sb.sessionize.")
    {
        return Err(OperationalStateStoreError::InvalidStatiBakerLedger);
    }
    for event in &ledger.activity_events {
        if event.id.trim().is_empty()
            || event.t_start.trim().is_empty()
            || event.t_end.trim().is_empty()
            || event.title.trim().is_empty()
            || !(0.0..=1.0).contains(&event.confidence)
        {
            return Err(OperationalStateStoreError::InvalidStatiBakerLedger);
        }
        if event.snapshot_ids.iter().any(|value| value.trim().is_empty())
            || event.derived_from.iter().any(|value| value.trim().is_empty())
            || event.policy_flags.iter().any(|value| value.trim().is_empty())
        {
            return Err(OperationalStateStoreError::InvalidStatiBakerLedger);
        }
    }
    Ok(())
}

pub fn install_operational_state_schema(
    config: &DatabaseConfig,
) -> Result<(), OperationalStateStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(
        r#"
        CREATE SCHEMA IF NOT EXISTS operational;

        CREATE TABLE IF NOT EXISTS operational.event (
          operational_event_ref TEXT PRIMARY KEY,
          producer_event_ref TEXT NOT NULL,
          producer_ref TEXT NOT NULL,
          state_date TEXT NOT NULL,
          start_time_ref TEXT NOT NULL,
          end_time_ref TEXT NOT NULL,
          primary_app_ref TEXT,
          label TEXT NOT NULL,
          kind_ref TEXT NOT NULL,
          producer_observed BOOLEAN NOT NULL CHECK (producer_observed),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          pays_evidence BOOLEAN NOT NULL CHECK (NOT pays_evidence),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
          UNIQUE (producer_ref, producer_event_ref, state_date)
        );

        CREATE TABLE IF NOT EXISTS operational.event_provenance (
          operational_event_ref TEXT NOT NULL REFERENCES operational.event(operational_event_ref) ON DELETE CASCADE,
          ordinal INTEGER NOT NULL,
          provenance_ref TEXT NOT NULL,
          PRIMARY KEY (operational_event_ref, provenance_ref)
        );

        CREATE TABLE IF NOT EXISTS operational.statibaker_event_detail (
          operational_event_ref TEXT PRIMARY KEY REFERENCES operational.event(operational_event_ref) ON DELETE CASCADE,
          algorithm_ref TEXT NOT NULL,
          input_hash TEXT NOT NULL,
          policy_receipt TEXT NOT NULL,
          confidence DOUBLE PRECISION NOT NULL,
          thumb_snapshot_id TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS operational.statibaker_snapshot_ref (
          operational_event_ref TEXT NOT NULL REFERENCES operational.event(operational_event_ref) ON DELETE CASCADE,
          ordinal INTEGER NOT NULL,
          snapshot_ref TEXT NOT NULL,
          derived_from BOOLEAN NOT NULL,
          PRIMARY KEY (operational_event_ref, snapshot_ref, derived_from)
        );

        CREATE TABLE IF NOT EXISTS operational.statibaker_policy_flag (
          operational_event_ref TEXT NOT NULL REFERENCES operational.event(operational_event_ref) ON DELETE CASCADE,
          ordinal INTEGER NOT NULL,
          policy_flag TEXT NOT NULL,
          PRIMARY KEY (operational_event_ref, policy_flag)
        );

        CREATE TABLE IF NOT EXISTS operational.semantic_link (
          link_ref TEXT PRIMARY KEY,
          operational_event_ref TEXT NOT NULL REFERENCES operational.event(operational_event_ref) ON DELETE CASCADE,
          target_ref TEXT NOT NULL,
          target_kind_ref TEXT NOT NULL,
          relation_kind_ref TEXT NOT NULL,
          relationship_receipt_ref TEXT NOT NULL,
          reviewed_link BOOLEAN NOT NULL CHECK (reviewed_link),
          creates_semantic_identity BOOLEAN NOT NULL CHECK (NOT creates_semantic_identity),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          pays_evidence BOOLEAN NOT NULL CHECK (NOT pays_evidence),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE INDEX IF NOT EXISTS operational_semantic_link_event_idx
          ON operational.semantic_link (operational_event_ref);
        CREATE INDEX IF NOT EXISTS operational_semantic_link_target_idx
          ON operational.semantic_link (target_ref);
        "#,
    )?;
    Ok(())
}

pub fn materialize_statibaker_activity_ledger(
    config: &DatabaseConfig,
    state_date: &str,
    ledger: &StatiBakerActivityLedger,
) -> Result<StatiBakerOperationalImportReceipt, OperationalStateStoreError> {
    validate_statibaker_ledger(ledger)?;
    if state_date.trim().is_empty() {
        return Err(OperationalStateStoreError::InvalidStatiBakerLedger);
    }
    install_operational_state_schema(config)?;

    let mut refs = Vec::new();
    for source_event in &ledger.activity_events {
        let operational_event_ref =
            format!("operational:statibaker:{state_date}:{}", source_event.id);
        let mut provenance_refs = vec![
            format!("statibaker-algorithm:{}", ledger.provenance.algorithm),
            format!("statibaker-input-sha256:{}", ledger.provenance.input_hash),
        ];
        if !ledger.provenance.policy_receipt.trim().is_empty() {
            provenance_refs.push(format!(
                "statibaker-policy-receipt:{}",
                ledger.provenance.policy_receipt
            ));
        }
        for snapshot in &source_event.derived_from {
            provenance_refs.push(format!("statibaker-snapshot:{snapshot}"));
        }
        provenance_refs.sort();
        provenance_refs.dedup();

        let event = OperationalEvent {
            operational_event_ref: operational_event_ref.clone(),
            producer_event_ref: source_event.id.clone(),
            producer_ref: format!("statibaker:{}", ledger.provenance.algorithm),
            state_date: state_date.to_owned(),
            start_time_ref: source_event.t_start.clone(),
            end_time_ref: source_event.t_end.clone(),
            primary_app_ref: if source_event.primary_app.trim().is_empty() {
                None
            } else {
                Some(source_event.primary_app.clone())
            },
            label: source_event.title.clone(),
            provenance_refs,
            producer_observed: true,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            kind: OperationalEventKind::Session,
        };
        persist_operational_event(config, &event)?;

        let mut client = Client::connect(config.database_url(), NoTls)?;
        let mut tx = client.transaction()?;
        tx.execute(
            r#"
            INSERT INTO operational.statibaker_event_detail
              (operational_event_ref, algorithm_ref, input_hash, policy_receipt,
               confidence, thumb_snapshot_id)
            VALUES ($1,$2,$3,$4,$5,$6)
            ON CONFLICT (operational_event_ref) DO NOTHING
            "#,
            &[
                &operational_event_ref,
                &ledger.provenance.algorithm,
                &ledger.provenance.input_hash,
                &ledger.provenance.policy_receipt,
                &source_event.confidence,
                &source_event.thumb_snapshot_id,
            ],
        )?;
        for (ordinal, snapshot) in source_event.snapshot_ids.iter().enumerate() {
            tx.execute(
                r#"
                INSERT INTO operational.statibaker_snapshot_ref
                  (operational_event_ref, ordinal, snapshot_ref, derived_from)
                VALUES ($1,$2,$3,false)
                ON CONFLICT DO NOTHING
                "#,
                &[&operational_event_ref, &(ordinal as i32), snapshot],
            )?;
        }
        for (ordinal, snapshot) in source_event.derived_from.iter().enumerate() {
            tx.execute(
                r#"
                INSERT INTO operational.statibaker_snapshot_ref
                  (operational_event_ref, ordinal, snapshot_ref, derived_from)
                VALUES ($1,$2,$3,true)
                ON CONFLICT DO NOTHING
                "#,
                &[&operational_event_ref, &(ordinal as i32), snapshot],
            )?;
        }
        for (ordinal, flag) in source_event.policy_flags.iter().enumerate() {
            tx.execute(
                r#"
                INSERT INTO operational.statibaker_policy_flag
                  (operational_event_ref, ordinal, policy_flag)
                VALUES ($1,$2,$3)
                ON CONFLICT DO NOTHING
                "#,
                &[&operational_event_ref, &(ordinal as i32), flag],
            )?;
        }
        tx.commit()?;
        refs.push(operational_event_ref);
    }
    refs.sort();

    Ok(StatiBakerOperationalImportReceipt {
        state_date: state_date.to_owned(),
        producer_algorithm: ledger.provenance.algorithm.clone(),
        producer_input_hash: ledger.provenance.input_hash.clone(),
        event_count: refs.len(),
        operational_event_refs: refs,
        creates_semantic_authority: false,
        pays_evidence: false,
        claim_truth_promoted: false,
    })
}

pub fn persist_operational_event(
    config: &DatabaseConfig,
    event: &OperationalEvent,
) -> Result<OperationalEvent, OperationalStateStoreError> {
    event
        .validate()
        .map_err(|_| OperationalStateStoreError::InvalidOperationalEvent)?;
    install_operational_state_schema(config)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"
        INSERT INTO operational.event
          (operational_event_ref, producer_event_ref, producer_ref, state_date,
           start_time_ref, end_time_ref, primary_app_ref, label, kind_ref,
           producer_observed, creates_semantic_authority, pays_evidence,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,true,false,false,false,false)
        ON CONFLICT (operational_event_ref) DO NOTHING
        "#,
        &[
            &event.operational_event_ref,
            &event.producer_event_ref,
            &event.producer_ref,
            &event.state_date,
            &event.start_time_ref,
            &event.end_time_ref,
            &event.primary_app_ref,
            &event.label,
            &operational_kind_as_db(event.kind),
        ],
    )?;
    for (ordinal, provenance) in event.provenance_refs.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO operational.event_provenance
              (operational_event_ref, ordinal, provenance_ref)
            VALUES ($1,$2,$3)
            ON CONFLICT DO NOTHING
            "#,
            &[&event.operational_event_ref, &(ordinal as i32), provenance],
        )?;
    }
    tx.commit()?;

    let persisted = load_operational_event(config, &event.operational_event_ref)?
        .ok_or(OperationalStateStoreError::ExistingOperationalEventConflict)?;
    if &persisted != event {
        return Err(OperationalStateStoreError::ExistingOperationalEventConflict);
    }
    Ok(persisted)
}

pub fn persist_operational_semantic_link(
    config: &DatabaseConfig,
    link: &OperationalSemanticLink,
) -> Result<OperationalSemanticLink, OperationalStateStoreError> {
    link.validate()
        .map_err(|_| OperationalStateStoreError::InvalidOperationalLink)?;
    install_operational_state_schema(config)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let exists: bool = client
        .query_one(
            "SELECT EXISTS (SELECT 1 FROM operational.event WHERE operational_event_ref=$1)",
            &[&link.operational_event_ref],
        )?
        .get(0);
    if !exists {
        return Err(OperationalStateStoreError::MissingOperationalEvent);
    }

    client.execute(
        r#"
        INSERT INTO operational.semantic_link
          (link_ref, operational_event_ref, target_ref, target_kind_ref,
           relation_kind_ref, relationship_receipt_ref, reviewed_link,
           creates_semantic_identity, creates_semantic_authority, pays_evidence,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,true,false,false,false,false,false)
        ON CONFLICT (link_ref) DO NOTHING
        "#,
        &[
            &link.link_ref,
            &link.operational_event_ref,
            &link.target_ref,
            &target_kind_as_db(link.target_kind),
            &relation_kind_as_db(link.relation_kind),
            &link.relationship_receipt_ref,
        ],
    )?;

    Ok(link.clone())
}

pub fn load_operational_event(
    config: &DatabaseConfig,
    operational_event_ref: &str,
) -> Result<Option<OperationalEvent>, OperationalStateStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let Some(row) = client.query_opt(
        r#"
        SELECT operational_event_ref, producer_event_ref, producer_ref,
               state_date, start_time_ref, end_time_ref, primary_app_ref,
               label, kind_ref, producer_observed, creates_semantic_authority,
               pays_evidence, applicability_promoted, claim_truth_promoted
        FROM operational.event WHERE operational_event_ref=$1
        "#,
        &[&operational_event_ref],
    )? else {
        return Ok(None);
    };
    let provenance_refs = client
        .query(
            r#"
            SELECT provenance_ref FROM operational.event_provenance
            WHERE operational_event_ref=$1 ORDER BY ordinal, provenance_ref
            "#,
            &[&operational_event_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    let event = OperationalEvent {
        operational_event_ref: row.get(0),
        producer_event_ref: row.get(1),
        producer_ref: row.get(2),
        state_date: row.get(3),
        start_time_ref: row.get(4),
        end_time_ref: row.get(5),
        primary_app_ref: row.get(6),
        label: row.get(7),
        kind: operational_kind_from_db(row.get::<_, String>(8).as_str())?,
        producer_observed: row.get(9),
        creates_semantic_authority: row.get(10),
        pays_evidence: row.get(11),
        applicability_promoted: row.get(12),
        claim_truth_promoted: row.get(13),
        provenance_refs,
    };
    event
        .validate()
        .map_err(|_| OperationalStateStoreError::InvalidOperationalEvent)?;
    Ok(Some(event))
}

pub fn load_operational_events_for_date(
    config: &DatabaseConfig,
    state_date: &str,
) -> Result<Vec<OperationalEvent>, OperationalStateStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client
        .query(
            r#"
            SELECT operational_event_ref FROM operational.event
            WHERE state_date=$1 ORDER BY start_time_ref, operational_event_ref
            "#,
            &[&state_date],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    refs.into_iter()
        .map(|reference| {
            load_operational_event(config, &reference)?
                .ok_or(OperationalStateStoreError::ExistingOperationalEventConflict)
        })
        .collect()
}

pub fn load_operational_semantic_links_for_event(
    config: &DatabaseConfig,
    operational_event_ref: &str,
) -> Result<Vec<OperationalSemanticLink>, OperationalStateStoreError> {
    load_operational_semantic_links(config, "operational_event_ref", operational_event_ref)
}

pub fn load_operational_semantic_links_for_target(
    config: &DatabaseConfig,
    target_ref: &str,
) -> Result<Vec<OperationalSemanticLink>, OperationalStateStoreError> {
    load_operational_semantic_links(config, "target_ref", target_ref)
}

fn load_operational_semantic_links(
    config: &DatabaseConfig,
    column: &str,
    value: &str,
) -> Result<Vec<OperationalSemanticLink>, OperationalStateStoreError> {
    let where_clause = match column {
        "operational_event_ref" => "operational_event_ref=$1",
        "target_ref" => "target_ref=$1",
        _ => unreachable!("fixed operational link query"),
    };
    let sql = format!(
        "SELECT link_ref, operational_event_ref, target_ref, target_kind_ref,          relation_kind_ref, relationship_receipt_ref, reviewed_link,          creates_semantic_identity, creates_semantic_authority, pays_evidence,          applicability_promoted, claim_truth_promoted          FROM operational.semantic_link WHERE {where_clause} ORDER BY link_ref"
    );
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client
        .query(&sql, &[&value])?
        .into_iter()
        .map(|row| {
            let link = OperationalSemanticLink {
                link_ref: row.get(0),
                operational_event_ref: row.get(1),
                target_ref: row.get(2),
                target_kind: target_kind_from_db(row.get::<_, String>(3).as_str())?,
                relation_kind: relation_kind_from_db(row.get::<_, String>(4).as_str())?,
                relationship_receipt_ref: row.get(5),
                reviewed_link: row.get(6),
                creates_semantic_identity: row.get(7),
                creates_semantic_authority: row.get(8),
                pays_evidence: row.get(9),
                applicability_promoted: row.get(10),
                claim_truth_promoted: row.get(11),
            };
            link.validate()
                .map_err(|_| OperationalStateStoreError::InvalidOperationalLink)?;
            Ok(link)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sessionizer_ledger_shape_maps_without_semantic_promotion() {
        let ledger = StatiBakerActivityLedger {
            activity_events: vec![StatiBakerActivityEvent {
                id: "act-000001".into(),
                t_start: "2026-09-24T14:01:00+10:00".into(),
                t_end: "2026-09-24T14:07:00+10:00".into(),
                snapshot_ids: vec!["snap:1".into(), "snap:2".into()],
                primary_app: "chatgpt".into(),
                title: "research".into(),
                key_text: vec![],
                thumb_snapshot_id: "snap:2".into(),
                confidence: 0.8,
                policy_flags: vec![],
                derived_from: vec!["snap:1".into(), "snap:2".into()],
            }],
            provenance: StatiBakerLedgerProvenance {
                algorithm: "sb.sessionize.v0".into(),
                input_hash: "abc123".into(),
                policy_receipt: "policy:1".into(),
            },
        };
        validate_statibaker_ledger(&ledger).unwrap();
        assert_eq!(ledger.activity_events.len(), 1);
    }

    #[test]
    fn non_sessionizer_algorithm_fails_closed() {
        let ledger = StatiBakerActivityLedger {
            activity_events: vec![],
            provenance: StatiBakerLedgerProvenance {
                algorithm: "unknown".into(),
                input_hash: "abc123".into(),
                policy_receipt: String::new(),
            },
        };
        assert!(matches!(
            validate_statibaker_ledger(&ledger),
            Err(OperationalStateStoreError::InvalidStatiBakerLedger)
        ));
    }
}
