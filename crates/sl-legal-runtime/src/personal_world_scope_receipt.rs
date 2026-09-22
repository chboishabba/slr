//! M9 explicit human/governance share-scope receipts.
//!
//! A review receipt is not a share receipt. A coordinate can be reviewed and
//! still remain unavailable to every professional consumer. This module keeps
//! that boundary explicit and replayable.
//!
//! The runtime never invents consent or professional share scope. A decision
//! must name the coordinate, consumer, decision, decision provenance, and the
//! reviewed source/run lineage it applies to.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    professional_payment_eligible, wave5_real_professional_handoff_candidates,
    FactReviewCandidateState, FactReviewPersonalCandidate, ShareClass,
    ADVOCATE_CONSUMER, DOCTOR_CONSUMER, LAWYER_CONSUMER, REGULATOR_CONSUMER,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ShareScopeDecisionKind {
    Allow,
    Deny,
    NotReady,
    Withdrawn,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareScopeDecisionReceipt {
    pub decision_ref: String,
    pub coordinate_ref: String,
    pub consumer_ref: String,
    pub source_run_ref: String,
    pub source_fact_ref: String,
    pub decision: ShareScopeDecisionKind,
    pub decision_provenance_ref: String,
    pub reviewed_coordinate_required: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wave5ScopedProjectionReceipt {
    pub source_run_ref: String,
    pub decisions: Vec<ShareScopeDecisionReceipt>,
    pub included_by_consumer: BTreeMap<String, BTreeSet<String>>,
    pub excluded_by_consumer: BTreeMap<String, BTreeSet<String>>,
    pub exclusion_reason_by_consumer_coordinate: BTreeMap<(String, String), String>,
    pub unresolved_coordinate_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ShareScopeDecisionReceipt {
    pub fn validate_against(
        &self,
        candidate: &FactReviewPersonalCandidate,
    ) -> Result<(), String> {
        if self.decision_ref.trim().is_empty()
            || self.decision_provenance_ref.trim().is_empty()
            || self.coordinate_ref != candidate.coordinate_ref
            || self.source_run_ref != candidate.run_ref
            || self.source_fact_ref != candidate.fact_ref
        {
            return Err("share-scope receipt does not match candidate lineage".into());
        }
        if !self.reviewed_coordinate_required
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("share-scope receipt crossed non-promotion boundary".into());
        }
        if self.decision == ShareScopeDecisionKind::Allow
            && candidate.state != FactReviewCandidateState::Reviewed
        {
            return Err("cannot allow professional share for unreviewed coordinate".into());
        }
        Ok(())
    }
}

fn role_class(consumer_ref: &str) -> Option<ShareClass> {
    match consumer_ref {
        LAWYER_CONSUMER => Some(ShareClass::Lawyer),
        DOCTOR_CONSUMER => Some(ShareClass::Doctor),
        ADVOCATE_CONSUMER => Some(ShareClass::Advocate),
        REGULATOR_CONSUMER => Some(ShareClass::Regulator),
        _ => None,
    }
}

pub fn apply_wave5_scope_decisions(
    decisions: &[ShareScopeDecisionReceipt],
) -> Result<Wave5ScopedProjectionReceipt, String> {
    let candidates = wave5_real_professional_handoff_candidates();
    let by_coordinate = candidates
        .iter()
        .map(|candidate| (candidate.coordinate_ref.as_str(), candidate))
        .collect::<BTreeMap<_, _>>();

    let mut seen = BTreeSet::new();
    let mut included_by_consumer: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut excluded_by_consumer: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut exclusion_reason_by_consumer_coordinate = BTreeMap::new();

    for decision in decisions {
        let candidate = by_coordinate
            .get(decision.coordinate_ref.as_str())
            .copied()
            .ok_or_else(|| format!("unknown Wave-5 coordinate {}", decision.coordinate_ref))?;
        decision.validate_against(candidate)?;

        let key = (
            decision.consumer_ref.clone(),
            decision.coordinate_ref.clone(),
        );
        if !seen.insert(key.clone()) {
            return Err("duplicate share-scope decision for consumer/coordinate".into());
        }
        if role_class(&decision.consumer_ref).is_none() {
            return Err(format!(
                "unsupported professional consumer {}",
                decision.consumer_ref
            ));
        }

        match decision.decision {
            ShareScopeDecisionKind::Allow => {
                included_by_consumer
                    .entry(decision.consumer_ref.clone())
                    .or_default()
                    .insert(decision.coordinate_ref.clone());
            }
            ShareScopeDecisionKind::Deny => {
                excluded_by_consumer
                    .entry(decision.consumer_ref.clone())
                    .or_default()
                    .insert(decision.coordinate_ref.clone());
                exclusion_reason_by_consumer_coordinate.insert(
                    key,
                    "explicit-scope-denial".into(),
                );
            }
            ShareScopeDecisionKind::NotReady => {
                excluded_by_consumer
                    .entry(decision.consumer_ref.clone())
                    .or_default()
                    .insert(decision.coordinate_ref.clone());
                exclusion_reason_by_consumer_coordinate.insert(
                    key,
                    "explicitly-not-ready-for-share".into(),
                );
            }
            ShareScopeDecisionKind::Withdrawn => {
                excluded_by_consumer
                    .entry(decision.consumer_ref.clone())
                    .or_default()
                    .insert(decision.coordinate_ref.clone());
                exclusion_reason_by_consumer_coordinate.insert(
                    key,
                    "share-scope-withdrawn".into(),
                );
            }
        }
    }

    // Any reviewed candidate without a scope decision remains unresolved for
    // professional handoff. Existence/review does not imply permission.
    let unresolved_coordinate_refs = candidates
        .iter()
        .filter(|candidate| candidate.state == FactReviewCandidateState::Reviewed)
        .filter(|candidate| {
            !decisions
                .iter()
                .any(|decision| decision.coordinate_ref == candidate.coordinate_ref)
        })
        .map(|candidate| candidate.coordinate_ref.clone())
        .collect();

    Ok(Wave5ScopedProjectionReceipt {
        source_run_ref: candidates
            .first()
            .map(|candidate| candidate.run_ref.clone())
            .unwrap_or_default(),
        decisions: decisions.to_vec(),
        included_by_consumer,
        excluded_by_consumer,
        exclusion_reason_by_consumer_coordinate,
        unresolved_coordinate_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

pub fn wave5_scope_gate_open(
    receipt: &Wave5ScopedProjectionReceipt,
    coordinate_ref: &str,
    consumer_ref: &str,
) -> bool {
    receipt
        .included_by_consumer
        .get(consumer_ref)
        .is_some_and(|coordinates| coordinates.contains(coordinate_ref))
}

pub fn wave5_professional_payment_eligible_after_scope(
    receipt: &Wave5ScopedProjectionReceipt,
    candidate: &FactReviewPersonalCandidate,
    consumer_ref: &str,
) -> bool {
    professional_payment_eligible(candidate)
        || (
            candidate.state == FactReviewCandidateState::Reviewed
                && wave5_scope_gate_open(receipt, &candidate.coordinate_ref, consumer_ref)
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_scope_receipt_means_reviewed_wave5_coordinate_remains_unresolved() {
        let receipt = apply_wave5_scope_decisions(&[]).unwrap();
        assert!(receipt
            .unresolved_coordinate_refs
            .contains("coordinate:wave5:therapist-note"));
        assert!(receipt.included_by_consumer.is_empty());
    }

    #[test]
    fn unreviewed_wave5_rows_cannot_be_allowed() {
        let candidate = wave5_real_professional_handoff_candidates()
            .into_iter()
            .find(|candidate| candidate.coordinate_ref == "coordinate:wave5:user-journal-account")
            .unwrap();
        let decision = ShareScopeDecisionReceipt {
            decision_ref: "scope:bad".into(),
            coordinate_ref: candidate.coordinate_ref.clone(),
            consumer_ref: LAWYER_CONSUMER.into(),
            source_run_ref: candidate.run_ref.clone(),
            source_fact_ref: candidate.fact_ref.clone(),
            decision: ShareScopeDecisionKind::Allow,
            decision_provenance_ref: "human-review:test".into(),
            reviewed_coordinate_required: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        assert!(decision.validate_against(&candidate).is_err());
    }

    #[test]
    fn explicit_allow_and_deny_are_distinct_and_replayable() {
        let candidate = wave5_real_professional_handoff_candidates()
            .into_iter()
            .find(|candidate| candidate.coordinate_ref == "coordinate:wave5:therapist-note")
            .unwrap();

        let lawyer = ShareScopeDecisionReceipt {
            decision_ref: "scope:test:therapist:lawyer".into(),
            coordinate_ref: candidate.coordinate_ref.clone(),
            consumer_ref: LAWYER_CONSUMER.into(),
            source_run_ref: candidate.run_ref.clone(),
            source_fact_ref: candidate.fact_ref.clone(),
            decision: ShareScopeDecisionKind::Allow,
            decision_provenance_ref: "human-review:test-fixture".into(),
            reviewed_coordinate_required: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let doctor = ShareScopeDecisionReceipt {
            decision_ref: "scope:test:therapist:doctor".into(),
            coordinate_ref: candidate.coordinate_ref.clone(),
            consumer_ref: DOCTOR_CONSUMER.into(),
            source_run_ref: candidate.run_ref.clone(),
            source_fact_ref: candidate.fact_ref.clone(),
            decision: ShareScopeDecisionKind::Deny,
            decision_provenance_ref: "human-review:test-fixture".into(),
            reviewed_coordinate_required: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };

        let receipt = apply_wave5_scope_decisions(&[lawyer, doctor]).unwrap();
        assert!(wave5_scope_gate_open(
            &receipt,
            &candidate.coordinate_ref,
            LAWYER_CONSUMER
        ));
        assert!(!wave5_scope_gate_open(
            &receipt,
            &candidate.coordinate_ref,
            DOCTOR_CONSUMER
        ));
        assert_eq!(
            receipt
                .exclusion_reason_by_consumer_coordinate
                .get(&(DOCTOR_CONSUMER.into(), candidate.coordinate_ref.clone()))
                .map(String::as_str),
            Some("explicit-scope-denial")
        );
        assert!(!receipt.creates_claim_truth);
    }
}
