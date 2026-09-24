//! S29 UI-independent review queue projection.
//!
//! This module sorts and filters already-owned ReviewItems. It does not apply
//! review commands or create semantic/evidential authority.

use std::collections::BTreeMap;

use sensiblaw_core::review_workstation::{ReviewItem, ReviewItemKind, ReviewStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewQueueProjection {
    pub items: Vec<ReviewItem>,
    pub count_by_kind: BTreeMap<ReviewItemKind, usize>,
    pub count_by_status: BTreeMap<ReviewStatusKey, usize>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReviewStatusKey {
    Pending,
    Accepted,
    Rejected,
    Abstained,
    Qualified,
    Superseded,
    NeedsEvidence,
}

impl From<ReviewStatus> for ReviewStatusKey {
    fn from(value: ReviewStatus) -> Self {
        match value {
            ReviewStatus::Pending => Self::Pending,
            ReviewStatus::Accepted => Self::Accepted,
            ReviewStatus::Rejected => Self::Rejected,
            ReviewStatus::Abstained => Self::Abstained,
            ReviewStatus::Qualified => Self::Qualified,
            ReviewStatus::Superseded => Self::Superseded,
            ReviewStatus::NeedsEvidence => Self::NeedsEvidence,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewQueueError {
    InvalidItem(String),
}

pub fn project_review_queue(
    items: &[ReviewItem],
) -> Result<ReviewQueueProjection, ReviewQueueError> {
    let mut selected = Vec::with_capacity(items.len());
    let mut count_by_kind = BTreeMap::new();
    let mut count_by_status = BTreeMap::new();

    for item in items {
        item.validate()
            .map_err(|_| ReviewQueueError::InvalidItem(item.review_item_ref.clone()))?;
        selected.push(item.clone());
        *count_by_kind.entry(item.item_kind).or_insert(0) += 1;
        *count_by_status
            .entry(ReviewStatusKey::from(item.current_status))
            .or_insert(0) += 1;
    }

    selected.sort_by(|left, right| {
        review_priority(left.current_status)
            .cmp(&review_priority(right.current_status))
            .then_with(|| left.item_kind.cmp(&right.item_kind))
            .then_with(|| left.review_item_ref.cmp(&right.review_item_ref))
    });

    Ok(ReviewQueueProjection {
        items: selected,
        count_by_kind,
        count_by_status,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

fn review_priority(status: ReviewStatus) -> u8 {
    match status {
        ReviewStatus::NeedsEvidence => 0,
        ReviewStatus::Pending => 1,
        ReviewStatus::Qualified => 2,
        ReviewStatus::Abstained => 3,
        ReviewStatus::Accepted => 4,
        ReviewStatus::Rejected => 5,
        ReviewStatus::Superseded => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::review_workstation::{ReviewAction, ReviewItemKind};

    fn item(reference: &str, kind: ReviewItemKind, status: ReviewStatus) -> ReviewItem {
        ReviewItem {
            review_item_ref: reference.into(),
            semantic_ref: format!("semantic:{reference}"),
            item_kind: kind,
            reason: "review required".into(),
            provenance_refs: vec![format!("provenance:{reference}")],
            source_refs: vec![format!("source:{reference}")],
            current_status: status,
            available_actions: vec![ReviewAction::OpenSource],
            affected_consumer_refs: vec![],
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn queue_prioritises_needs_evidence_and_pending_without_changing_semantics() {
        let projection = project_review_queue(&[
            item(
                "accepted",
                ReviewItemKind::Observation,
                ReviewStatus::Accepted,
            ),
            item(
                "evidence",
                ReviewItemKind::ClaimContestation,
                ReviewStatus::NeedsEvidence,
            ),
            item(
                "pending",
                ReviewItemKind::PnfParse,
                ReviewStatus::Pending,
            ),
        ])
        .unwrap();

        assert_eq!(projection.items[0].review_item_ref, "evidence");
        assert_eq!(projection.items[1].review_item_ref, "pending");
        assert!(!projection.creates_semantic_authority);
        assert!(!projection.applicability_promoted);
        assert!(!projection.claim_truth_promoted);
    }

    #[test]
    fn queue_retains_portable_item_kinds() {
        let projection = project_review_queue(&[
            item("parse", ReviewItemKind::PnfParse, ReviewStatus::Pending),
            item(
                "chronology",
                ReviewItemKind::ChronologyAmbiguity,
                ReviewStatus::Pending,
            ),
            item(
                "authority",
                ReviewItemKind::AuthorityFollow,
                ReviewStatus::Pending,
            ),
            item(
                "handoff",
                ReviewItemKind::ScopeHandoff,
                ReviewStatus::Pending,
            ),
        ])
        .unwrap();
        assert_eq!(projection.items.len(), 4);
        assert_eq!(
            projection.count_by_kind[&ReviewItemKind::PnfParse],
            1
        );
        assert_eq!(
            projection.count_by_kind[&ReviewItemKind::ScopeHandoff],
            1
        );
    }
}
