//! Post-acquisition PNF/world re-entry for residual-driven expansion.
//!
//! Predicted contraction belongs to candidate selection. This module consumes
//! an explicit post-acquisition PNF/world observation, compiles it into the
//! existing `ResidualAssessment` transition surface, appends only explicitly
//! diagnosed new open residuals, and emits durable-lineage coordinates. It does
//! not parse sources, infer new residuals, grant authority, or pay proof claims.

use std::collections::BTreeSet;

use crate::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use crate::transition::{
    apply_assessments, classify_frontier, FrontierTransitionError, FrontierTransitionReceipt,
    ResidualAssessment, ResidualAssessmentKind,
};
use crate::world_expansion::ProducerLane;
use crate::world_expansion_step::WorldExpansionStepReceipt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostAcquisitionWorldObservation {
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub triggering_residual_ref: String,
    pub assessment_kind: ResidualAssessmentKind,
    pub observed_residual_contraction: u64,
    pub newly_exposed_residuals: Vec<ProofResidual>,
    pub pnf_world_disambiguation_ref: String,
    pub observation_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryLineageReceipt {
    pub object_ref: String,
    pub discovery_parent_ref: String,
    pub triggering_residual_ref: String,
    pub selected_candidate_ref: String,
    pub producer_lane: ProducerLane,
    pub source_revision_ref: String,
    pub pnf_world_disambiguation_ref: String,
    pub expected_residual_contraction: u64,
    pub observed_residual_contraction: u64,
    pub new_residual_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldReentryReceipt {
    pub expected_residual_contraction: u64,
    pub observed_residual_contraction: u64,
    pub new_residual_refs: Vec<String>,
    pub next_frontier: ProofFrontier,
    pub transition: FrontierTransitionReceipt,
    pub lineage: DiscoveryLineageReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorldReentryError {
    AdmissionRequired,
    ResidualMismatch,
    ObservationMayNotClaimAuthority,
    EmptyObservationCoordinate(&'static str),
    SourceRevisionMismatch,
    DuplicateResidual(String),
    DuplicateNewResidual(String),
    NewResidualMustBeOpen(String),
    FrontierTransition(FrontierTransitionError),
}

impl From<FrontierTransitionError> for WorldReentryError {
    fn from(value: FrontierTransitionError) -> Self {
        Self::FrontierTransition(value)
    }
}

fn validate_observation(
    frontier: &ProofFrontier,
    step: &WorldExpansionStepReceipt,
    observation: &PostAcquisitionWorldObservation,
) -> Result<(), WorldReentryError> {
    if !step.admission.admitted {
        return Err(WorldReentryError::AdmissionRequired);
    }
    if observation.triggering_residual_ref != step.residual_ref {
        return Err(WorldReentryError::ResidualMismatch);
    }
    if observation.observation_authority != "experimental_candidate_only" {
        return Err(WorldReentryError::ObservationMayNotClaimAuthority);
    }
    for (name, value) in [
        ("observation_ref", observation.observation_ref.as_str()),
        ("source_revision_ref", observation.source_revision_ref.as_str()),
        (
            "pnf_world_disambiguation_ref",
            observation.pnf_world_disambiguation_ref.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(WorldReentryError::EmptyObservationCoordinate(name));
        }
    }
    if step
        .selected_source_revision_ref
        .as_deref()
        .is_some_and(|revision| revision != observation.source_revision_ref)
    {
        return Err(WorldReentryError::SourceRevisionMismatch);
    }

    let existing: BTreeSet<&str> = frontier
        .residuals
        .iter()
        .map(|residual| residual.residual_ref.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    for residual in &observation.newly_exposed_residuals {
        if residual.status != ResidualStatus::Open {
            return Err(WorldReentryError::NewResidualMustBeOpen(
                residual.residual_ref.clone(),
            ));
        }
        if existing.contains(residual.residual_ref.as_str()) {
            return Err(WorldReentryError::DuplicateResidual(
                residual.residual_ref.clone(),
            ));
        }
        if !seen.insert(residual.residual_ref.as_str()) {
            return Err(WorldReentryError::DuplicateNewResidual(
                residual.residual_ref.clone(),
            ));
        }
    }
    Ok(())
}

/// Re-enter one admitted acquired object through an explicit PNF/world
/// observation. Actual observed contraction, not predicted candidate score,
/// drives the canonical frontier transition. Any new residuals must already be
/// explicitly diagnosed and typed by the caller; this function never invents
/// residuals from source strings or adjacency.
pub fn reenter_after_acquisition(
    frontier: &ProofFrontier,
    next_frontier_ref: impl Into<String>,
    step: &WorldExpansionStepReceipt,
    observation: &PostAcquisitionWorldObservation,
) -> Result<WorldReentryReceipt, WorldReentryError> {
    validate_observation(frontier, step, observation)?;

    let assessment = ResidualAssessment {
        residual_ref: observation.triggering_residual_ref.clone(),
        kind: observation.assessment_kind,
        observed_proof_reduction: observation.observed_residual_contraction,
        assessment_ref: observation.observation_ref.clone(),
        assessment_authority: "experimental_candidate_only",
    };
    let (mut next_frontier, mut transition) =
        apply_assessments(frontier, next_frontier_ref, &[assessment])?;

    let mut new_residual_refs = Vec::with_capacity(observation.newly_exposed_residuals.len());
    for residual in &observation.newly_exposed_residuals {
        new_residual_refs.push(residual.residual_ref.clone());
        next_frontier.residuals.push(residual.clone());
        transition.changed_residual_refs.push(residual.residual_ref.clone());
    }
    new_residual_refs.sort();
    transition.changed_residual_refs.sort();
    transition.changed_residual_refs.dedup();
    transition.termination = classify_frontier(&next_frontier);

    let lineage = DiscoveryLineageReceipt {
        object_ref: step.selected_object_ref.clone(),
        discovery_parent_ref: step.selected_discovery_parent_ref.clone(),
        triggering_residual_ref: step.residual_ref.clone(),
        selected_candidate_ref: step.selected_candidate_ref.clone(),
        producer_lane: step.selected_producer_lane,
        source_revision_ref: observation.source_revision_ref.clone(),
        pnf_world_disambiguation_ref: observation.pnf_world_disambiguation_ref.clone(),
        expected_residual_contraction: step.expected_residual_contraction,
        observed_residual_contraction: observation.observed_residual_contraction,
        new_residual_refs: new_residual_refs.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "candidate_world_expansion_only",
    };

    Ok(WorldReentryReceipt {
        expected_residual_contraction: step.expected_residual_contraction,
        observed_residual_contraction: observation.observed_residual_contraction,
        new_residual_refs,
        next_frontier,
        transition,
        lineage,
    })
}

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
            selected_discovery_parent_ref: "Q1501525".into(),
            selected_source_revision_ref: Some("oalc:[1992]-HCA-23:sha256:abc".into()),
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
        let duplicate = residual(
            "residual:mabo:authority-source",
            "mabo:proposition:duplicate",
        );
        let observation = PostAcquisitionWorldObservation {
            observation_ref: "world-observation:mabo:3".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
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
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
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

        assert_eq!(receipt.lineage.object_ref, "case:[1992]-HCA-23");
        assert_eq!(receipt.lineage.discovery_parent_ref, "Q1501525");
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
