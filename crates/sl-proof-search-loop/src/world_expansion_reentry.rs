#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
    use crate::transition::{ResidualAssessmentKind, ResearchTermination};
    use crate::world_expansion::{
        DisambiguationOutcome, ProducerLane, ResidualClass, ReviewDecision,
    };
    use crate::world_expansion_step::WorldExpansionStepReceipt;

    fn residual(reference: &str, proposition: &str) -> ProofResidual {
        ProofResidual {
            residual_ref: reference.into(),
            proposition_ref: proposition.into(),
            producer_class_ref: "producer:world-expansion".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: 10,
            dependency_refs: vec![],
            status: ResidualStatus::Open,
        }
    }

    fn frontier() -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:mabo-100-object-world".into(),
            frontier_ref: "frontier:mabo:0".into(),
            residuals: vec![residual(
                "residual:mabo:authority-source",
                "mabo:proposition:radical-title-native-title",
            )],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    fn step_receipt() -> WorldExpansionStepReceipt {
        WorldExpansionStepReceipt {
            residual_ref: "residual:mabo:authority-source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "pnf:authority-obligation".into(),
            selected_candidate_ref: "oalc:case:[1992]-HCA-23".into(),
            selected_object_ref: "case:[1992]-HCA-23".into(),
            selected_producer_lane: ProducerLane::GovernedLegal,
            expected_residual_contraction: 4,
            admission: crate::world_expansion::AdmissionReceipt {
                candidate_ref: "oalc:case:[1992]-HCA-23".into(),
                object_ref: "case:[1992]-HCA-23".into(),
                triggering_residual_ref: "residual:mabo:authority-source".into(),
                producer_lane: ProducerLane::GovernedLegal,
                disambiguation_outcome: DisambiguationOutcome::NewEvidentiarySource,
                review_decision: ReviewDecision::Reviewed,
                admitted: true,
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            total_new_world_objects: 1,
            target_novel_objects: 100,
            target_complete: false,
            receipt_authority: "candidate_world_expansion_only",
        }
    }

    #[test]
    fn observed_world_delta_drives_frontier_transition_not_predicted_score() {
        let observation = PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:1".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            assessment_kind: ResidualAssessmentKind::Narrowed,
            observed_residual_contraction: 1,
            newly_exposed_residuals: vec![],
            pnf_world_disambiguation_ref: "pnf-world:mabo:1".into(),
            observation_authority: "experimental_candidate_only",
        };

        let receipt = reenter_after_acquisition(
            &frontier(),
            "frontier:mabo:1",
            &step_receipt(),
            &observation,
        )
        .unwrap();

        assert_eq!(receipt.expected_residual_contraction, 4);
        assert_eq!(receipt.observed_residual_contraction, 1);
        assert_eq!(receipt.next_frontier.frontier_ref, "frontier:mabo:1");
        assert_eq!(receipt.transition.changed_residual_refs, vec!["residual:mabo:authority-source"]);
        assert_eq!(receipt.transition.termination, ResearchTermination::Continue);
    }

    #[test]
    fn explicit_new_pnf_world_residuals_are_appended_open_and_recur() {
        let new_residual = residual(
            "residual:mabo:precedent-treatment",
            "mabo:proposition:precedent-treatment",
        );
        let observation = PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:2".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 4,
            newly_exposed_residuals: vec![new_residual.clone()],
            pnf_world_disambiguation_ref: "pnf-world:mabo:2".into(),
            observation_authority: "experimental_candidate_only",
        };

        let receipt = reenter_after_acquisition(
            &frontier(),
            "frontier:mabo:2",
            &step_receipt(),
            &observation,
        )
        .unwrap();

        assert_eq!(receipt.next_frontier.residuals.len(), 2);
        assert_eq!(receipt.next_frontier.residuals[0].status, ResidualStatus::SatisfiedCandidate);
        assert_eq!(receipt.next_frontier.residuals[1], new_residual);
        assert_eq!(receipt.transition.termination, ResearchTermination::Continue);
        assert_eq!(receipt.new_residual_refs, vec!["residual:mabo:precedent-treatment"]);
    }

    #[test]
    fn duplicate_or_closed_new_residuals_fail_closed() {
        let mut duplicate = residual(
            "residual:mabo:authority-source",
            "mabo:proposition:duplicate",
        );
        duplicate.status = ResidualStatus::Open;
        let observation = PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:3".into(),
            source_revision_ref: "oalc:rev".into(),
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            assessment_kind: ResidualAssessmentKind::Narrowed,
            observed_residual_contraction: 1,
            newly_exposed_residuals: vec![duplicate],
            pnf_world_disambiguation_ref: "pnf-world:mabo:3".into(),
            observation_authority: "experimental_candidate_only",
        };
        assert_eq!(
            reenter_after_acquisition(&frontier(), "frontier:mabo:3", &step_receipt(), &observation),
            Err(WorldReentryError::DuplicateResidual("residual:mabo:authority-source".into()))
        );

        let mut closed = residual("residual:mabo:new", "mabo:proposition:new");
        closed.status = ResidualStatus::SatisfiedCandidate;
        let observation = PostAcquisitionWorldObservation {
            newly_exposed_residuals: vec![closed],
            ..observation
        };
        assert_eq!(
            reenter_after_acquisition(&frontier(), "frontier:mabo:4", &step_receipt(), &observation),
            Err(WorldReentryError::NewResidualMustBeOpen("residual:mabo:new".into()))
        );
    }

    #[test]
    fn observation_must_match_admitted_step_and_remain_candidate_only() {
        let observation = PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:4".into(),
            source_revision_ref: "oalc:rev".into(),
            triggering_residual_ref: "residual:other".into(),
            assessment_kind: ResidualAssessmentKind::Narrowed,
            observed_residual_contraction: 1,
            newly_exposed_residuals: vec![],
            pnf_world_disambiguation_ref: "pnf-world:mabo:4".into(),
            observation_authority: "experimental_candidate_only",
        };
        assert_eq!(
            reenter_after_acquisition(&frontier(), "frontier:mabo:4", &step_receipt(), &observation),
            Err(WorldReentryError::ResidualMismatch)
        );

        let observation = PostAcquisitionWorldObservation {
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            observation_authority: "semantic_authority",
            ..observation
        };
        assert_eq!(
            reenter_after_acquisition(&frontier(), "frontier:mabo:4", &step_receipt(), &observation),
            Err(WorldReentryError::ObservationMayNotClaimAuthority)
        );
    }

    #[test]
    fn lineage_retains_parent_residual_producer_revision_and_world_delta() {
        let observation = PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:5".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
            triggering_residual_ref: "residual:mabo:authority-source".into(),
            assessment_kind: ResidualAssessmentKind::Narrowed,
            observed_residual_contraction: 2,
            newly_exposed_residuals: vec![residual(
                "residual:mabo:case-follow",
                "mabo:proposition:case-follow",
            )],
            pnf_world_disambiguation_ref: "pnf-world:mabo:5".into(),
            observation_authority: "experimental_candidate_only",
        };
        let receipt = reenter_after_acquisition(
            &frontier(),
            "frontier:mabo:5",
            &step_receipt(),
            &observation,
        )
        .unwrap();

        assert_eq!(receipt.lineage.discovery_parent_ref, "case:[1992]-HCA-23");
        assert_eq!(receipt.lineage.triggering_residual_ref, "residual:mabo:authority-source");
        assert_eq!(receipt.lineage.producer_lane, ProducerLane::GovernedLegal);
        assert_eq!(receipt.lineage.source_revision_ref, "oalc:[1992]-HCA-23:sha256:abc");
        assert_eq!(receipt.lineage.observed_residual_contraction, 2);
        assert_eq!(receipt.lineage.new_residual_refs, vec!["residual:mabo:case-follow"]);
        assert!(receipt.lineage.candidate_only);
        assert!(!receipt.lineage.creates_semantic_authority);
        assert!(!receipt.lineage.claim_truth_promoted);
    }
}
