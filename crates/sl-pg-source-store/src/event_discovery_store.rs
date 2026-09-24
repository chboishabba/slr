use postgres::{Client, NoTls};
use thiserror::Error;

use sensiblaw_core::{
    event_discovery::{
        CandidateEventJoinProposal, EventJoinSignal, EventJoinSignalKind,
    },
    review_workstation::ReviewItem,
};

use crate::{
    install_review_workstation_schema, persist_review_item, DatabaseConfig,
    ReviewWorkstationStoreError,
};

#[derive(Debug, Error)]
pub enum EventDiscoveryStoreError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error("invalid event discovery proposal")]
    InvalidProposal,
    #[error("existing event proposal differs from requested value")]
    ExistingProposalConflict,
    #[error(transparent)]
    Review(#[from] ReviewWorkstationStoreError),
}

fn signal_kind_as_db(kind: EventJoinSignalKind) -> &'static str {
    match kind {
        EventJoinSignalKind::SharedQid => "shared_qid",
        EventJoinSignalKind::SharedEntity => "shared_entity",
        EventJoinSignalKind::SharedTemporalBucket => "shared_temporal_bucket",
        EventJoinSignalKind::SharedFingerprint => "shared_fingerprint",
        EventJoinSignalKind::ExplicitCrossReference => "explicit_cross_reference",
        EventJoinSignalKind::UserDeclaredSameIncident => "user_declared_same_incident",
    }
}

fn signal_kind_from_db(value: &str) -> Result<EventJoinSignalKind, EventDiscoveryStoreError> {
    match value {
        "shared_qid" => Ok(EventJoinSignalKind::SharedQid),
        "shared_entity" => Ok(EventJoinSignalKind::SharedEntity),
        "shared_temporal_bucket" => Ok(EventJoinSignalKind::SharedTemporalBucket),
        "shared_fingerprint" => Ok(EventJoinSignalKind::SharedFingerprint),
        "explicit_cross_reference" => Ok(EventJoinSignalKind::ExplicitCrossReference),
        "user_declared_same_incident" => Ok(EventJoinSignalKind::UserDeclaredSameIncident),
        _ => Err(EventDiscoveryStoreError::InvalidProposal),
    }
}

pub fn install_event_discovery_schema(
    config: &DatabaseConfig,
) -> Result<(), EventDiscoveryStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(
        r#"
        CREATE SCHEMA IF NOT EXISTS semantic;

        CREATE TABLE IF NOT EXISTS semantic.event_join_proposal (
          proposal_ref TEXT PRIMARY KEY,
          policy_ref TEXT NOT NULL,
          candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
          requires_review BOOLEAN NOT NULL CHECK (requires_review),
          creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE TABLE IF NOT EXISTS semantic.event_join_proposal_observation (
          proposal_ref TEXT NOT NULL REFERENCES semantic.event_join_proposal(proposal_ref) ON DELETE CASCADE,
          observation_ref TEXT NOT NULL,
          ordinal INTEGER NOT NULL,
          PRIMARY KEY (proposal_ref, observation_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.event_join_proposal_statement (
          proposal_ref TEXT NOT NULL REFERENCES semantic.event_join_proposal(proposal_ref) ON DELETE CASCADE,
          statement_ref TEXT NOT NULL,
          ordinal INTEGER NOT NULL,
          PRIMARY KEY (proposal_ref, statement_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.event_join_proposal_source_family (
          proposal_ref TEXT NOT NULL REFERENCES semantic.event_join_proposal(proposal_ref) ON DELETE CASCADE,
          source_family_ref TEXT NOT NULL,
          ordinal INTEGER NOT NULL,
          PRIMARY KEY (proposal_ref, source_family_ref)
        );

        CREATE TABLE IF NOT EXISTS semantic.event_join_proposal_signal (
          proposal_ref TEXT NOT NULL REFERENCES semantic.event_join_proposal(proposal_ref) ON DELETE CASCADE,
          signal_kind TEXT NOT NULL,
          evidence_ref TEXT NOT NULL,
          detector_ref TEXT NOT NULL,
          ordinal INTEGER NOT NULL,
          PRIMARY KEY (proposal_ref, signal_kind, evidence_ref, detector_ref)
        );
        "#,
    )?;
    Ok(())
}

pub fn persist_event_join_proposal(
    config: &DatabaseConfig,
    proposal: &CandidateEventJoinProposal,
) -> Result<CandidateEventJoinProposal, EventDiscoveryStoreError> {
    proposal
        .validate()
        .map_err(|_| EventDiscoveryStoreError::InvalidProposal)?;
    install_event_discovery_schema(config)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"
        INSERT INTO semantic.event_join_proposal
          (proposal_ref, policy_ref, candidate_only, requires_review,
           creates_event_identity, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,true,true,false,false,false,false)
        ON CONFLICT (proposal_ref) DO NOTHING
        "#,
        &[&proposal.proposal_ref, &proposal.policy_ref],
    )?;

    for (ordinal, value) in proposal.observation_refs.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO semantic.event_join_proposal_observation
              (proposal_ref, observation_ref, ordinal)
            VALUES ($1,$2,$3)
            ON CONFLICT DO NOTHING
            "#,
            &[&proposal.proposal_ref, value, &(ordinal as i32)],
        )?;
    }
    for (ordinal, value) in proposal.statement_refs.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO semantic.event_join_proposal_statement
              (proposal_ref, statement_ref, ordinal)
            VALUES ($1,$2,$3)
            ON CONFLICT DO NOTHING
            "#,
            &[&proposal.proposal_ref, value, &(ordinal as i32)],
        )?;
    }
    for (ordinal, value) in proposal.source_family_refs.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO semantic.event_join_proposal_source_family
              (proposal_ref, source_family_ref, ordinal)
            VALUES ($1,$2,$3)
            ON CONFLICT DO NOTHING
            "#,
            &[&proposal.proposal_ref, value, &(ordinal as i32)],
        )?;
    }
    for (ordinal, signal) in proposal.signals.iter().enumerate() {
        tx.execute(
            r#"
            INSERT INTO semantic.event_join_proposal_signal
              (proposal_ref, signal_kind, evidence_ref, detector_ref, ordinal)
            VALUES ($1,$2,$3,$4,$5)
            ON CONFLICT DO NOTHING
            "#,
            &[
                &proposal.proposal_ref,
                &signal_kind_as_db(signal.kind),
                &signal.evidence_ref,
                &signal.detector_ref,
                &(ordinal as i32),
            ],
        )?;
    }
    tx.commit()?;

    let persisted = load_event_join_proposal(config, &proposal.proposal_ref)?
        .ok_or(EventDiscoveryStoreError::ExistingProposalConflict)?;
    if &persisted != proposal {
        return Err(EventDiscoveryStoreError::ExistingProposalConflict);
    }
    Ok(persisted)
}

pub fn persist_event_join_proposal_with_review(
    config: &DatabaseConfig,
    proposal: &CandidateEventJoinProposal,
    affected_consumer_refs: Vec<String>,
) -> Result<(CandidateEventJoinProposal, ReviewItem), EventDiscoveryStoreError> {
    install_review_workstation_schema(config)?;
    let proposal = persist_event_join_proposal(config, proposal)?;
    let review_item = proposal.to_review_item(affected_consumer_refs);
    persist_review_item(config, &review_item)?;
    Ok((proposal, review_item))
}

pub fn load_event_join_proposal(
    config: &DatabaseConfig,
    proposal_ref: &str,
) -> Result<Option<CandidateEventJoinProposal>, EventDiscoveryStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let Some(row) = client.query_opt(
        r#"
        SELECT proposal_ref, policy_ref, candidate_only, requires_review,
               creates_event_identity, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted
        FROM semantic.event_join_proposal
        WHERE proposal_ref=$1
        "#,
        &[&proposal_ref],
    )? else {
        return Ok(None);
    };

    let observation_refs = client
        .query(
            r#"
            SELECT observation_ref FROM semantic.event_join_proposal_observation
            WHERE proposal_ref=$1 ORDER BY ordinal, observation_ref
            "#,
            &[&proposal_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();
    let statement_refs = client
        .query(
            r#"
            SELECT statement_ref FROM semantic.event_join_proposal_statement
            WHERE proposal_ref=$1 ORDER BY ordinal, statement_ref
            "#,
            &[&proposal_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();
    let source_family_refs = client
        .query(
            r#"
            SELECT source_family_ref FROM semantic.event_join_proposal_source_family
            WHERE proposal_ref=$1 ORDER BY ordinal, source_family_ref
            "#,
            &[&proposal_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect();
    let signals = client
        .query(
            r#"
            SELECT signal_kind, evidence_ref, detector_ref
            FROM semantic.event_join_proposal_signal
            WHERE proposal_ref=$1 ORDER BY ordinal, signal_kind, evidence_ref
            "#,
            &[&proposal_ref],
        )?
        .into_iter()
        .map(|row| {
            Ok(EventJoinSignal {
                kind: signal_kind_from_db(row.get::<_, String>(0).as_str())?,
                evidence_ref: row.get(1),
                detector_ref: row.get(2),
            })
        })
        .collect::<Result<Vec<_>, EventDiscoveryStoreError>>()?;

    let value = CandidateEventJoinProposal {
        proposal_ref: row.get(0),
        policy_ref: row.get(1),
        observation_refs,
        statement_refs,
        source_family_refs,
        signals,
        candidate_only: row.get(2),
        requires_review: row.get(3),
        creates_event_identity: row.get(4),
        creates_semantic_authority: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    };
    value
        .validate()
        .map_err(|_| EventDiscoveryStoreError::InvalidProposal)?;
    Ok(Some(value))
}

pub fn load_pending_event_join_proposals(
    config: &DatabaseConfig,
) -> Result<Vec<CandidateEventJoinProposal>, EventDiscoveryStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let refs = client
        .query(
            r#"
            SELECT proposal_ref FROM semantic.event_join_proposal
            ORDER BY proposal_ref
            "#,
            &[],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    refs.into_iter()
        .map(|reference| {
            load_event_join_proposal(config, &reference)?
                .ok_or(EventDiscoveryStoreError::ExistingProposalConflict)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::event_discovery::{
        CandidateEventJoinProposal, EventJoinSignal, EventJoinSignalKind,
    };

    #[test]
    fn proposal_remains_review_gated_before_store() {
        let proposal = CandidateEventJoinProposal {
            proposal_ref: "proposal:1".into(),
            observation_refs: vec!["observation:a".into(), "observation:b".into()],
            statement_refs: vec!["statement:a".into(), "statement:b".into()],
            source_family_refs: vec!["book".into(), "wiki".into()],
            signals: vec![
                EventJoinSignal {
                    kind: EventJoinSignalKind::SharedEntity,
                    evidence_ref: "entity:gwb".into(),
                    detector_ref: "detector:1".into(),
                },
                EventJoinSignal {
                    kind: EventJoinSignalKind::SharedTemporalBucket,
                    evidence_ref: "date:2001".into(),
                    detector_ref: "detector:1".into(),
                },
            ],
            policy_ref: "policy:1".into(),
            candidate_only: true,
            requires_review: true,
            creates_event_identity: false,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        proposal.validate().unwrap();
        let item = proposal.to_review_item(vec!["timeline:matter:gwb".into()]);
        assert_eq!(item.item_kind, sensiblaw_core::review_workstation::ReviewItemKind::EventAssembly);
    }
}
