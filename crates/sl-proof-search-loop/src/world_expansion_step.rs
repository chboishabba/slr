//! One reviewed residual-driven world-expansion step.
//!
//! This module is orchestration glue over the existing ProofFrontier and
//! world_expansion controller. It does not infer residual classes from strings,
//! perform acquisition, parse PNF, or grant proof/legal authority. The caller
//! supplies an explicit PNF/world routing classification for one exact open
//! residual, then this step selects the best already-declared candidate and
//! accounts for the explicit review/disambiguation result.

use crate::frontier::{ProofFrontier, ResidualStatus};
use crate::world_expansion::{
    review_admission, select_for_proof_residual, AdmissionReceipt, DisambiguationOutcome,
    ExpansionCandidate, ProducerLane, ResidualClass, ReviewDecision, WorldExpansionLedger,
    WorldExpansionPolicy,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualRouting {
    pub residual_ref: String,
    pub residual_class: ResidualClass,
    pub routing_reason_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldExpansionStepReceipt {
    pub residual_ref: String,
    pub residual_class: ResidualClass,
    pub routing_reason_ref: String,
    pub selected_candidate_ref: String,
    pub selected_object_ref: String,
    pub selected_producer_lane: ProducerLane,
    pub expected_residual_contraction: u64,
    pub admission: AdmissionReceipt,
    pub total_new_world_objects: usize,
    pub target_novel_objects: usize,
    pub target_complete: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldExpansionStepError {
    ResidualNotFound,
    ResidualNotOpen,
    NoCandidateMeetsThreshold,
}

/// Execute one explicitly reviewed world-expansion step for one exact open
/// residual. Cross-residual prioritisation stays with the existing frontier /
/// Ibrahim policy; this function only binds a supplied routed residual to its
/// candidate set and records the resulting admission.
pub fn execute_reviewed_expansion_step(
    frontier: &ProofFrontier,
    routing: &ResidualRouting,
    candidates: &[ExpansionCandidate],
    ledger: &mut WorldExpansionLedger,
    policy: WorldExpansionPolicy,
    review_decision: ReviewDecision,
    disambiguation_outcome: DisambiguationOutcome,
) -> Result<WorldExpansionStepReceipt, WorldExpansionStepError> {
    let residual = frontier
        .residuals
        .iter()
        .find(|residual| residual.residual_ref == routing.residual_ref)
        .ok_or(WorldExpansionStepError::ResidualNotFound)?;

    if residual.status != ResidualStatus::Open {
        return Err(WorldExpansionStepError::ResidualNotOpen);
    }

    let selected = select_for_proof_residual(
        residual,
        routing.residual_class,
        candidates,
        policy.minimum_expected_residual_contraction,
    )
    .ok_or(WorldExpansionStepError::NoCandidateMeetsThreshold)?;

    let admission = review_admission(
        ledger,
        selected,
        review_decision,
        disambiguation_outcome,
    );

    Ok(WorldExpansionStepReceipt {
        residual_ref: residual.residual_ref.clone(),
        residual_class: routing.residual_class,
        routing_reason_ref: routing.routing_reason_ref.clone(),
        selected_candidate_ref: selected.candidate_ref.clone(),
        selected_object_ref: selected.object_ref.clone(),
        selected_producer_lane: selected.producer_lane,
        expected_residual_contraction: selected.expected_residual_contraction,
        admission,
        total_new_world_objects: ledger.total_new_world_objects,
        target_novel_objects: policy.target_novel_objects,
        target_complete: policy.complete(ledger),
        receipt_authority: "candidate_world_expansion_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofResidual, ResidualStatus};
    use crate::world_expansion::{
        KnowledgeObjectKind, mabo_world_expansion_policy,
    };

    fn frontier(status: ResidualStatus) -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:mabo-100-object-world".into(),
            frontier_ref: "frontier:mabo:1".into(),
            residuals: vec![ProofResidual {
                residual_ref: "residual:mabo:authority-source".into(),
                proposition_ref: "mabo:proposition:radical-title-native-title".into(),
                producer_class_ref: "producer:exact-primary-authority".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("official-primary-case".into()),
                salience: 100,
                dependency_refs: vec![],
                status,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    fn candidate(
        candidate_ref: &str,
        object_ref: &str,
        producer_lane: ProducerLane,
        contraction: u64,
    ) -> ExpansionCandidate {
        ExpansionCandidate {
            candidate_ref: candidate_ref.into(),
            object_ref: object_ref.into(),
            object_kind: if producer_lane == ProducerLane::GovernedLegal {
                KnowledgeObjectKind::PrimaryLegalSource
            } else {
                KnowledgeObjectKind::Qid
            },
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            residual_class: ResidualClass::Legal,
            producer_lane,
            source_revision_ref: None,
            expected_residual_contraction: contraction,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 1,
            admissible: true,
        }
    }

    #[test]
    fn live_step_binds_open_residual_routes_legal_tie_and_admits_one_object() {
        let mut ledger = WorldExpansionLedger::default();
        let routing = ResidualRouting {
            residual_ref: "residual:mabo:authority-source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "pnf:authority-obligation".into(),
        };
        let candidates = [
            candidate("wiki", "Q975866", ProducerLane::WikidataIdentity, 3),
            candidate(
                "oalc",
                "source:mabo:[1992]-HCA-23",
                ProducerLane::GovernedLegal,
                3,
            ),
        ];

        let receipt = execute_reviewed_expansion_step(
            &frontier(ResidualStatus::Open),
            &routing,
            &candidates,
            &mut ledger,
            mabo_world_expansion_policy(),
            ReviewDecision::Reviewed,
            DisambiguationOutcome::NewEvidentiarySource,
        )
        .unwrap();

        assert_eq!(receipt.selected_candidate_ref, "oalc");
        assert_eq!(receipt.selected_producer_lane, ProducerLane::GovernedLegal);
        assert_eq!(receipt.routing_reason_ref, "pnf:authority-obligation");
        assert_eq!(receipt.total_new_world_objects, 1);
        assert!(!receipt.target_complete);
        assert!(receipt.admission.admitted);
        assert!(!receipt.admission.creates_semantic_authority);
        assert!(!receipt.admission.claim_truth_promoted);
    }

    #[test]
    fn stronger_nonlegal_contraction_can_beat_legal_prior() {
        let mut ledger = WorldExpansionLedger::default();
        let routing = ResidualRouting {
            residual_ref: "residual:mabo:authority-source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "world:gap:identity-disambiguation".into(),
        };
        let candidates = [
            candidate(
                "oalc",
                "source:mabo:[1992]-HCA-23",
                ProducerLane::GovernedLegal,
                2,
            ),
            candidate("qid", "Q975866", ProducerLane::WikidataIdentity, 4),
        ];

        let receipt = execute_reviewed_expansion_step(
            &frontier(ResidualStatus::Open),
            &routing,
            &candidates,
            &mut ledger,
            mabo_world_expansion_policy(),
            ReviewDecision::Reviewed,
            DisambiguationOutcome::NewRelatedObject,
        )
        .unwrap();

        assert_eq!(receipt.selected_candidate_ref, "qid");
        assert_eq!(receipt.selected_producer_lane, ProducerLane::WikidataIdentity);
    }

    #[test]
    fn closed_or_unknown_residual_fails_before_selection() {
        let mut ledger = WorldExpansionLedger::default();
        let routing = ResidualRouting {
            residual_ref: "residual:mabo:authority-source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "pnf:authority-obligation".into(),
        };
        let candidates = [candidate(
            "oalc",
            "source:mabo:[1992]-HCA-23",
            ProducerLane::GovernedLegal,
            3,
        )];

        assert_eq!(
            execute_reviewed_expansion_step(
                &frontier(ResidualStatus::SatisfiedCandidate),
                &routing,
                &candidates,
                &mut ledger,
                mabo_world_expansion_policy(),
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewEvidentiarySource,
            ),
            Err(WorldExpansionStepError::ResidualNotOpen)
        );

        let missing = ResidualRouting {
            residual_ref: "residual:mabo:missing".into(),
            ..routing
        };
        assert_eq!(
            execute_reviewed_expansion_step(
                &frontier(ResidualStatus::Open),
                &missing,
                &candidates,
                &mut ledger,
                mabo_world_expansion_policy(),
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewEvidentiarySource,
            ),
            Err(WorldExpansionStepError::ResidualNotFound)
        );
    }

    #[test]
    fn fail_closed_disambiguation_does_not_advance_100_object_target() {
        let mut ledger = WorldExpansionLedger::default();
        let routing = ResidualRouting {
            residual_ref: "residual:mabo:authority-source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "pnf:authority-obligation".into(),
        };
        let candidates = [candidate(
            "oalc",
            "source:mabo:[1992]-HCA-23",
            ProducerLane::GovernedLegal,
            3,
        )];

        let receipt = execute_reviewed_expansion_step(
            &frontier(ResidualStatus::Open),
            &routing,
            &candidates,
            &mut ledger,
            mabo_world_expansion_policy(),
            ReviewDecision::Reviewed,
            DisambiguationOutcome::Ambiguous,
        )
        .unwrap();

        assert!(!receipt.admission.admitted);
        assert_eq!(receipt.total_new_world_objects, 0);
        assert!(!receipt.target_complete);
    }
}
