//! S28.AUTO reader projection for candidate event-join proposals.

use sensiblaw_core::event_discovery::CandidateEventJoinProposal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventJoinProposalView {
    pub proposal: CandidateEventJoinProposal,
    pub signal_kind_count: usize,
    pub observation_count: usize,
    pub source_family_count: usize,
    pub candidate_only: bool,
    pub requires_review: bool,
    pub creates_event_identity: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventDiscoveryProjection {
    pub proposals: Vec<EventJoinProposalView>,
    pub candidate_only: bool,
    pub creates_event_identity: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventDiscoveryProjectionError {
    InvalidProposal(String),
}

pub fn project_event_join_proposals(
    proposals: &[CandidateEventJoinProposal],
) -> Result<EventDiscoveryProjection, EventDiscoveryProjectionError> {
    let mut views = Vec::with_capacity(proposals.len());
    for proposal in proposals {
        proposal.validate().map_err(|_| {
            EventDiscoveryProjectionError::InvalidProposal(proposal.proposal_ref.clone())
        })?;
        views.push(EventJoinProposalView {
            proposal: proposal.clone(),
            signal_kind_count: proposal.signal_kind_count(),
            observation_count: proposal.observation_refs.len(),
            source_family_count: proposal.source_family_refs.len(),
            candidate_only: true,
            requires_review: true,
            creates_event_identity: false,
        });
    }

    views.sort_by(|left, right| {
        right
            .signal_kind_count
            .cmp(&left.signal_kind_count)
            .then_with(|| right.observation_count.cmp(&left.observation_count))
            .then_with(|| left.proposal.proposal_ref.cmp(&right.proposal.proposal_ref))
    });

    Ok(EventDiscoveryProjection {
        proposals: views,
        candidate_only: true,
        creates_event_identity: false,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::event_discovery::{
        EventJoinSignal, EventJoinSignalKind,
    };

    #[test]
    fn projection_orders_stronger_candidates_without_promoting_them() {
        let one = CandidateEventJoinProposal {
            proposal_ref: "proposal:one".into(),
            observation_refs: vec!["o1".into(), "o2".into()],
            statement_refs: vec!["s1".into(), "s2".into()],
            source_family_refs: vec!["book".into(), "wiki".into()],
            signals: vec![EventJoinSignal {
                kind: EventJoinSignalKind::SharedQid,
                evidence_ref: "qid:1".into(),
                detector_ref: "detector:1".into(),
            }],
            policy_ref: "policy:1".into(),
            candidate_only: true,
            requires_review: true,
            creates_event_identity: false,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        let mut two = one.clone();
        two.proposal_ref = "proposal:two".into();
        two.signals.push(EventJoinSignal {
            kind: EventJoinSignalKind::SharedTemporalBucket,
            evidence_ref: "time:1".into(),
            detector_ref: "detector:1".into(),
        });

        let projection = project_event_join_proposals(&[one, two]).unwrap();
        assert_eq!(projection.proposals[0].proposal.proposal_ref, "proposal:two");
        assert!(!projection.creates_event_identity);
        assert!(!projection.claim_truth_promoted);
    }
}
