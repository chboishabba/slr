//! S21.3 permanent Pabai-style adversarial recurrence regression.
//!
//! This fixture is deliberately tiny and generic: support makes a candidate
//! route reachable; a reviewed defeater closes it; an exact reviewed
//! counter-defeater reopens it; the reopened route immediately emits fresh
//! defeater search.  It is the stable behavioural regression for S20 changes.

use crate::{
    compile_reviewed_treatment_rerun, AdversarialSearchRole, CandidateRouteStatus,
    ReviewedTreatmentCoordinate, ReviewedTreatmentRole,
};

pub const PABAI_GOLDEN_CONSUMER: &str = "consumer:pabai-climate-duty";
pub const PABAI_GOLDEN_ROUTE: &str = "route:pabai:golden-adversarial-recurrence";
pub const PABAI_GOLDEN_TARGET: &str = "proposition:pabai:duty-route-candidate";

fn treatment(
    treatment_ref: &str,
    coordinate_ref: &str,
    proposition_ref: &str,
    role: ReviewedTreatmentRole,
) -> ReviewedTreatmentCoordinate {
    ReviewedTreatmentCoordinate {
        treatment_ref: treatment_ref.into(),
        coordinate_ref: coordinate_ref.into(),
        source_ref: format!("source:pabai:regression:{treatment_ref}"),
        proposition_ref: proposition_ref.into(),
        cited_authority_ref: "case:au:fca:2025:796".into(),
        role,
        reviewed: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn run_pabai_golden_regression() -> Result<crate::AdversarialRerunReceipt, String> {
    compile_reviewed_treatment_rerun(
        PABAI_GOLDEN_CONSUMER,
        PABAI_GOLDEN_ROUTE,
        PABAI_GOLDEN_TARGET,
        &[
            treatment(
                "support",
                "coordinate:pabai:golden:support",
                "proposition:pabai:golden:support",
                ReviewedTreatmentRole::Support,
            ),
            treatment(
                "defeater",
                "coordinate:pabai:golden:defeater",
                "proposition:pabai:golden:defeater",
                ReviewedTreatmentRole::Defeater,
            ),
            treatment(
                "counter-defeater",
                "coordinate:pabai:golden:defeater",
                "proposition:pabai:golden:distinction",
                ReviewedTreatmentRole::CounterDefeater,
            ),
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pabai_regression_reopens_then_attacks_again() {
        let receipt = run_pabai_golden_regression().unwrap();
        assert_eq!(
            receipt.route_status,
            CandidateRouteStatus::ReachableCandidate
        );
        assert!(receipt.active_defeater_refs.is_empty());
        assert_eq!(receipt.counter_defeater_refs.len(), 1);
        assert!(receipt
            .next_demands
            .iter()
            .any(|demand| demand.role == AdversarialSearchRole::Defeater));
        assert!(!receipt.creates_claim_truth);
    }
}
