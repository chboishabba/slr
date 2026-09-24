//! S28.AUTO candidate event discovery.
//!
//! Automatic discovery is deliberately separated from canonical event identity.
//! The detector may propose that observations describe the same incident, but
//! only a later reviewed assembly may materialise observation -> event links.

use std::collections::{BTreeMap, BTreeSet};

use crate::review_workstation::{
    ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventJoinSignalKind {
    SharedQid,
    SharedEntity,
    SharedTemporalBucket,
    SharedFingerprint,
    ExplicitCrossReference,
    UserDeclaredSameIncident,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventJoinSignal {
    pub kind: EventJoinSignalKind,
    pub evidence_ref: String,
    pub detector_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventJoinObservation {
    pub observation_ref: String,
    pub statement_refs: Vec<String>,
    pub source_family_ref: String,
    pub qid_refs: Vec<String>,
    pub entity_refs: Vec<String>,
    pub temporal_bucket_refs: Vec<String>,
    pub fingerprint_refs: Vec<String>,
    pub explicit_cross_reference_refs: Vec<String>,
    pub user_declared_same_incident_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventDiscoveryPolicy {
    pub policy_ref: String,
    pub detector_ref: String,
    pub min_independent_signal_kinds: usize,
    pub require_cross_source: bool,
}

impl Default for EventDiscoveryPolicy {
    fn default() -> Self {
        Self {
            policy_ref: "event-discovery-policy:v1".into(),
            detector_ref: "slr:event-discovery:v1".into(),
            min_independent_signal_kinds: 2,
            require_cross_source: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEventJoinProposal {
    pub proposal_ref: String,
    pub observation_refs: Vec<String>,
    pub statement_refs: Vec<String>,
    pub source_family_refs: Vec<String>,
    pub signals: Vec<EventJoinSignal>,
    pub policy_ref: String,
    pub candidate_only: bool,
    pub requires_review: bool,
    pub creates_event_identity: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventDiscoveryError {
    EmptyCoordinate(&'static str),
    InvalidPolicy,
    DuplicateObservationRef(String),
    InsufficientObservations,
    InsufficientSignals,
    PromotionNotAllowed,
}

fn require(name: &'static str, value: &str) -> Result<(), EventDiscoveryError> {
    if value.trim().is_empty() {
        Err(EventDiscoveryError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn validate_refs(name: &'static str, refs: &[String]) -> Result<(), EventDiscoveryError> {
    if refs.iter().any(|value| value.trim().is_empty()) {
        Err(EventDiscoveryError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl EventJoinObservation {
    pub fn validate(&self) -> Result<(), EventDiscoveryError> {
        require("observation_ref", &self.observation_ref)?;
        require("source_family_ref", &self.source_family_ref)?;
        validate_refs("statement_refs", &self.statement_refs)?;
        validate_refs("qid_refs", &self.qid_refs)?;
        validate_refs("entity_refs", &self.entity_refs)?;
        validate_refs("temporal_bucket_refs", &self.temporal_bucket_refs)?;
        validate_refs("fingerprint_refs", &self.fingerprint_refs)?;
        validate_refs(
            "explicit_cross_reference_refs",
            &self.explicit_cross_reference_refs,
        )?;
        validate_refs(
            "user_declared_same_incident_refs",
            &self.user_declared_same_incident_refs,
        )?;
        Ok(())
    }
}

impl EventDiscoveryPolicy {
    pub fn validate(&self) -> Result<(), EventDiscoveryError> {
        require("policy_ref", &self.policy_ref)?;
        require("detector_ref", &self.detector_ref)?;
        if self.min_independent_signal_kinds == 0 {
            return Err(EventDiscoveryError::InvalidPolicy);
        }
        Ok(())
    }
}

impl CandidateEventJoinProposal {
    pub fn validate(&self) -> Result<(), EventDiscoveryError> {
        require("proposal_ref", &self.proposal_ref)?;
        require("policy_ref", &self.policy_ref)?;
        if self.observation_refs.len() < 2 {
            return Err(EventDiscoveryError::InsufficientObservations);
        }
        validate_refs("observation_refs", &self.observation_refs)?;
        validate_refs("statement_refs", &self.statement_refs)?;
        validate_refs("source_family_refs", &self.source_family_refs)?;
        if self.signals.is_empty() {
            return Err(EventDiscoveryError::InsufficientSignals);
        }
        for signal in &self.signals {
            require("signal_evidence_ref", &signal.evidence_ref)?;
            require("signal_detector_ref", &signal.detector_ref)?;
        }
        if !self.candidate_only
            || !self.requires_review
            || self.creates_event_identity
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(EventDiscoveryError::PromotionNotAllowed);
        }
        Ok(())
    }

    pub fn to_review_item(&self, affected_consumer_refs: Vec<String>) -> ReviewItem {
        ReviewItem {
            review_item_ref: format!("review-item:{}", self.proposal_ref),
            semantic_ref: self.proposal_ref.clone(),
            item_kind: ReviewItemKind::EventAssembly,
            reason: format!(
                "automatic event-join proposal has {} independent signal kinds across {} observations",
                self.signal_kind_count(),
                self.observation_refs.len()
            ),
            provenance_refs: self
                .signals
                .iter()
                .map(|signal| signal.evidence_ref.clone())
                .collect(),
            source_refs: self.statement_refs.clone(),
            current_status: ReviewStatus::Pending,
            available_actions: vec![
                ReviewAction::Accept,
                ReviewAction::Reject,
                ReviewAction::Abstain,
                ReviewAction::Qualify,
                ReviewAction::RequestEvidence,
                ReviewAction::OpenSource,
            ],
            affected_consumer_refs,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[must_use]
    pub fn signal_kind_count(&self) -> usize {
        self.signals
            .iter()
            .map(|signal| signal.kind)
            .collect::<BTreeSet<_>>()
            .len()
    }
}

pub fn discover_event_join_proposals(
    observations: &[EventJoinObservation],
    policy: &EventDiscoveryPolicy,
) -> Result<Vec<CandidateEventJoinProposal>, EventDiscoveryError> {
    policy.validate()?;

    let mut by_ref = BTreeMap::new();
    for observation in observations {
        observation.validate()?;
        if by_ref
            .insert(observation.observation_ref.clone(), observation)
            .is_some()
        {
            return Err(EventDiscoveryError::DuplicateObservationRef(
                observation.observation_ref.clone(),
            ));
        }
    }

    let ordered = by_ref.into_values().collect::<Vec<_>>();
    let mut proposals = Vec::new();

    for i in 0..ordered.len() {
        for j in (i + 1)..ordered.len() {
            let left = ordered[i];
            let right = ordered[j];

            if policy.require_cross_source
                && left.source_family_ref == right.source_family_ref
            {
                continue;
            }

            let mut signals = Vec::new();
            signals.extend(intersection_signals(
                EventJoinSignalKind::SharedQid,
                &left.qid_refs,
                &right.qid_refs,
                &policy.detector_ref,
                "qid",
            ));
            signals.extend(intersection_signals(
                EventJoinSignalKind::SharedEntity,
                &left.entity_refs,
                &right.entity_refs,
                &policy.detector_ref,
                "entity",
            ));
            signals.extend(intersection_signals(
                EventJoinSignalKind::SharedTemporalBucket,
                &left.temporal_bucket_refs,
                &right.temporal_bucket_refs,
                &policy.detector_ref,
                "temporal",
            ));
            signals.extend(intersection_signals(
                EventJoinSignalKind::SharedFingerprint,
                &left.fingerprint_refs,
                &right.fingerprint_refs,
                &policy.detector_ref,
                "fingerprint",
            ));
            signals.extend(intersection_signals(
                EventJoinSignalKind::ExplicitCrossReference,
                &left.explicit_cross_reference_refs,
                &right.explicit_cross_reference_refs,
                &policy.detector_ref,
                "crossref",
            ));
            signals.extend(intersection_signals(
                EventJoinSignalKind::UserDeclaredSameIncident,
                &left.user_declared_same_incident_refs,
                &right.user_declared_same_incident_refs,
                &policy.detector_ref,
                "user-declared",
            ));
            signals.sort();
            signals.dedup();

            let signal_kind_count = signals
                .iter()
                .map(|signal| signal.kind)
                .collect::<BTreeSet<_>>()
                .len();
            if signal_kind_count < policy.min_independent_signal_kinds {
                continue;
            }

            let mut observation_refs = vec![
                left.observation_ref.clone(),
                right.observation_ref.clone(),
            ];
            observation_refs.sort();

            let mut statement_refs = left
                .statement_refs
                .iter()
                .chain(right.statement_refs.iter())
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            statement_refs.sort();

            let mut source_family_refs = vec![
                left.source_family_ref.clone(),
                right.source_family_ref.clone(),
            ];
            source_family_refs.sort();
            source_family_refs.dedup();

            let proposal_ref = format!(
                "event-join-proposal:{}:{}",
                observation_refs[0], observation_refs[1]
            );
            let proposal = CandidateEventJoinProposal {
                proposal_ref,
                observation_refs,
                statement_refs,
                source_family_refs,
                signals,
                policy_ref: policy.policy_ref.clone(),
                candidate_only: true,
                requires_review: true,
                creates_event_identity: false,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            };
            proposal.validate()?;
            proposals.push(proposal);
        }
    }

    proposals.sort_by(|left, right| left.proposal_ref.cmp(&right.proposal_ref));
    Ok(proposals)
}

fn intersection_signals(
    kind: EventJoinSignalKind,
    left: &[String],
    right: &[String],
    detector_ref: &str,
    prefix: &str,
) -> Vec<EventJoinSignal> {
    let right = right.iter().map(String::as_str).collect::<BTreeSet<_>>();
    left.iter()
        .filter(|value| right.contains(value.as_str()))
        .map(|value| EventJoinSignal {
            kind,
            evidence_ref: format!("event-signal:{prefix}:{value}"),
            detector_ref: detector_ref.to_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        reference: &str,
        family: &str,
        qids: &[&str],
        entities: &[&str],
        temporal: &[&str],
        fingerprints: &[&str],
    ) -> EventJoinObservation {
        EventJoinObservation {
            observation_ref: reference.into(),
            statement_refs: vec![format!("statement:{reference}")],
            source_family_ref: family.into(),
            qid_refs: qids.iter().map(|value| (*value).into()).collect(),
            entity_refs: entities.iter().map(|value| (*value).into()).collect(),
            temporal_bucket_refs: temporal.iter().map(|value| (*value).into()).collect(),
            fingerprint_refs: fingerprints
                .iter()
                .map(|value| (*value).into())
                .collect(),
            explicit_cross_reference_refs: vec![],
            user_declared_same_incident_refs: vec![],
        }
    }

    #[test]
    fn same_qid_alone_does_not_propose_event_join() {
        let proposals = discover_event_join_proposals(
            &[
                observation("a", "memoir", &["Q1"], &[], &[], &[]),
                observation("b", "wiki", &["Q1"], &[], &[], &[]),
            ],
            &EventDiscoveryPolicy::default(),
        )
        .unwrap();
        assert!(proposals.is_empty());
    }

    #[test]
    fn same_person_and_date_proposes_review_but_does_not_create_event() {
        let proposals = discover_event_join_proposals(
            &[
                observation("a", "memoir", &[], &["person:1"], &["date:1"], &[]),
                observation("b", "wiki", &[], &["person:1"], &["date:1"], &[]),
            ],
            &EventDiscoveryPolicy::default(),
        )
        .unwrap();
        assert_eq!(proposals.len(), 1);
        let proposal = &proposals[0];
        assert!(proposal.requires_review);
        assert!(!proposal.creates_event_identity);
        assert_eq!(proposal.signal_kind_count(), 2);
        let review = proposal.to_review_item(vec!["timeline:matter:1".into()]);
        assert_eq!(review.item_kind, ReviewItemKind::EventAssembly);
        assert_eq!(review.current_status, ReviewStatus::Pending);
    }

    #[test]
    fn similar_text_plus_qid_is_still_candidate_only() {
        let proposals = discover_event_join_proposals(
            &[
                observation("a", "book", &["Q1"], &[], &[], &["fp:abc"]),
                observation("b", "legal", &["Q1"], &[], &[], &["fp:abc"]),
            ],
            &EventDiscoveryPolicy::default(),
        )
        .unwrap();
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].candidate_only);
        assert!(!proposals[0].creates_semantic_authority);
        assert!(!proposals[0].claim_truth_promoted);
    }

    #[test]
    fn default_policy_requires_cross_source_overlap() {
        let proposals = discover_event_join_proposals(
            &[
                observation("a", "wiki", &["Q1"], &["person:1"], &[], &[]),
                observation("b", "wiki", &["Q1"], &["person:1"], &[], &[]),
            ],
            &EventDiscoveryPolicy::default(),
        )
        .unwrap();
        assert!(proposals.is_empty());
    }
}
