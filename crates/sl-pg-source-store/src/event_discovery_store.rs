use postgres::{Client, NoTls};
use thiserror::Error;

use sensiblaw_core::{
    event_discovery::{
        AcceptedEventAssemblyReceipt, CandidateEventJoinProposal,
        EventJoinSignal, EventJoinSignalKind,
    },
    review_workstation::ReviewItem,
};

use crate::{
    canonical_observation_event_link_ref, install_review_workstation_schema,
    install_statement_trace_schema, persist_review_item, DatabaseConfig,
    ObservationEventLink, ReviewWorkstationStoreError, StatementTraceStoreError,
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
    #[error(transparent)]
    StatementTrace(#[from] StatementTraceStoreError),
    #[error("event assembly review has not been accepted")]
    ReviewNotAccepted,
    #[error("accepted EventAssembly review receipt is missing or mismatched")]
    AcceptedReviewReceiptMissing,
    #[error("proposal observation lacks persisted M12 observation ancestry: {0}")]
    MissingObservationAncestry(String),
    #[error("existing reviewed event assembly conflicts with requested identity")]
    ExistingAssemblyConflict,
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

        CREATE TABLE IF NOT EXISTS semantic.event_assembly_materialization (
          assembly_ref TEXT PRIMARY KEY,
          proposal_ref TEXT NOT NULL UNIQUE
            REFERENCES semantic.event_join_proposal(proposal_ref),
          event_ref TEXT NOT NULL,
          review_item_ref TEXT NOT NULL
            REFERENCES semantic.review_item(review_item_ref),
          accepted_review_command_ref TEXT NOT NULL
            REFERENCES semantic.review_receipt(command_ref),
          reviewed_event_identity BOOLEAN NOT NULL CHECK (reviewed_event_identity),
          creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
          claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
        );

        CREATE INDEX IF NOT EXISTS event_assembly_event_idx
          ON semantic.event_assembly_materialization (event_ref);
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

pub fn canonical_event_assembly_ref(
    proposal_ref: &str,
    event_ref: &str,
) -> String {
    format!("event-assembly:{proposal_ref}:{event_ref}")
}

pub fn materialize_accepted_event_join(
    config: &DatabaseConfig,
    proposal_ref: &str,
    event_ref: &str,
    accepted_review_command_ref: &str,
) -> Result<AcceptedEventAssemblyReceipt, EventDiscoveryStoreError> {
    if proposal_ref.trim().is_empty()
        || event_ref.trim().is_empty()
        || accepted_review_command_ref.trim().is_empty()
    {
        return Err(EventDiscoveryStoreError::InvalidProposal);
    }

    install_review_workstation_schema(config)?;
    install_statement_trace_schema(config)?;
    install_event_discovery_schema(config)?;

    let proposal = load_event_join_proposal(config, proposal_ref)?
        .ok_or(EventDiscoveryStoreError::ExistingProposalConflict)?;
    let review_item_ref = format!("review-item:{proposal_ref}");
    let assembly_ref = canonical_event_assembly_ref(proposal_ref, event_ref);

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;

    let review = tx.query_opt(
        r#"
        SELECT semantic_ref, item_kind_ref, current_status_ref
        FROM semantic.review_item
        WHERE review_item_ref=$1
        FOR UPDATE
        "#,
        &[&review_item_ref],
    )?;
    let Some(review) = review else {
        return Err(EventDiscoveryStoreError::ReviewNotAccepted);
    };
    let semantic_ref: String = review.get(0);
    let item_kind_ref: String = review.get(1);
    let current_status_ref: String = review.get(2);
    if semantic_ref != proposal_ref
        || item_kind_ref != "event_assembly"
        || current_status_ref != "accepted"
    {
        return Err(EventDiscoveryStoreError::ReviewNotAccepted);
    }

    let accepted_receipt: bool = tx
        .query_one(
            r#"
            SELECT EXISTS (
              SELECT 1
              FROM semantic.review_receipt
              WHERE command_ref=$1
                AND review_item_ref=$2
                AND semantic_ref=$3
                AND action_ref='accept'
                AND effect_ref='status_changed'
                AND effect_value_ref='accepted'
                AND candidate_only
                AND NOT creates_semantic_authority
                AND NOT applicability_promoted
                AND NOT claim_truth_promoted
            )
            "#,
            &[
                &accepted_review_command_ref,
                &review_item_ref,
                &proposal_ref,
            ],
        )?
        .get(0);
    if !accepted_receipt {
        return Err(EventDiscoveryStoreError::AcceptedReviewReceiptMissing);
    }

    for observation_ref in &proposal.observation_refs {
        let has_ancestry: bool = tx
            .query_one(
                r#"
                SELECT EXISTS (
                  SELECT 1
                  FROM pnf.statement_observation_link
                  WHERE observation_ref=$1
                )
                "#,
                &[observation_ref],
            )?
            .get(0);
        if !has_ancestry {
            return Err(EventDiscoveryStoreError::MissingObservationAncestry(
                observation_ref.clone(),
            ));
        }
    }

    tx.execute(
        r#"
        INSERT INTO semantic.event_assembly_materialization
          (assembly_ref, proposal_ref, event_ref, review_item_ref,
           accepted_review_command_ref, reviewed_event_identity,
           creates_semantic_authority, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,true,false,false)
        ON CONFLICT (assembly_ref) DO NOTHING
        "#,
        &[
            &assembly_ref,
            &proposal_ref,
            &event_ref,
            &review_item_ref,
            &accepted_review_command_ref,
        ],
    )?;

    let persisted_assembly = tx.query_opt(
        r#"
        SELECT proposal_ref, event_ref, review_item_ref,
               accepted_review_command_ref, reviewed_event_identity,
               creates_semantic_authority, claim_truth_promoted
        FROM semantic.event_assembly_materialization
        WHERE assembly_ref=$1
        "#,
        &[&assembly_ref],
    )?;
    let Some(persisted_assembly) = persisted_assembly else {
        return Err(EventDiscoveryStoreError::ExistingAssemblyConflict);
    };
    if persisted_assembly.get::<_, String>(0) != proposal_ref
        || persisted_assembly.get::<_, String>(1) != event_ref
        || persisted_assembly.get::<_, String>(2) != review_item_ref
        || persisted_assembly.get::<_, String>(3) != accepted_review_command_ref
        || !persisted_assembly.get::<_, bool>(4)
        || persisted_assembly.get::<_, bool>(5)
        || persisted_assembly.get::<_, bool>(6)
    {
        return Err(EventDiscoveryStoreError::ExistingAssemblyConflict);
    }

    for observation_ref in &proposal.observation_refs {
        let link = ObservationEventLink {
            link_ref: canonical_observation_event_link_ref(
                observation_ref,
                event_ref,
            ),
            observation_ref: observation_ref.clone(),
            event_ref: event_ref.to_owned(),
            assembly_receipt_ref: accepted_review_command_ref.to_owned(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        link.validate()?;

        tx.execute(
            r#"
            INSERT INTO pnf.observation_event_link
              (link_ref, observation_ref, event_ref, assembly_receipt_ref,
               candidate_only, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted)
            VALUES ($1,$2,$3,$4,true,false,false,false)
            ON CONFLICT (link_ref) DO NOTHING
            "#,
            &[
                &link.link_ref,
                &link.observation_ref,
                &link.event_ref,
                &link.assembly_receipt_ref,
            ],
        )?;

        let persisted = tx.query_opt(
            r#"
            SELECT observation_ref, event_ref, assembly_receipt_ref,
                   candidate_only, creates_semantic_authority,
                   applicability_promoted, claim_truth_promoted
            FROM pnf.observation_event_link
            WHERE link_ref=$1
            "#,
            &[&link.link_ref],
        )?;
        let Some(persisted) = persisted else {
            return Err(EventDiscoveryStoreError::ExistingAssemblyConflict);
        };
        if persisted.get::<_, String>(0) != link.observation_ref
            || persisted.get::<_, String>(1) != link.event_ref
            || persisted.get::<_, String>(2) != link.assembly_receipt_ref
            || !persisted.get::<_, bool>(3)
            || persisted.get::<_, bool>(4)
            || persisted.get::<_, bool>(5)
            || persisted.get::<_, bool>(6)
        {
            return Err(EventDiscoveryStoreError::ExistingAssemblyConflict);
        }
    }

    tx.commit()?;

    let receipt = AcceptedEventAssemblyReceipt {
        assembly_ref,
        proposal_ref: proposal.proposal_ref,
        event_ref: event_ref.to_owned(),
        review_item_ref,
        accepted_review_command_ref: accepted_review_command_ref.to_owned(),
        observation_refs: proposal.observation_refs,
        statement_refs: proposal.statement_refs,
        reviewed_event_identity: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    };
    receipt
        .validate()
        .map_err(|_| EventDiscoveryStoreError::InvalidProposal)?;
    Ok(receipt)
}

pub fn load_event_assembly_receipt(
    config: &DatabaseConfig,
    proposal_ref: &str,
) -> Result<Option<AcceptedEventAssemblyReceipt>, EventDiscoveryStoreError> {
    install_event_discovery_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"
        SELECT assembly_ref, event_ref, review_item_ref,
               accepted_review_command_ref, reviewed_event_identity,
               creates_semantic_authority, claim_truth_promoted
        FROM semantic.event_assembly_materialization
        WHERE proposal_ref=$1
        "#,
        &[&proposal_ref],
    )?;
    let Some(row) = row else {
        return Ok(None);
    };
    let proposal = load_event_join_proposal(config, proposal_ref)?
        .ok_or(EventDiscoveryStoreError::ExistingProposalConflict)?;
    let receipt = AcceptedEventAssemblyReceipt {
        assembly_ref: row.get(0),
        proposal_ref: proposal_ref.to_owned(),
        event_ref: row.get(1),
        review_item_ref: row.get(2),
        accepted_review_command_ref: row.get(3),
        observation_refs: proposal.observation_refs,
        statement_refs: proposal.statement_refs,
        reviewed_event_identity: row.get(4),
        creates_semantic_authority: row.get(5),
        claim_truth_promoted: row.get(6),
    };
    receipt
        .validate()
        .map_err(|_| EventDiscoveryStoreError::ExistingAssemblyConflict)?;
    Ok(Some(receipt))
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
    fn assembly_ref_is_deterministic_and_does_not_equal_proposal() {
        let reference =
            canonical_event_assembly_ref("proposal:1", "event:reviewed:1");
        assert_eq!(
            reference,
            "event-assembly:proposal:1:event:reviewed:1"
        );
        assert_ne!(reference, "proposal:1");
    }

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
