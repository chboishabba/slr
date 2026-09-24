//! M9 adapter from ITIR fact-review rows into personal-world coordinate candidates.
//!
//! Source class and review state are preserved.  Share permission is NOT inferred
//! from source type, professional authorship, or mere presence in a handoff
//! workbench.  A downstream scope decision must still admit any professional use.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{ShareClass, SharedCoordinateKind};

pub const WAVE5_REAL_RUN_REF: &str =
    "factrun:7c3275b6c4d303154ddf3b0ec11f20fde5a740466421c23a2ac9a547d15ba37d";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactReviewCandidateState {
    Reviewed,
    Unreviewed,
    Abstained,
    NotReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactReviewPersonalCandidate {
    pub run_ref: String,
    pub fact_ref: String,
    pub source_ref: String,
    pub statement_ref: String,
    pub coordinate_ref: String,
    pub kind: SharedCoordinateKind,
    pub source_signal_classes: BTreeSet<String>,
    pub state: FactReviewCandidateState,
    pub proposed_share_classes: BTreeSet<ShareClass>,
    pub share_scope_review_required: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn wave5_real_professional_handoff_candidates() -> Vec<FactReviewPersonalCandidate> {
    vec![
        FactReviewPersonalCandidate {
            run_ref: WAVE5_REAL_RUN_REF.into(),
            fact_ref: "fact:f5b42c5d8feb0ec6".into(),
            source_ref: "src:aae9767bfc03c41a".into(),
            statement_ref: "statement:0b109faaec28961a".into(),
            coordinate_ref: "coordinate:wave5:clinic-letter".into(),
            kind: SharedCoordinateKind::Evidence,
            source_signal_classes: BTreeSet::from([
                "documentary_record".into(),
                "third_party_record".into(),
            ]),
            state: FactReviewCandidateState::Unreviewed,
            proposed_share_classes: BTreeSet::new(),
            share_scope_review_required: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        FactReviewPersonalCandidate {
            run_ref: WAVE5_REAL_RUN_REF.into(),
            fact_ref: "fact:e0667b55061037f4".into(),
            source_ref: "src:816003715d9b4ab0".into(),
            statement_ref: "statement:9d022f16a280c923".into(),
            coordinate_ref: "coordinate:wave5:user-journal-account".into(),
            kind: SharedCoordinateKind::Context,
            source_signal_classes: BTreeSet::from([
                "client_account".into(),
                "user_authored".into(),
            ]),
            state: FactReviewCandidateState::Unreviewed,
            proposed_share_classes: BTreeSet::from([ShareClass::PersonalOnly]),
            share_scope_review_required: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        FactReviewPersonalCandidate {
            run_ref: WAVE5_REAL_RUN_REF.into(),
            fact_ref: "fact:2499c1b666ccd133".into(),
            source_ref: "src:5e78e41d87374808".into(),
            statement_ref: "statement:6a76d26b680c03e9".into(),
            coordinate_ref: "coordinate:wave5:therapist-note".into(),
            kind: SharedCoordinateKind::Context,
            source_signal_classes: BTreeSet::from([
                "later_annotation".into(),
                "professional_interpretation".into(),
                "professional_note".into(),
            ]),
            state: FactReviewCandidateState::Reviewed,
            // Reviewed source state still does not decide share scope.
            proposed_share_classes: BTreeSet::new(),
            share_scope_review_required: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
    ]
}

pub fn professional_payment_eligible(
    candidate: &FactReviewPersonalCandidate,
) -> bool {
    candidate.state == FactReviewCandidateState::Reviewed
        && !candidate.share_scope_review_required
        && candidate
            .proposed_share_classes
            .contains(&ShareClass::SelectedProfessional)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_wave5_lineage_and_source_classes_are_preserved() {
        let candidates = wave5_real_professional_handoff_candidates();
        assert_eq!(candidates.len(), 3);

        let journal = candidates
            .iter()
            .find(|item| item.fact_ref == "fact:e0667b55061037f4")
            .unwrap();
        assert_eq!(journal.state, FactReviewCandidateState::Unreviewed);
        assert!(journal.source_signal_classes.contains("user_authored"));
        assert!(journal.source_signal_classes.contains("client_account"));

        let therapist = candidates
            .iter()
            .find(|item| item.fact_ref == "fact:2499c1b666ccd133")
            .unwrap();
        assert_eq!(therapist.state, FactReviewCandidateState::Reviewed);
        assert!(therapist
            .source_signal_classes
            .contains("professional_interpretation"));
    }

    #[test]
    fn clinic_and_journal_do_not_become_professional_payments() {
        for candidate in wave5_real_professional_handoff_candidates()
            .iter()
            .filter(|item| {
                item.fact_ref == "fact:f5b42c5d8feb0ec6"
                    || item.fact_ref == "fact:e0667b55061037f4"
            })
        {
            assert_eq!(candidate.state, FactReviewCandidateState::Unreviewed);
            assert!(!professional_payment_eligible(candidate));
        }
    }

    #[test]
    fn reviewed_professional_note_still_requires_explicit_share_scope() {
        let therapist = wave5_real_professional_handoff_candidates()
            .into_iter()
            .find(|item| item.fact_ref == "fact:2499c1b666ccd133")
            .unwrap();
        assert_eq!(therapist.state, FactReviewCandidateState::Reviewed);
        assert!(therapist.share_scope_review_required);
        assert!(!professional_payment_eligible(&therapist));
        assert!(!therapist.creates_claim_truth);
    }
}
