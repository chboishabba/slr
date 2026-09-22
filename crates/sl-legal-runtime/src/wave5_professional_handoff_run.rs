//! M9 real Wave-5 scope -> professional projection -> exact recompute receipt.
//!
//! This is the end-to-end machine receipt for the real Wave-5 lineage adapter.
//! Human/governance scope decisions are inputs. The runtime never fabricates
//! them. With no decisions, the reviewed therapist-note coordinate remains
//! unresolved for professional handoff.
//!
//! A reviewed/scoped coordinate delta reopens only consumers whose explicit
//! dependency + scope receipt includes that exact coordinate.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    apply_wave5_scope_decisions, wave5_real_professional_handoff_candidates,
    FactReviewCandidateState, ShareScopeDecisionReceipt, Wave5ScopedProjectionReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wave5ProfessionalHandoffMachineReceipt {
    pub source_run_ref: String,
    pub source_fact_refs: BTreeSet<String>,
    pub source_statement_refs: BTreeSet<String>,
    pub source_refs: BTreeSet<String>,
    pub review_state_by_coordinate: BTreeMap<String, FactReviewCandidateState>,
    pub scope: Wave5ScopedProjectionReceipt,
    pub affected_consumers_after_delta: BTreeMap<String, BTreeSet<String>>,
    pub changed_coordinate_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compile_wave5_professional_handoff_receipt(
    scope_decisions: &[ShareScopeDecisionReceipt],
    changed_coordinate_refs: impl IntoIterator<Item = String>,
) -> Result<Wave5ProfessionalHandoffMachineReceipt, String> {
    let candidates = wave5_real_professional_handoff_candidates();
    let scope = apply_wave5_scope_decisions(scope_decisions)?;
    let by_coordinate = candidates
        .iter()
        .map(|candidate| (candidate.coordinate_ref.clone(), candidate))
        .collect::<BTreeMap<_, _>>();

    let changed_coordinate_refs = changed_coordinate_refs.into_iter().collect::<BTreeSet<_>>();
    let mut affected_consumers_after_delta: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for coordinate_ref in &changed_coordinate_refs {
        let candidate = by_coordinate
            .get(coordinate_ref)
            .ok_or_else(|| format!("delta references unknown Wave-5 coordinate {coordinate_ref}"))?;

        if candidate.state != FactReviewCandidateState::Reviewed {
            return Err(format!(
                "delta coordinate {coordinate_ref} is not a reviewed professional-handoff coordinate"
            ));
        }

        let affected = scope
            .included_by_consumer
            .iter()
            .filter_map(|(consumer_ref, included)| {
                included
                    .contains(coordinate_ref)
                    .then_some(consumer_ref.clone())
            })
            .collect::<BTreeSet<_>>();

        affected_consumers_after_delta.insert(coordinate_ref.clone(), affected);
    }

    Ok(Wave5ProfessionalHandoffMachineReceipt {
        source_run_ref: candidates
            .first()
            .map(|candidate| candidate.run_ref.clone())
            .unwrap_or_default(),
        source_fact_refs: candidates.iter().map(|candidate| candidate.fact_ref.clone()).collect(),
        source_statement_refs: candidates
            .iter()
            .map(|candidate| candidate.statement_ref.clone())
            .collect(),
        source_refs: candidates.iter().map(|candidate| candidate.source_ref.clone()).collect(),
        review_state_by_coordinate: candidates
            .iter()
            .map(|candidate| (candidate.coordinate_ref.clone(), candidate.state))
            .collect(),
        scope,
        affected_consumers_after_delta,
        changed_coordinate_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ShareScopeDecisionKind, DOCTOR_CONSUMER, LAWYER_CONSUMER,
    };

    fn test_scope_decisions() -> Vec<ShareScopeDecisionReceipt> {
        let therapist = wave5_real_professional_handoff_candidates()
            .into_iter()
            .find(|candidate| candidate.coordinate_ref == "coordinate:wave5:therapist-note")
            .unwrap();

        vec![
            ShareScopeDecisionReceipt {
                decision_ref: "scope:test-only:therapist:lawyer".into(),
                coordinate_ref: therapist.coordinate_ref.clone(),
                consumer_ref: LAWYER_CONSUMER.into(),
                source_run_ref: therapist.run_ref.clone(),
                source_fact_ref: therapist.fact_ref.clone(),
                decision: ShareScopeDecisionKind::Allow,
                decision_provenance_ref: "test-fixture:not-a-human-governance-decision".into(),
                reviewed_coordinate_required: true,
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            },
            ShareScopeDecisionReceipt {
                decision_ref: "scope:test-only:therapist:doctor".into(),
                coordinate_ref: therapist.coordinate_ref.clone(),
                consumer_ref: DOCTOR_CONSUMER.into(),
                source_run_ref: therapist.run_ref.clone(),
                source_fact_ref: therapist.fact_ref.clone(),
                decision: ShareScopeDecisionKind::Deny,
                decision_provenance_ref: "test-fixture:not-a-human-governance-decision".into(),
                reviewed_coordinate_required: true,
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            },
        ]
    }

    #[test]
    fn empty_human_scope_input_keeps_reviewed_coordinate_unresolved() {
        let receipt =
            compile_wave5_professional_handoff_receipt(&[], std::iter::empty::<String>())
                .unwrap();
        assert!(receipt
            .scope
            .unresolved_coordinate_refs
            .contains("coordinate:wave5:therapist-note"));
        assert!(receipt.scope.included_by_consumer.is_empty());
    }

    #[test]
    fn test_only_scope_fixture_proves_distinct_professional_fibres() {
        let receipt =
            compile_wave5_professional_handoff_receipt(
                &test_scope_decisions(),
                std::iter::empty::<String>(),
            )
            .unwrap();

        assert!(receipt
            .scope
            .included_by_consumer
            .get(LAWYER_CONSUMER)
            .unwrap()
            .contains("coordinate:wave5:therapist-note"));
        assert!(receipt
            .scope
            .excluded_by_consumer
            .get(DOCTOR_CONSUMER)
            .unwrap()
            .contains("coordinate:wave5:therapist-note"));
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn exact_reviewed_delta_reopens_only_scope_admitted_consumer() {
        let receipt =
            compile_wave5_professional_handoff_receipt(
                &test_scope_decisions(),
                ["coordinate:wave5:therapist-note".to_owned()],
            )
            .unwrap();

        assert_eq!(
            receipt
                .affected_consumers_after_delta
                .get("coordinate:wave5:therapist-note")
                .unwrap(),
            &BTreeSet::from([LAWYER_CONSUMER.to_owned()])
        );
    }

    #[test]
    fn unreviewed_wave5_coordinate_cannot_drive_professional_delta_recompute() {
        let error = compile_wave5_professional_handoff_receipt(
            &test_scope_decisions(),
            ["coordinate:wave5:user-journal-account".to_owned()],
        )
        .unwrap_err();
        assert!(error.contains("not a reviewed professional-handoff coordinate"));
    }
}
