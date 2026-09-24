//! S28.SB operational-state bridge.
//!
//! Operational events describe what the operator/system was doing. They are not
//! semantic/world events and they do not pay evidence, applicability, or truth.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationalEventKind {
    Session,
    ToolActivity,
    ResearchActivity,
    ReviewAction,
    GitCommit,
    PullRequest,
    TaskTransition,
    Interruption,
    BrowserActivity,
    EditorActivity,
    CommunicationActivity,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalEvent {
    pub operational_event_ref: String,
    pub producer_event_ref: String,
    pub producer_ref: String,
    pub state_date: String,
    pub start_time_ref: String,
    pub end_time_ref: String,
    pub primary_app_ref: Option<String>,
    pub label: String,
    pub provenance_refs: Vec<String>,
    pub producer_observed: bool,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub kind: OperationalEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationalTargetKind {
    Matter,
    Source,
    Statement,
    ParserReceipt,
    Observation,
    SemanticEvent,
    Proposition,
    Claim,
    Authority,
    ReviewItem,
    Artifact,
    ResearchResidual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationalSemanticRelationKind {
    Opened,
    Observed,
    Edited,
    Parsed,
    Reviewed,
    Researched,
    FollowedAuthority,
    AcceptedReview,
    RejectedReview,
    QualifiedReview,
    RequestedEvidence,
    ProducedArtifact,
    CommittedChange,
    Affected,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OperationalOutstandingKind {
    Carryover,
    InterruptedThread,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalOutstandingState {
    pub operational_state_ref: String,
    pub state_date: String,
    pub subject_ref: String,
    pub label: String,
    pub provenance_refs: Vec<String>,
    pub kind: OperationalOutstandingKind,
    pub producer_observed: bool,
    pub creates_review_pending: bool,
    pub creates_semantic_unresolved: bool,
    pub creates_user_priority: bool,
}

impl OperationalOutstandingState {
    pub fn validate(&self) -> Result<(), OperationalStateError> {
        require("operational_state_ref", &self.operational_state_ref)?;
        require("state_date", &self.state_date)?;
        require("subject_ref", &self.subject_ref)?;
        require("label", &self.label)?;
        if self.provenance_refs.is_empty() {
            return Err(OperationalStateError::MissingProvenance);
        }
        if self.provenance_refs.iter().any(|value| value.trim().is_empty()) {
            return Err(OperationalStateError::EmptyCoordinate("provenance_refs"));
        }
        if !self.producer_observed {
            return Err(OperationalStateError::ProducerObservationRequired);
        }
        if self.creates_review_pending
            || self.creates_semantic_unresolved
            || self.creates_user_priority
        {
            return Err(OperationalStateError::PromotionNotAllowed);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalSemanticLink {
    pub link_ref: String,
    pub operational_event_ref: String,
    pub target_ref: String,
    pub target_kind: OperationalTargetKind,
    pub relation_kind: OperationalSemanticRelationKind,
    pub relationship_receipt_ref: String,
    pub reviewed_link: bool,
    pub creates_semantic_identity: bool,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationalStateError {
    EmptyCoordinate(&'static str),
    MissingProvenance,
    ProducerObservationRequired,
    PromotionNotAllowed,
    LinkReviewRequired,
}

fn require(name: &'static str, value: &str) -> Result<(), OperationalStateError> {
    if value.trim().is_empty() {
        Err(OperationalStateError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl OperationalEvent {
    pub fn validate(&self) -> Result<(), OperationalStateError> {
        require("operational_event_ref", &self.operational_event_ref)?;
        require("producer_event_ref", &self.producer_event_ref)?;
        require("producer_ref", &self.producer_ref)?;
        require("state_date", &self.state_date)?;
        require("start_time_ref", &self.start_time_ref)?;
        require("end_time_ref", &self.end_time_ref)?;
        require("label", &self.label)?;
        if let Some(value) = &self.primary_app_ref {
            require("primary_app_ref", value)?;
        }
        if self.provenance_refs.is_empty() {
            return Err(OperationalStateError::MissingProvenance);
        }
        if self.provenance_refs.iter().any(|value| value.trim().is_empty()) {
            return Err(OperationalStateError::EmptyCoordinate("provenance_refs"));
        }
        if !self.producer_observed {
            return Err(OperationalStateError::ProducerObservationRequired);
        }
        if self.creates_semantic_authority
            || self.pays_evidence
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(OperationalStateError::PromotionNotAllowed);
        }
        Ok(())
    }
}

impl OperationalSemanticLink {
    pub fn validate(&self) -> Result<(), OperationalStateError> {
        require("link_ref", &self.link_ref)?;
        require("operational_event_ref", &self.operational_event_ref)?;
        require("target_ref", &self.target_ref)?;
        require("relationship_receipt_ref", &self.relationship_receipt_ref)?;
        if !self.reviewed_link {
            return Err(OperationalStateError::LinkReviewRequired);
        }
        if self.creates_semantic_identity
            || self.creates_semantic_authority
            || self.pays_evidence
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(OperationalStateError::PromotionNotAllowed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operational_event() -> OperationalEvent {
        OperationalEvent {
            operational_event_ref: "operational:sb:act-000001".into(),
            producer_event_ref: "act-000001".into(),
            producer_ref: "statibaker:sb.sessionize.v0".into(),
            state_date: "2026-09-24".into(),
            start_time_ref: "2026-09-24T14:01:00+10:00".into(),
            end_time_ref: "2026-09-24T14:07:00+10:00".into(),
            primary_app_ref: Some("app:chatgpt".into()),
            label: "ChatGPT activity".into(),
            provenance_refs: vec![
                "sha256:input".into(),
                "policy:sb.sessionize.v0".into(),
            ],
            producer_observed: true,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            kind: OperationalEventKind::Session,
        }
    }

    #[test]
    fn operational_event_is_not_semantic_authority() {
        let event = operational_event();
        event.validate().unwrap();
        assert!(!event.creates_semantic_authority);
        assert!(!event.pays_evidence);
        assert!(!event.claim_truth_promoted);
    }

    #[test]
    fn operational_semantic_link_requires_review_and_remains_non_promoting() {
        let link = OperationalSemanticLink {
            link_ref: "op-sem-link:1".into(),
            operational_event_ref: "operational:sb:act-000001".into(),
            target_ref: "statement:1".into(),
            target_kind: OperationalTargetKind::Statement,
            relation_kind: OperationalSemanticRelationKind::Reviewed,
            relationship_receipt_ref: "review:op-sem-link:1".into(),
            reviewed_link: true,
            creates_semantic_identity: false,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        link.validate().unwrap();
    }

    #[test]
    fn operational_unresolved_does_not_create_review_semantic_or_priority_state() {
        let value = OperationalOutstandingState {
            operational_state_ref: "operational-unresolved:1".into(),
            state_date: "2026-09-24".into(),
            subject_ref: "authority-follow:1".into(),
            label: "authority follow remained unresolved".into(),
            provenance_refs: vec!["statibaker:carryover:1".into()],
            kind: OperationalOutstandingKind::Unresolved,
            producer_observed: true,
            creates_review_pending: false,
            creates_semantic_unresolved: false,
            creates_user_priority: false,
        };
        value.validate().unwrap();
        assert!(!value.creates_review_pending);
        assert!(!value.creates_semantic_unresolved);
        assert!(!value.creates_user_priority);
    }

    #[test]
    fn opening_source_does_not_pay_evidence() {
        let mut link = OperationalSemanticLink {
            link_ref: "op-sem-link:open".into(),
            operational_event_ref: "operational:sb:act-000001".into(),
            target_ref: "source:1".into(),
            target_kind: OperationalTargetKind::Source,
            relation_kind: OperationalSemanticRelationKind::Opened,
            relationship_receipt_ref: "review:open-link".into(),
            reviewed_link: true,
            creates_semantic_identity: false,
            creates_semantic_authority: false,
            pays_evidence: true,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        assert_eq!(
            link.validate(),
            Err(OperationalStateError::PromotionNotAllowed)
        );
        link.pays_evidence = false;
        assert!(link.validate().is_ok());
    }
}
