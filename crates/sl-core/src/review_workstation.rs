//! S29 portable human-review workstation domain.
//!
//! Review is an explicit operator action over an already-owned semantic object.
//! It may change review/workflow state or request more evidence, but it does not
//! itself create proposition truth, legal applicability, or semantic authority.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewItemKind {
    PnfParse,
    Observation,
    ClaimContestation,
    EventAssembly,
    ChronologyAmbiguity,
    AuthorityFollow,
    ResearchAcquisition,
    LegalTreatment,
    ScopeHandoff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewStatus {
    Pending,
    Accepted,
    Rejected,
    Abstained,
    Qualified,
    Superseded,
    NeedsEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewAction {
    Accept,
    Reject,
    Abstain,
    Qualify,
    Supersede,
    RequestEvidence,
    OpenSource,
    FollowAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewItem {
    pub review_item_ref: String,
    pub semantic_ref: String,
    pub item_kind: ReviewItemKind,
    pub reason: String,
    pub provenance_refs: Vec<String>,
    pub source_refs: Vec<String>,
    pub current_status: ReviewStatus,
    pub available_actions: Vec<ReviewAction>,
    pub affected_consumer_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewCommand {
    pub command_ref: String,
    pub review_item_ref: String,
    pub action: ReviewAction,
    pub reviewer_ref: String,
    pub qualification_ref: Option<String>,
    pub evidence_request_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewEffect {
    StatusChanged {
        previous: ReviewStatus,
        current: ReviewStatus,
    },
    EvidenceRequested {
        evidence_request_ref: String,
    },
    SourceOpenRequested,
    AuthorityFollowRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewReceipt {
    pub command_ref: String,
    pub review_item_ref: String,
    pub semantic_ref: String,
    pub reviewer_ref: String,
    pub action: ReviewAction,
    pub effect: ReviewEffect,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewWorkstationError {
    EmptyCoordinate(&'static str),
    EmptyReason,
    NoProvenanceOrSource,
    ActionNotAvailable,
    QualificationRequired,
    EvidenceRequestRequired,
    TerminalStatusTransition,
    PromotionNotAllowed,
}

fn require(name: &'static str, value: &str) -> Result<(), ReviewWorkstationError> {
    if value.trim().is_empty() {
        Err(ReviewWorkstationError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl ReviewItem {
    pub fn validate(&self) -> Result<(), ReviewWorkstationError> {
        require("review_item_ref", &self.review_item_ref)?;
        require("semantic_ref", &self.semantic_ref)?;
        if self.reason.trim().is_empty() {
            return Err(ReviewWorkstationError::EmptyReason);
        }
        if self.provenance_refs.is_empty() && self.source_refs.is_empty() {
            return Err(ReviewWorkstationError::NoProvenanceOrSource);
        }
        if self
            .provenance_refs
            .iter()
            .chain(self.source_refs.iter())
            .chain(self.affected_consumer_refs.iter())
            .any(|value| value.trim().is_empty())
        {
            return Err(ReviewWorkstationError::EmptyCoordinate("review_item_refs"));
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(ReviewWorkstationError::PromotionNotAllowed);
        }
        Ok(())
    }

    #[must_use]
    pub fn offers(&self, action: ReviewAction) -> bool {
        self.available_actions.contains(&action)
    }

    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.current_status,
            ReviewStatus::Rejected | ReviewStatus::Superseded
        )
    }
}

impl ReviewCommand {
    pub fn validate(&self) -> Result<(), ReviewWorkstationError> {
        require("command_ref", &self.command_ref)?;
        require("review_item_ref", &self.review_item_ref)?;
        require("reviewer_ref", &self.reviewer_ref)?;
        if self.action == ReviewAction::Qualify
            && self
                .qualification_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(ReviewWorkstationError::QualificationRequired);
        }
        if self.action == ReviewAction::RequestEvidence
            && self
                .evidence_request_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(ReviewWorkstationError::EvidenceRequestRequired);
        }
        Ok(())
    }
}

pub fn apply_review_command(
    item: &mut ReviewItem,
    command: &ReviewCommand,
) -> Result<ReviewReceipt, ReviewWorkstationError> {
    item.validate()?;
    command.validate()?;
    if item.review_item_ref != command.review_item_ref {
        return Err(ReviewWorkstationError::EmptyCoordinate(
            "review_item_ref_mismatch",
        ));
    }
    if !item.offers(command.action) {
        return Err(ReviewWorkstationError::ActionNotAvailable);
    }
    if item.is_terminal()
        && !matches!(
            command.action,
            ReviewAction::OpenSource | ReviewAction::FollowAuthority
        )
    {
        return Err(ReviewWorkstationError::TerminalStatusTransition);
    }

    let previous = item.current_status;
    let effect = match command.action {
        ReviewAction::Accept => {
            item.current_status = ReviewStatus::Accepted;
            ReviewEffect::StatusChanged {
                previous,
                current: item.current_status,
            }
        }
        ReviewAction::Reject => {
            item.current_status = ReviewStatus::Rejected;
            ReviewEffect::StatusChanged {
                previous,
                current: item.current_status,
            }
        }
        ReviewAction::Abstain => {
            item.current_status = ReviewStatus::Abstained;
            ReviewEffect::StatusChanged {
                previous,
                current: item.current_status,
            }
        }
        ReviewAction::Qualify => {
            item.current_status = ReviewStatus::Qualified;
            ReviewEffect::StatusChanged {
                previous,
                current: item.current_status,
            }
        }
        ReviewAction::Supersede => {
            item.current_status = ReviewStatus::Superseded;
            ReviewEffect::StatusChanged {
                previous,
                current: item.current_status,
            }
        }
        ReviewAction::RequestEvidence => {
            item.current_status = ReviewStatus::NeedsEvidence;
            ReviewEffect::EvidenceRequested {
                evidence_request_ref: command
                    .evidence_request_ref
                    .clone()
                    .expect("validated request evidence ref"),
            }
        }
        ReviewAction::OpenSource => ReviewEffect::SourceOpenRequested,
        ReviewAction::FollowAuthority => ReviewEffect::AuthorityFollowRequested,
    };

    item.validate()?;

    Ok(ReviewReceipt {
        command_ref: command.command_ref.clone(),
        review_item_ref: item.review_item_ref.clone(),
        semantic_ref: item.semantic_ref.clone(),
        reviewer_ref: command.reviewer_ref.clone(),
        action: command.action,
        effect,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> ReviewItem {
        ReviewItem {
            review_item_ref: "review-item:claim:1".into(),
            semantic_ref: "claim:1".into(),
            item_kind: ReviewItemKind::ClaimContestation,
            reason: "competing account requires human review".into(),
            provenance_refs: vec!["relation:a:b".into()],
            source_refs: vec!["statement:a".into(), "statement:b".into()],
            current_status: ReviewStatus::Pending,
            available_actions: vec![
                ReviewAction::Accept,
                ReviewAction::Reject,
                ReviewAction::Abstain,
                ReviewAction::Qualify,
                ReviewAction::RequestEvidence,
                ReviewAction::OpenSource,
            ],
            affected_consumer_refs: vec!["timeline:matter:1".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    fn command(action: ReviewAction) -> ReviewCommand {
        ReviewCommand {
            command_ref: format!("review-command:{action:?}"),
            review_item_ref: "review-item:claim:1".into(),
            action,
            reviewer_ref: "reviewer:1".into(),
            qualification_ref: (action == ReviewAction::Qualify)
                .then(|| "qualification:1".into()),
            evidence_request_ref: (action == ReviewAction::RequestEvidence)
                .then(|| "evidence-request:1".into()),
        }
    }

    #[test]
    fn accept_changes_review_state_but_not_truth() {
        let mut value = item();
        let receipt = apply_review_command(&mut value, &command(ReviewAction::Accept)).unwrap();
        assert_eq!(value.current_status, ReviewStatus::Accepted);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.applicability_promoted);
        assert!(!receipt.claim_truth_promoted);
    }

    #[test]
    fn request_evidence_is_not_rejection_or_negative_finding() {
        let mut value = item();
        let receipt =
            apply_review_command(&mut value, &command(ReviewAction::RequestEvidence)).unwrap();
        assert_eq!(value.current_status, ReviewStatus::NeedsEvidence);
        assert!(matches!(receipt.effect, ReviewEffect::EvidenceRequested { .. }));
        assert!(!receipt.claim_truth_promoted);
    }

    #[test]
    fn open_source_is_navigation_not_status_mutation() {
        let mut value = item();
        let before = value.current_status;
        let receipt =
            apply_review_command(&mut value, &command(ReviewAction::OpenSource)).unwrap();
        assert_eq!(value.current_status, before);
        assert_eq!(receipt.effect, ReviewEffect::SourceOpenRequested);
    }

    #[test]
    fn unavailable_action_fails_closed() {
        let mut value = item();
        assert_eq!(
            apply_review_command(&mut value, &command(ReviewAction::FollowAuthority)),
            Err(ReviewWorkstationError::ActionNotAvailable)
        );
    }

    #[test]
    fn qualification_requires_explicit_qualification_ref() {
        let mut value = item();
        let mut cmd = command(ReviewAction::Qualify);
        cmd.qualification_ref = None;
        assert_eq!(
            apply_review_command(&mut value, &cmd),
            Err(ReviewWorkstationError::QualificationRequired)
        );
    }
}
