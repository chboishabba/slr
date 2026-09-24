use postgres::{Client, GenericClient, NoTls};
use sensiblaw_core::review_workstation::{
    apply_review_command, ReviewAction, ReviewCommand, ReviewEffect, ReviewItem,
    ReviewItemKind, ReviewReceipt, ReviewStatus,
};
use thiserror::Error;

use crate::DatabaseConfig;

#[derive(Debug, Error)]
pub enum ReviewWorkstationStoreError {
    #[error("invalid review workstation object")]
    InvalidDomainObject,
    #[error("persisted review row conflicts with requested coordinates")]
    ExistingRowConflict,
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
}

fn kind_db(kind: ReviewItemKind) -> &'static str {
    match kind {
        ReviewItemKind::PnfParse => "pnf_parse",
        ReviewItemKind::Observation => "observation",
        ReviewItemKind::ClaimContestation => "claim_contestation",
        ReviewItemKind::EventAssembly => "event_assembly",
        ReviewItemKind::ChronologyAmbiguity => "chronology_ambiguity",
        ReviewItemKind::AuthorityFollow => "authority_follow",
        ReviewItemKind::ResearchAcquisition => "research_acquisition",
        ReviewItemKind::LegalTreatment => "legal_treatment",
        ReviewItemKind::ScopeHandoff => "scope_handoff",
    }
}

fn kind_from_db(value: &str) -> Result<ReviewItemKind, ReviewWorkstationStoreError> {
    match value {
        "pnf_parse" => Ok(ReviewItemKind::PnfParse),
        "observation" => Ok(ReviewItemKind::Observation),
        "claim_contestation" => Ok(ReviewItemKind::ClaimContestation),
        "event_assembly" => Ok(ReviewItemKind::EventAssembly),
        "chronology_ambiguity" => Ok(ReviewItemKind::ChronologyAmbiguity),
        "authority_follow" => Ok(ReviewItemKind::AuthorityFollow),
        "research_acquisition" => Ok(ReviewItemKind::ResearchAcquisition),
        "legal_treatment" => Ok(ReviewItemKind::LegalTreatment),
        "scope_handoff" => Ok(ReviewItemKind::ScopeHandoff),
        _ => Err(ReviewWorkstationStoreError::ExistingRowConflict),
    }
}

fn status_db(status: ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::Pending => "pending",
        ReviewStatus::Accepted => "accepted",
        ReviewStatus::Rejected => "rejected",
        ReviewStatus::Abstained => "abstained",
        ReviewStatus::Qualified => "qualified",
        ReviewStatus::Superseded => "superseded",
        ReviewStatus::NeedsEvidence => "needs_evidence",
    }
}

fn status_from_db(value: &str) -> Result<ReviewStatus, ReviewWorkstationStoreError> {
    match value {
        "pending" => Ok(ReviewStatus::Pending),
        "accepted" => Ok(ReviewStatus::Accepted),
        "rejected" => Ok(ReviewStatus::Rejected),
        "abstained" => Ok(ReviewStatus::Abstained),
        "qualified" => Ok(ReviewStatus::Qualified),
        "superseded" => Ok(ReviewStatus::Superseded),
        "needs_evidence" => Ok(ReviewStatus::NeedsEvidence),
        _ => Err(ReviewWorkstationStoreError::ExistingRowConflict),
    }
}

fn action_db(action: ReviewAction) -> &'static str {
    match action {
        ReviewAction::Accept => "accept",
        ReviewAction::Reject => "reject",
        ReviewAction::Abstain => "abstain",
        ReviewAction::Qualify => "qualify",
        ReviewAction::Supersede => "supersede",
        ReviewAction::RequestEvidence => "request_evidence",
        ReviewAction::OpenSource => "open_source",
        ReviewAction::FollowAuthority => "follow_authority",
    }
}

fn action_from_db(value: &str) -> Result<ReviewAction, ReviewWorkstationStoreError> {
    match value {
        "accept" => Ok(ReviewAction::Accept),
        "reject" => Ok(ReviewAction::Reject),
        "abstain" => Ok(ReviewAction::Abstain),
        "qualify" => Ok(ReviewAction::Qualify),
        "supersede" => Ok(ReviewAction::Supersede),
        "request_evidence" => Ok(ReviewAction::RequestEvidence),
        "open_source" => Ok(ReviewAction::OpenSource),
        "follow_authority" => Ok(ReviewAction::FollowAuthority),
        _ => Err(ReviewWorkstationStoreError::ExistingRowConflict),
    }
}

fn canonicalize_string_set(values: &mut Vec<String>) {
    values.sort_unstable();
    values.dedup();
}

fn canonical_review_item_for_persistence(mut item: ReviewItem) -> ReviewItem {
    canonicalize_string_set(&mut item.provenance_refs);
    canonicalize_string_set(&mut item.source_refs);
    canonicalize_string_set(&mut item.affected_consumer_refs);
    item.available_actions.sort_by_key(|action| action_db(*action));
    item.available_actions.dedup();
    item
}

#[cfg(test)]
mod persistence_canonicalization_tests {
    use super::*;

    #[test]
    fn canonicalizes_set_valued_review_item_refs_before_round_trip_comparison() {
        let item = ReviewItem {
            review_item_ref: "review-item:1".into(),
            semantic_ref: "proposal:1".into(),
            item_kind: ReviewItemKind::EventAssembly,
            reason: "review fixture".into(),
            provenance_refs: vec!["signal:z".into(), "signal:a".into()],
            source_refs: vec!["statement:z".into(), "statement:a".into()],
            current_status: ReviewStatus::Pending,
            available_actions: vec![ReviewAction::Reject, ReviewAction::Accept],
            affected_consumer_refs: vec!["matter:z".into(), "matter:a".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };

        let canonical = canonical_review_item_for_persistence(item);

        assert_eq!(canonical.provenance_refs, ["signal:a", "signal:z"]);
        assert_eq!(canonical.source_refs, ["statement:a", "statement:z"]);
        assert_eq!(canonical.affected_consumer_refs, ["matter:a", "matter:z"]);
        assert_eq!(canonical.available_actions, [ReviewAction::Accept, ReviewAction::Reject]);
    }
}

pub fn install_review_workstation_schema(
    config: &DatabaseConfig,
) -> Result<(), ReviewWorkstationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(
        r#"
        CREATE SCHEMA IF NOT EXISTS semantic;

        CREATE TABLE IF NOT EXISTS semantic.review_item (
          review_item_ref TEXT PRIMARY KEY,
          semantic_ref TEXT NOT NULL,
          item_kind_ref TEXT NOT NULL,
          reason TEXT NOT NULL,
          current_status_ref TEXT NOT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE TABLE IF NOT EXISTS semantic.review_item_provenance (
          review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
          provenance_ref TEXT NOT NULL,
          PRIMARY KEY (review_item_ref, provenance_ref)
        );
        CREATE TABLE IF NOT EXISTS semantic.review_item_source (
          review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
          source_ref TEXT NOT NULL,
          PRIMARY KEY (review_item_ref, source_ref)
        );
        CREATE TABLE IF NOT EXISTS semantic.review_item_action (
          review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
          action_ref TEXT NOT NULL,
          PRIMARY KEY (review_item_ref, action_ref)
        );
        CREATE TABLE IF NOT EXISTS semantic.review_item_consumer (
          review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
          consumer_ref TEXT NOT NULL,
          PRIMARY KEY (review_item_ref, consumer_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.review_receipt (
          command_ref TEXT PRIMARY KEY,
          review_item_ref TEXT NOT NULL REFERENCES semantic.review_item(review_item_ref),
          semantic_ref TEXT NOT NULL,
          reviewer_ref TEXT NOT NULL,
          action_ref TEXT NOT NULL,
          effect_ref TEXT NOT NULL,
          effect_value_ref TEXT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );
        "#,
    )?;
    Ok(())
}

pub fn persist_review_item(
    config: &DatabaseConfig,
    item: &ReviewItem,
) -> Result<ReviewItem, ReviewWorkstationStoreError> {
    let item = canonical_review_item_for_persistence(item.clone());
    item.validate()
        .map_err(|_| ReviewWorkstationStoreError::InvalidDomainObject)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"
        INSERT INTO semantic.review_item
          (review_item_ref, semantic_ref, item_kind_ref, reason, current_status_ref,
           candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,true,false,false,false)
        ON CONFLICT (review_item_ref) DO NOTHING
        "#,
        &[
            &item.review_item_ref,
            &item.semantic_ref,
            &kind_db(item.item_kind),
            &item.reason,
            &status_db(item.current_status),
        ],
    )?;
    persist_refs(
        &mut tx,
        "semantic.review_item_provenance",
        "provenance_ref",
        &item.review_item_ref,
        &item.provenance_refs,
    )?;
    persist_refs(
        &mut tx,
        "semantic.review_item_source",
        "source_ref",
        &item.review_item_ref,
        &item.source_refs,
    )?;
    let action_refs = item
        .available_actions
        .iter()
        .map(|action| action_db(*action).to_owned())
        .collect::<Vec<_>>();
    persist_refs(
        &mut tx,
        "semantic.review_item_action",
        "action_ref",
        &item.review_item_ref,
        &action_refs,
    )?;
    persist_refs(
        &mut tx,
        "semantic.review_item_consumer",
        "consumer_ref",
        &item.review_item_ref,
        &item.affected_consumer_refs,
    )?;
    tx.commit()?;
    let loaded = load_review_item_with_client(&mut client, &item.review_item_ref)?
        .ok_or(ReviewWorkstationStoreError::ExistingRowConflict)?;
    if loaded != item {
        return Err(ReviewWorkstationStoreError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn persist_review_receipt(
    config: &DatabaseConfig,
    receipt: &ReviewReceipt,
) -> Result<(), ReviewWorkstationStoreError> {
    validate_review_receipt(receipt)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    persist_review_receipt_with_client(&mut tx, receipt)?;
    tx.commit()?;
    Ok(())
}

pub fn apply_persisted_review_command(
    config: &DatabaseConfig,
    command: &ReviewCommand,
) -> Result<(ReviewReceipt, ReviewItem), ReviewWorkstationStoreError> {
    command
        .validate()
        .map_err(|_| ReviewWorkstationStoreError::InvalidDomainObject)?;
    install_review_workstation_schema(config)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;

    let locked = tx.query_opt(
        "SELECT review_item_ref FROM semantic.review_item WHERE review_item_ref=$1 FOR UPDATE",
        &[&command.review_item_ref],
    )?;
    if locked.is_none() {
        return Err(ReviewWorkstationStoreError::ExistingRowConflict);
    }

    let mut item = load_review_item_with_client(&mut tx, &command.review_item_ref)?
        .ok_or(ReviewWorkstationStoreError::ExistingRowConflict)?;
    let receipt = apply_review_command(&mut item, command)
        .map_err(|_| ReviewWorkstationStoreError::InvalidDomainObject)?;

    persist_review_receipt_with_client(&mut tx, &receipt)?;

    let persisted = load_review_item_with_client(&mut tx, &command.review_item_ref)?
        .ok_or(ReviewWorkstationStoreError::ExistingRowConflict)?;
    if persisted.current_status != item.current_status {
        return Err(ReviewWorkstationStoreError::ExistingRowConflict);
    }

    tx.commit()?;
    Ok((receipt, persisted))
}

fn validate_review_receipt(
    receipt: &ReviewReceipt,
) -> Result<(), ReviewWorkstationStoreError> {
    if !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
        || receipt.command_ref.trim().is_empty()
        || receipt.review_item_ref.trim().is_empty()
        || receipt.semantic_ref.trim().is_empty()
        || receipt.reviewer_ref.trim().is_empty()
    {
        return Err(ReviewWorkstationStoreError::InvalidDomainObject);
    }
    Ok(())
}

fn persist_review_receipt_with_client(
    client: &mut impl GenericClient,
    receipt: &ReviewReceipt,
) -> Result<(), ReviewWorkstationStoreError> {
    validate_review_receipt(receipt)?;
    let (effect_ref, effect_value_ref) = effect_row(&receipt.effect);
    client.execute(
        r#"
        INSERT INTO semantic.review_receipt
          (command_ref, review_item_ref, semantic_ref, reviewer_ref, action_ref,
           effect_ref, effect_value_ref, candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,true,false,false,false)
        ON CONFLICT (command_ref) DO NOTHING
        "#,
        &[
            &receipt.command_ref,
            &receipt.review_item_ref,
            &receipt.semantic_ref,
            &receipt.reviewer_ref,
            &action_db(receipt.action),
            &effect_ref,
            &effect_value_ref,
        ],
    )?;
    if let Some(status) = receipt_status_after_effect(&receipt.effect) {
        client.execute(
            "UPDATE semantic.review_item SET current_status_ref=$2 WHERE review_item_ref=$1",
            &[&receipt.review_item_ref, &status],
        )?;
    }
    Ok(())
}

pub fn load_review_queue(
    config: &DatabaseConfig,
) -> Result<Vec<ReviewItem>, ReviewWorkstationStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client.query(
        "SELECT review_item_ref FROM semantic.review_item ORDER BY review_item_ref",
        &[],
    )?;
    refs.into_iter()
        .map(|row| {
            let reference: String = row.get(0);
            load_review_item_with_client(&mut client, &reference)?
                .ok_or(ReviewWorkstationStoreError::ExistingRowConflict)
        })
        .collect()
}

fn persist_refs(
    client: &mut impl GenericClient,
    table: &str,
    value_column: &str,
    item_ref: &str,
    values: &[String],
) -> Result<(), ReviewWorkstationStoreError> {
    let sql = format!(
        "INSERT INTO {table} (review_item_ref, {value_column}) VALUES ($1,$2) ON CONFLICT DO NOTHING"
    );
    for value in values {
        client.execute(&sql, &[&item_ref, value])?;
    }
    Ok(())
}

fn load_refs(
    client: &mut impl GenericClient,
    table: &str,
    value_column: &str,
    item_ref: &str,
) -> Result<Vec<String>, ReviewWorkstationStoreError> {
    let sql = format!(
        "SELECT {value_column} FROM {table} WHERE review_item_ref=$1 ORDER BY {value_column}"
    );
    Ok(client
        .query(&sql, &[&item_ref])?
        .into_iter()
        .map(|row| row.get(0))
        .collect())
}

fn load_review_item_with_client(
    client: &mut impl GenericClient,
    item_ref: &str,
) -> Result<Option<ReviewItem>, ReviewWorkstationStoreError> {
    let row = client.query_opt(
        "SELECT semantic_ref, item_kind_ref, reason, current_status_ref,
                candidate_only, creates_semantic_authority, applicability_promoted, claim_truth_promoted
         FROM semantic.review_item WHERE review_item_ref=$1",
        &[&item_ref],
    )?;
    let Some(row) = row else { return Ok(None); };
    let action_refs = load_refs(
        client,
        "semantic.review_item_action",
        "action_ref",
        item_ref,
    )?;
    Ok(Some(ReviewItem {
        review_item_ref: item_ref.to_owned(),
        semantic_ref: row.get(0),
        item_kind: kind_from_db(row.get::<_, String>(1).as_str())?,
        reason: row.get(2),
        provenance_refs: load_refs(
            client,
            "semantic.review_item_provenance",
            "provenance_ref",
            item_ref,
        )?,
        source_refs: load_refs(
            client,
            "semantic.review_item_source",
            "source_ref",
            item_ref,
        )?,
        current_status: status_from_db(row.get::<_, String>(3).as_str())?,
        available_actions: action_refs
            .iter()
            .map(|value| action_from_db(value))
            .collect::<Result<Vec<_>, _>>()?,
        affected_consumer_refs: load_refs(
            client,
            "semantic.review_item_consumer",
            "consumer_ref",
            item_ref,
        )?,
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    }))
}

fn effect_row(effect: &ReviewEffect) -> (&'static str, Option<String>) {
    match effect {
        ReviewEffect::StatusChanged { current, .. } => {
            ("status_changed", Some(status_db(*current).to_owned()))
        }
        ReviewEffect::EvidenceRequested {
            evidence_request_ref,
        } => ("evidence_requested", Some(evidence_request_ref.clone())),
        ReviewEffect::SourceOpenRequested => ("source_open_requested", None),
        ReviewEffect::AuthorityFollowRequested => ("authority_follow_requested", None),
    }
}

fn receipt_status_after_effect(effect: &ReviewEffect) -> Option<&'static str> {
    match effect {
        ReviewEffect::StatusChanged { current, .. } => Some(status_db(*current)),
        ReviewEffect::EvidenceRequested { .. } => Some("needs_evidence"),
        ReviewEffect::SourceOpenRequested | ReviewEffect::AuthorityFollowRequested => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_effects_do_not_persist_review_status_changes() {
        assert_eq!(
            receipt_status_after_effect(&ReviewEffect::SourceOpenRequested),
            None
        );
        assert_eq!(
            receipt_status_after_effect(&ReviewEffect::AuthorityFollowRequested),
            None
        );
        assert_eq!(
            receipt_status_after_effect(&ReviewEffect::EvidenceRequested {
                evidence_request_ref: "request:1".into(),
            }),
            Some("needs_evidence")
        );
    }

    #[test]
    fn item_kind_and_action_storage_tags_round_trip() {
        for kind in [
            ReviewItemKind::PnfParse,
            ReviewItemKind::Observation,
            ReviewItemKind::ClaimContestation,
            ReviewItemKind::EventAssembly,
            ReviewItemKind::ChronologyAmbiguity,
            ReviewItemKind::AuthorityFollow,
            ReviewItemKind::ResearchAcquisition,
            ReviewItemKind::LegalTreatment,
            ReviewItemKind::ScopeHandoff,
        ] {
            assert_eq!(kind_from_db(kind_db(kind)).unwrap(), kind);
        }
        for action in [
            ReviewAction::Accept,
            ReviewAction::Reject,
            ReviewAction::Abstain,
            ReviewAction::Qualify,
            ReviewAction::Supersede,
            ReviewAction::RequestEvidence,
            ReviewAction::OpenSource,
            ReviewAction::FollowAuthority,
        ] {
            assert_eq!(action_from_db(action_db(action)).unwrap(), action);
        }
    }
}
