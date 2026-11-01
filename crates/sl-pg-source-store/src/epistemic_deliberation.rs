//! Generic, source-native epistemic deliberation diagnostic.
//!
//! Marcin Roszkowski (2024; Wikimedia CEE 2026) investigates how proposal,
//! collaborative knowledge work, and closure change Wikidata's vocabulary.
//! This integration adds a *record-completeness* view, not a consensus oracle.
//! Exact source revisions and event order survive. Original S29 review retains
//! authority for workflow decisions; nothing here publishes Wikidata changes.
use std::collections::BTreeSet;

use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const DELIBERATION_SCHEMA: &str = "itir.governance.deliberation_assessment.v1";
const MAX_EVENTS: usize = 128;
const MAX_REF_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationKind {
    Proposal,
    Support,
    Objection,
    Amendment,
    Decision,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationJudgement {
    Approve,
    Reject,
    Defer,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliberationEvent {
    pub event_ref: String,
    pub source_revision_ref: String,
    pub actor_ref: String,
    /// Producer-supplied Unix timestamp, never inferred from a revision id.
    pub at_unix_seconds: i64,
    pub kind: DeliberationKind,
    pub supersedes_ref: Option<String>,
    pub judgement: Option<DeliberationJudgement>,
    pub consensus_assessment_ref: Option<String>,
    pub authority_receipt_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliberationPacket {
    pub proposal_ref: String,
    pub source_revision_ref: String,
    pub policy_revision_ref: String,
    /// Historical rule supplied by the caller; not silently replaced by today's policy.
    pub minimum_age_seconds: i64,
    pub events: Vec<DeliberationEvent>,
    /// Producer-reported property creation, not proof of a statement's truth.
    pub created_property_ref: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliberationHold {
    IndependentParticipationNotObserved,
    NoTerminalDecision,
    DecisionMakerEqualsProposer,
    MinimumProposalAgeNotMet,
    ConsensusJudgementSourceMissing,
    DecisionAuthoritySourceMissing,
    CreatedPropertyIdentityNotObserved,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliberationAssessment {
    pub schema: String,
    pub packet_ref: String,
    pub proposal_ref: String,
    pub source_revision_ref: String,
    pub policy_revision_ref: String,
    pub event_refs: Vec<String>,
    pub event_source_revision_refs: Vec<String>,
    pub objection_refs: Vec<String>,
    pub amendment_refs: Vec<String>,
    pub recorded_judgement: Option<DeliberationJudgement>,
    pub recorded_consensus_ref: Option<String>,
    pub recorded_authority_ref: Option<String>,
    pub holds: Vec<DeliberationHold>,
    pub record_reviewable: bool,
    pub candidate_only: bool,
    pub semantic_truth_promoted: bool,
    pub community_consensus_verified: bool,
    pub actor_authority_verified: bool,
    pub wikidata_edit_authorised: bool,
    pub ao_dispatch_allowed: bool,
}
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DeliberationError {
    #[error("missing or unbounded source/actor/policy reference")]
    MissingSource,
    #[error("invalid sequence, timestamp, duplicate id or amendment reference")]
    InvalidEventHistory,
    #[error("invalid or missing decision-specific evidence")]
    InvalidDecision,
    #[error("missing or invalid policy guard")]
    InvalidPolicy,
}

fn nonempty(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= MAX_REF_CHARS && !s.chars().any(char::is_control)
}
fn optional_valid(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|v| nonempty(v))
}
fn supplied(value: &Option<String>) -> bool {
    value.as_ref().is_some_and(|v| nonempty(v))
}

/// Evaluate evidence completeness only. Source ID, role receipt and consensus
/// source are *unverified caller claims*. SLR's persisted source revision and
/// S29 original review history must be independently reopened before any use.
pub fn assess_deliberation(packet: &DeliberationPacket) -> Result<DeliberationAssessment, DeliberationError> {
    if !nonempty(&packet.proposal_ref)
        || !nonempty(&packet.source_revision_ref)
        || !nonempty(&packet.policy_revision_ref)
        || !optional_valid(&packet.created_property_ref)
    {
        return Err(DeliberationError::MissingSource);
    }
    if !(0..=366 * 86400).contains(&packet.minimum_age_seconds) {
        return Err(DeliberationError::InvalidPolicy);
    }
    if packet.events.is_empty() || packet.events.len() > MAX_EVENTS {
        return Err(DeliberationError::InvalidEventHistory);
    }
    let mut seen = BTreeSet::new();
    let mut previous: Option<i64> = None;
    let mut ended = false;
    for (i, event) in packet.events.iter().enumerate() {
        if !nonempty(&event.event_ref) || !nonempty(&event.source_revision_ref)
            || !nonempty(&event.actor_ref) || !seen.insert(event.event_ref.as_str())
            || event.at_unix_seconds < 0
            || previous.is_some_and(|p| event.at_unix_seconds < p)
            || ended
            || (i == 0) != (event.kind == DeliberationKind::Proposal)
        {
            return Err(DeliberationError::InvalidEventHistory);
        }
        if let Some(superseded) = &event.supersedes_ref {
            if event.kind != DeliberationKind::Amendment || superseded == &event.event_ref
                || !seen.contains(superseded.as_str()) {
                return Err(DeliberationError::InvalidEventHistory);
            }
        }
        if !optional_valid(&event.consensus_assessment_ref)
            || !optional_valid(&event.authority_receipt_ref)
        {
            return Err(DeliberationError::MissingSource);
        }
        if event.kind == DeliberationKind::Decision {
            if event.judgement.is_none() {
                return Err(DeliberationError::InvalidDecision);
            }
            ended = true;
        } else if event.judgement.is_some()
            || event.consensus_assessment_ref.is_some()
            || event.authority_receipt_ref.is_some()
        {
            return Err(DeliberationError::InvalidDecision);
        }
        previous = Some(event.at_unix_seconds);
    }
    let proposer = &packet.events[0].actor_ref;
    let independent = packet.events.iter().skip(1).any(|e| {
        e.kind != DeliberationKind::Decision && &e.actor_ref != proposer
    });
    let decision = packet.events.last().filter(|e| e.kind == DeliberationKind::Decision);
    let mut holds = vec![];
    if !independent {
        holds.push(DeliberationHold::IndependentParticipationNotObserved);
    }
    if let Some(d) = decision {
        if &d.actor_ref == proposer {
            holds.push(DeliberationHold::DecisionMakerEqualsProposer);
        }
        if d.at_unix_seconds - packet.events[0].at_unix_seconds <= packet.minimum_age_seconds {
            holds.push(DeliberationHold::MinimumProposalAgeNotMet);
        }
        if !supplied(&d.consensus_assessment_ref) {
            holds.push(DeliberationHold::ConsensusJudgementSourceMissing);
        }
        if !supplied(&d.authority_receipt_ref) {
            holds.push(DeliberationHold::DecisionAuthoritySourceMissing);
        }
        if d.judgement == Some(DeliberationJudgement::Approve) && !supplied(&packet.created_property_ref) {
            holds.push(DeliberationHold::CreatedPropertyIdentityNotObserved);
        }
    } else {
        holds.push(DeliberationHold::NoTerminalDecision);
    }
    // Digest exact serialized source-coordinates, not a fabricated SLR PG
    // record or an S29 review receipt.
    let serialized = serde_json::to_vec(packet).map_err(|_| DeliberationError::InvalidEventHistory)?;
    let packet_ref = format!("deliberation:sha256:{:x}", Sha256::digest(&serialized));
    Ok(DeliberationAssessment {
        schema: DELIBERATION_SCHEMA.into(),
        packet_ref,
        proposal_ref: packet.proposal_ref.clone(),
        source_revision_ref: packet.source_revision_ref.clone(),
        policy_revision_ref: packet.policy_revision_ref.clone(),
        event_refs: packet.events.iter().map(|e| e.event_ref.clone()).collect(),
        event_source_revision_refs: packet.events.iter().map(|e| e.source_revision_ref.clone()).collect(),
        objection_refs: packet.events.iter().filter(|e| e.kind == DeliberationKind::Objection)
            .map(|e| e.event_ref.clone()).collect(),
        amendment_refs: packet.events.iter().filter(|e| e.kind == DeliberationKind::Amendment)
            .map(|e| e.event_ref.clone()).collect(),
        recorded_judgement: decision.and_then(|d| d.judgement),
        recorded_consensus_ref: decision.and_then(|d| d.consensus_assessment_ref.clone()),
        recorded_authority_ref: decision.and_then(|d| d.authority_receipt_ref.clone()),
        record_reviewable: holds.is_empty(),
        holds,
        candidate_only: true,
        semantic_truth_promoted: false,
        community_consensus_verified: false,
        actor_authority_verified: false,
        wikidata_edit_authorised: false,
        ao_dispatch_allowed: false,
    })
}

/// Project a source-pinned diagnostic into the *existing* S29 review kind.
/// This does not persist it or bypass SLR source checks. Accepting this
/// review item concerns the diagnostic, never approval of a Wikidata proposal.
pub fn project_s29_diagnostic_review_item(
    assessment: &DeliberationAssessment, consumer_scope_ref: &str,
) -> Result<ReviewItem, DeliberationError> {
    if !nonempty(consumer_scope_ref) {
        return Err(DeliberationError::MissingSource);
    }
    let mut source_refs = assessment.event_source_revision_refs.clone();
    source_refs.push(assessment.source_revision_ref.clone());
    source_refs.sort();
    source_refs.dedup();
    let mut provenance_refs = assessment.event_refs.clone();
    provenance_refs.push(assessment.policy_revision_ref.clone());
    provenance_refs.push(assessment.packet_ref.clone());
    provenance_refs.sort();
    provenance_refs.dedup();
    Ok(ReviewItem {
        review_item_ref: format!("review:{}", assessment.packet_ref),
        semantic_ref: assessment.proposal_ref.clone(),
        item_kind: ReviewItemKind::OntologyDiagnostic,
        reason: "Source-pinned deliberation completeness, not Wikidata approval".into(),
        provenance_refs,
        source_refs,
        current_status: ReviewStatus::Pending,
        available_actions: vec![
            ReviewAction::RequestEvidence, ReviewAction::Qualify,
            ReviewAction::Abstain, ReviewAction::OpenSource,
        ],
        affected_consumer_refs: vec![consumer_scope_ref.into()],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: DeliberationKind, n: i64, actor: &str, t: i64) -> DeliberationEvent {
        DeliberationEvent {
            event_ref: format!("event:{n}"), source_revision_ref: format!("revision:{n}"),
            actor_ref: actor.into(), at_unix_seconds: t, kind,
            supersedes_ref: None, judgement: None,
            consensus_assessment_ref: None, authority_receipt_ref: None,
        }
    }
    fn sample() -> DeliberationPacket {
        let mut decision = event(DeliberationKind::Decision, 4, "reviewer", 8 * 86400);
        decision.judgement = Some(DeliberationJudgement::Approve);
        decision.consensus_assessment_ref = Some("discussion:assessment".into());
        decision.authority_receipt_ref = Some("community:role-snapshot".into());
        DeliberationPacket {
            proposal_ref: "proposal:example".into(),
            source_revision_ref: "revision:archive".into(),
            policy_revision_ref: "policy:historical".into(),
            minimum_age_seconds: 7 * 86400,
            created_property_ref: Some("property:example".into()),
            events: vec![
                event(DeliberationKind::Proposal, 1, "author", 0),
                event(DeliberationKind::Objection, 2, "other", 86400),
                event(DeliberationKind::Amendment, 3, "author", 2 * 86400),
                decision,
            ],
        }
    }

    #[test]
    fn source_events_and_criticisms_survive_without_decision_authority() {
        let packet = sample();
        let a = assess_deliberation(&packet).unwrap();
        assert!(a.record_reviewable);
        assert_eq!(a.objection_refs, vec!["event:2"]);
        assert_eq!(a.amendment_refs, vec!["event:3"]);
        assert_eq!(a.event_source_revision_refs.len(), 4);
        assert_eq!(a.recorded_judgement, Some(DeliberationJudgement::Approve));
        assert!(!a.community_consensus_verified && !a.actor_authority_verified);
        assert!(!a.semantic_truth_promoted && !a.wikidata_edit_authorised && !a.ao_dispatch_allowed);
        let item = project_s29_diagnostic_review_item(&a, "matter:explicit").unwrap();
        assert_eq!(item.item_kind, ReviewItemKind::OntologyDiagnostic);
        assert!(item.validate().is_ok());
        assert!(item.candidate_only && !item.claim_truth_promoted);
        assert!(!item.available_actions.contains(&ReviewAction::Accept));
        assert!(item.source_refs.contains(&"revision:2".to_owned()));
    }

    #[test]
    fn missing_judgement_and_policy_conditions_remain_separate_holds() {
        let mut packet = sample();
        packet.events.pop();
        let a = assess_deliberation(&packet).unwrap();
        assert!(a.holds.contains(&DeliberationHold::NoTerminalDecision));
        assert!(!a.record_reviewable);
        packet = sample();
        packet.events[3].at_unix_seconds = 7 * 86400;
        packet.events[3].consensus_assessment_ref = None;
        let a = assess_deliberation(&packet).unwrap();
        assert!(a.holds.contains(&DeliberationHold::MinimumProposalAgeNotMet));
        assert!(a.holds.contains(&DeliberationHold::ConsensusJudgementSourceMissing));
    }

    #[test]
    fn duplicate_unordered_or_unsourced_events_are_invalid() {
        let mut packet = sample();
        packet.events[1].event_ref = "event:1".into();
        assert_eq!(assess_deliberation(&packet).unwrap_err(), DeliberationError::InvalidEventHistory);
        packet = sample();
        packet.events[1].at_unix_seconds = -1;
        assert_eq!(assess_deliberation(&packet).unwrap_err(), DeliberationError::InvalidEventHistory);
        packet = sample();
        packet.events[1].source_revision_ref.clear();
        assert_eq!(assess_deliberation(&packet).unwrap_err(), DeliberationError::MissingSource);
    }
}
