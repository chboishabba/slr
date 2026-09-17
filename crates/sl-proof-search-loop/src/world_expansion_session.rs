//! Atomic in-memory orchestration for one reviewed residual-expansion cycle.
//!
//! Admission is staged on a cloned ledger. The session commits ledger, frontier,
//! and lineage together only after post-acquisition PNF/world re-entry succeeds.

use crate::frontier::ProofFrontier;
use crate::world_expansion::{
    DisambiguationOutcome, ExpansionCandidate, ReviewDecision, WorldExpansionLedger,
    WorldExpansionPolicy,
};
use crate::world_expansion_reentry::{
    reenter_after_acquisition, DiscoveryLineageReceipt, PostAcquisitionWorldObservation,
    WorldReentryError, WorldReentryReceipt,
};
use crate::world_expansion_step::{
    execute_reviewed_expansion_step, ResidualRouting, WorldExpansionStepError,
    WorldExpansionStepReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldExpansionSession {
    pub frontier: ProofFrontier,
    pub ledger: WorldExpansionLedger,
    pub policy: WorldExpansionPolicy,
    pub lineages: Vec<DiscoveryLineageReceipt>,
}

impl WorldExpansionSession {
    #[must_use]
    pub fn new(frontier: ProofFrontier, policy: WorldExpansionPolicy) -> Self {
        Self {
            frontier,
            ledger: WorldExpansionLedger::default(),
            policy,
            lineages: Vec::new(),
        }
    }

    #[must_use]
    pub fn complete(&self) -> bool {
        self.policy.complete(&self.ledger)
    }

    pub fn apply_reviewed_cycle(
        &mut self,
        next_frontier_ref: impl Into<String>,
        routing: &ResidualRouting,
        candidates: &[ExpansionCandidate],
        review_decision: ReviewDecision,
        disambiguation_outcome: DisambiguationOutcome,
        observation: &PostAcquisitionWorldObservation,
    ) -> Result<WorldExpansionCycleReceipt, WorldExpansionCycleError> {
        let mut staged_ledger = self.ledger.clone();
        let step = execute_reviewed_expansion_step(
            &self.frontier,
            routing,
            candidates,
            &mut staged_ledger,
            self.policy,
            review_decision,
            disambiguation_outcome,
        )?;
        let reentry = reenter_after_acquisition(
            &self.frontier,
            next_frontier_ref,
            &step,
            observation,
        )?;

        self.ledger = staged_ledger;
        self.frontier = reentry.next_frontier.clone();
        self.lineages.push(reentry.lineage.clone());

        Ok(WorldExpansionCycleReceipt {
            step,
            reentry,
            total_new_world_objects: self.ledger.total_new_world_objects,
            target_novel_objects: self.policy.target_novel_objects,
            target_complete: self.complete(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldExpansionCycleReceipt {
    pub step: WorldExpansionStepReceipt,
    pub reentry: WorldReentryReceipt,
    pub total_new_world_objects: usize,
    pub target_novel_objects: usize,
    pub target_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldExpansionCycleError {
    Step(WorldExpansionStepError),
    Reentry(WorldReentryError),
}

impl From<WorldExpansionStepError> for WorldExpansionCycleError {
    fn from(value: WorldExpansionStepError) -> Self {
        Self::Step(value)
    }
}

impl From<WorldReentryError> for WorldExpansionCycleError {
    fn from(value: WorldReentryError) -> Self {
        Self::Reentry(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
    use crate::transition::ResidualAssessmentKind;
    use crate::world_expansion::{
        DisambiguationOutcome, ExpansionCandidate, KnowledgeObjectKind, ProducerLane,
        ResidualClass, ReviewDecision, mabo_world_expansion_policy,
    };
    use crate::world_expansion_reentry::PostAcquisitionWorldObservation;
    use crate::world_expansion_step::ResidualRouting;

    fn frontier() -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:mabo-100-object-world".into(),
            frontier_ref: "frontier:mabo:0".into(),
            residuals: vec![ProofResidual {
                residual_ref: "residual:mabo:source".into(),
                proposition_ref: "mabo:proposition:source".into(),
                producer_class_ref: "producer:world-expansion".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("primary-case".into()),
                salience: 100,
                dependency_refs: vec![],
                status: ResidualStatus::Open,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    fn candidate() -> ExpansionCandidate {
        ExpansionCandidate {
            candidate_ref: "oalc:case:[1992]-HCA-23".into(),
            object_ref: "case:[1992]-HCA-23".into(),
            object_kind: KnowledgeObjectKind::PrimaryLegalSource,
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: "residual:mabo:source".into(),
            residual_class: ResidualClass::Legal,
            producer_lane: ProducerLane::GovernedLegal,
            source_revision_ref: Some("oalc:[1992]-HCA-23:sha256:abc".into()),
            expected_residual_contraction: 4,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 0,
            admissible: true,
        }
    }

    fn observation(residual_ref: &str) -> PostAcquisitionWorldObservation {
        PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:1".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
            triggering_residual_ref: residual_ref.into(),
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 3,
            newly_exposed_residuals: vec![ProofResidual {
                residual_ref: "residual:mabo:precedent".into(),
                proposition_ref: "mabo:proposition:precedent".into(),
                producer_class_ref: "producer:world-expansion".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: Some("primary-case".into()),
                salience: 90,
                dependency_refs: vec![],
                status: ResidualStatus::Open,
            }],
            pnf_world_disambiguation_ref: "pnf-world:mabo:1".into(),
            observation_authority: "experimental_candidate_only",
        }
    }

    #[test]
    fn successful_cycle_commits_ledger_frontier_and_lineage_together() {
        let mut session = WorldExpansionSession::new(frontier(), mabo_world_expansion_policy());
        let routing = ResidualRouting {
            residual_ref: "residual:mabo:source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "pnf:source-authority".into(),
        };
        let candidates = [candidate()];
        let receipt = session
            .apply_reviewed_cycle(
                "frontier:mabo:1",
                &routing,
                &candidates,
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewEvidentiarySource,
                &observation("residual:mabo:source"),
            )
            .unwrap();

        assert_eq!(session.ledger.total_new_world_objects, 1);
        assert_eq!(session.frontier.frontier_ref, "frontier:mabo:1");
        assert_eq!(session.lineages.len(), 1);
        assert_eq!(receipt.reentry.new_residual_refs, vec!["residual:mabo:precedent"]);
        assert_eq!(session.frontier.open_residuals().count(), 1);
    }

    #[test]
    fn failed_reentry_rolls_back_staged_admission() {
        let initial = frontier();
        let mut session = WorldExpansionSession::new(initial.clone(), mabo_world_expansion_policy());
        let routing = ResidualRouting {
            residual_ref: "residual:mabo:source".into(),
            residual_class: ResidualClass::Legal,
            routing_reason_ref: "pnf:source-authority".into(),
        };
        let candidates = [candidate()];
        let error = session
            .apply_reviewed_cycle(
                "frontier:mabo:1",
                &routing,
                &candidates,
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewEvidentiarySource,
                &observation("residual:wrong"),
            )
            .unwrap_err();

        assert!(matches!(error, WorldExpansionCycleError::Reentry(_)));
        assert_eq!(session.ledger.total_new_world_objects, 0);
        assert_eq!(session.frontier, initial);
        assert!(session.lineages.is_empty());
    }
}
