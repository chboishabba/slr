use std::collections::BTreeSet;

use crate::frontier::{ProofFrontier, ResidualStatus};
use crate::transition::{
    apply_assessments, classify_frontier, FrontierTransitionError, FrontierTransitionReceipt,
    ResidualAssessment, ResidualAssessmentKind,
};
use crate::world_expansion_reentry::PostAcquisitionWorldObservation;
use crate::world_expansion_session::WorldExpansionSession;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownIdentityPaymentReceipt {
    pub identity_class_ref: String,
    pub triggering_residual_ref: String,
    pub observed_residual_contraction: u64,
    pub new_residual_refs: Vec<String>,
    pub next_frontier: ProofFrontier,
    pub transition: FrontierTransitionReceipt,
    pub non_novel_payment: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownIdentityPaymentError {
    EmptyIdentityClass,
    ResidualNotFound,
    ResidualNotOpen,
    ObservationMayNotClaimAuthority,
    ObservationMustSatisfyCandidateResidual,
    ObservedContractionRequired,
    DuplicateResidual(String),
    DuplicateNewResidual(String),
    NewResidualMustBeOpen(String),
    FrontierTransition(FrontierTransitionError),
}

impl From<FrontierTransitionError> for KnownIdentityPaymentError {
    fn from(value: FrontierTransitionError) -> Self {
        Self::FrontierTransition(value)
    }
}

pub fn reenter_known_identity_payment(
    session: &WorldExpansionSession,
    next_frontier_ref: impl Into<String>,
    identity_class_ref: &str,
    observation: &PostAcquisitionWorldObservation,
) -> Result<KnownIdentityPaymentReceipt, KnownIdentityPaymentError> {
    if identity_class_ref.trim().is_empty() {
        return Err(KnownIdentityPaymentError::EmptyIdentityClass);
    }
    let residual = session
        .frontier
        .residuals
        .iter()
        .find(|residual| residual.residual_ref == observation.triggering_residual_ref)
        .ok_or(KnownIdentityPaymentError::ResidualNotFound)?;
    if residual.status != ResidualStatus::Open {
        return Err(KnownIdentityPaymentError::ResidualNotOpen);
    }
    if observation.observation_authority != "experimental_candidate_only" {
        return Err(KnownIdentityPaymentError::ObservationMayNotClaimAuthority);
    }
    if observation.assessment_kind != ResidualAssessmentKind::SatisfiedCandidate {
        return Err(KnownIdentityPaymentError::ObservationMustSatisfyCandidateResidual);
    }
    if observation.observed_residual_contraction == 0 {
        return Err(KnownIdentityPaymentError::ObservedContractionRequired);
    }

    let existing: BTreeSet<&str> = session
        .frontier
        .residuals
        .iter()
        .map(|residual| residual.residual_ref.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    for new_residual in &observation.newly_exposed_residuals {
        if new_residual.status != ResidualStatus::Open {
            return Err(KnownIdentityPaymentError::NewResidualMustBeOpen(
                new_residual.residual_ref.clone(),
            ));
        }
        if existing.contains(new_residual.residual_ref.as_str()) {
            return Err(KnownIdentityPaymentError::DuplicateResidual(
                new_residual.residual_ref.clone(),
            ));
        }
        if !seen.insert(new_residual.residual_ref.as_str()) {
            return Err(KnownIdentityPaymentError::DuplicateNewResidual(
                new_residual.residual_ref.clone(),
            ));
        }
    }

    let assessment = ResidualAssessment {
        residual_ref: observation.triggering_residual_ref.clone(),
        kind: observation.assessment_kind,
        observed_proof_reduction: observation.observed_residual_contraction,
        assessment_ref: observation.observation_ref.clone(),
        assessment_authority: "experimental_candidate_only",
    };
    let (mut next_frontier, mut transition) =
        apply_assessments(&session.frontier, next_frontier_ref, &[assessment])?;

    let mut new_residual_refs = Vec::with_capacity(observation.newly_exposed_residuals.len());
    for new_residual in &observation.newly_exposed_residuals {
        new_residual_refs.push(new_residual.residual_ref.clone());
        next_frontier.residuals.push(new_residual.clone());
        transition
            .changed_residual_refs
            .push(new_residual.residual_ref.clone());
    }
    new_residual_refs.sort();
    transition.changed_residual_refs.sort();
    transition.changed_residual_refs.dedup();
    transition.termination = classify_frontier(&next_frontier);

    Ok(KnownIdentityPaymentReceipt {
        identity_class_ref: identity_class_ref.to_owned(),
        triggering_residual_ref: observation.triggering_residual_ref.clone(),
        observed_residual_contraction: observation.observed_residual_contraction,
        new_residual_refs,
        next_frontier,
        transition,
        non_novel_payment: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
