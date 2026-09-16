#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
    use crate::world_expansion::{
        DisambiguationOutcome, ExpansionCandidate, KnowledgeObjectKind, ProducerLane,
        ResidualClass, ReviewDecision, WorldExpansionLedger, mabo_world_expansion_policy,
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
