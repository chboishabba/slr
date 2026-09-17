use crate::frontier::ResidualStatus;
use crate::world_expansion::{
    review_admission_with_identity, select_for_proof_residual, DisambiguationOutcome,
    ExpansionCandidate, ReviewDecision,
};
use crate::world_expansion_reentry::{reenter_after_acquisition, PostAcquisitionWorldObservation};
use crate::world_expansion_session::{
    WorldExpansionCycleError, WorldExpansionCycleReceipt, WorldExpansionSession,
};
use crate::world_expansion_step::{
    ResidualRouting, WorldExpansionStepError, WorldExpansionStepReceipt,
};
use crate::world_identity::WorldIdentityResolutionReceipt;

pub fn apply_reviewed_cycle_with_identity(
    session: &mut WorldExpansionSession,
    next_frontier_ref: impl Into<String>,
    routing: &ResidualRouting,
    candidates: &[ExpansionCandidate],
    review_decision: ReviewDecision,
    disambiguation_outcome: DisambiguationOutcome,
    identity_resolution: &WorldIdentityResolutionReceipt,
    observation: &PostAcquisitionWorldObservation,
) -> Result<WorldExpansionCycleReceipt, WorldExpansionCycleError> {
    let residual = session
        .frontier
        .residuals
        .iter()
        .find(|residual| residual.residual_ref == routing.residual_ref)
        .ok_or(WorldExpansionStepError::ResidualNotFound)?;
    if residual.status != ResidualStatus::Open {
        return Err(WorldExpansionStepError::ResidualNotOpen.into());
    }

    let selected = select_for_proof_residual(
        residual,
        routing.residual_class,
        candidates,
        session.policy.minimum_expected_residual_contraction,
    )
    .ok_or(WorldExpansionStepError::NoCandidateMeetsThreshold)?;

    let mut staged_ledger = session.ledger.clone();
    let admission = review_admission_with_identity(
        &mut staged_ledger,
        selected,
        review_decision,
        disambiguation_outcome,
        identity_resolution,
    );
    let step = WorldExpansionStepReceipt {
        residual_ref: residual.residual_ref.clone(),
        residual_class: routing.residual_class,
        routing_reason_ref: routing.routing_reason_ref.clone(),
        selected_candidate_ref: selected.candidate_ref.clone(),
        selected_object_ref: selected.object_ref.clone(),
        selected_discovery_parent_ref: selected.discovery_parent_ref.clone(),
        selected_source_revision_ref: selected.source_revision_ref.clone(),
        selected_producer_lane: selected.producer_lane,
        expected_residual_contraction: selected.expected_residual_contraction,
        admission,
        total_new_world_objects: staged_ledger.total_new_world_objects,
        target_novel_objects: session.policy.target_novel_objects,
        target_complete: session.policy.complete(&staged_ledger),
        receipt_authority: "candidate_world_expansion_only",
    };

    let reentry = reenter_after_acquisition(
        &session.frontier,
        next_frontier_ref,
        &step,
        observation,
    )?;

    session.ledger = staged_ledger;
    session.frontier = reentry.next_frontier.clone();
    session.lineages.push(reentry.lineage.clone());

    Ok(WorldExpansionCycleReceipt {
        step,
        reentry,
        total_new_world_objects: session.ledger.total_new_world_objects,
        target_novel_objects: session.policy.target_novel_objects,
        target_complete: session.complete(),
    })
}
