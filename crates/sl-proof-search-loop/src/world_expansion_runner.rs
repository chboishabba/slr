//! Bounded recurrent orchestration for the P7d Mabo world-expansion target.
//!
//! The runner owns no acquisition semantics. A cycle source must prepare one
//! already-governed residual-bound acquisition/review/re-entry package from the
//! current session. Each package is executed against a cloned session and is
//! committed only after the lineage sink accepts the cycle receipt.

use crate::world_expansion::{DisambiguationOutcome, ExpansionCandidate, ReviewDecision};
use crate::world_expansion_identity_session::apply_reviewed_cycle_with_identity;
use crate::world_expansion_reentry::PostAcquisitionWorldObservation;
use crate::world_expansion_session::{
    WorldExpansionCycleError, WorldExpansionCycleReceipt, WorldExpansionSession,
};
use crate::world_expansion_step::ResidualRouting;
use crate::world_identity::WorldIdentityResolutionReceipt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldExpansionRunnerConfig {
    pub max_cycles: usize,
}

impl Default for WorldExpansionRunnerConfig {
    fn default() -> Self {
        Self { max_cycles: 1000 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedWorldExpansionCycle {
    pub next_frontier_ref: String,
    pub routing: ResidualRouting,
    pub candidates: Vec<ExpansionCandidate>,
    pub review_decision: ReviewDecision,
    pub disambiguation_outcome: DisambiguationOutcome,
    pub identity_resolution: WorldIdentityResolutionReceipt,
    pub observation: PostAcquisitionWorldObservation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrentRunBlocker {
    pub blocker_ref: String,
}

impl RecurrentRunBlocker {
    #[must_use]
    pub fn new(blocker_ref: impl Into<String>) -> Self {
        Self {
            blocker_ref: blocker_ref.into(),
        }
    }
}

pub trait WorldExpansionCycleSource {
    fn prepare_next_cycle(
        &mut self,
        session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker>;
}

pub trait WorldExpansionCycleSink {
    fn persist_cycle(
        &mut self,
        receipt: &WorldExpansionCycleReceipt,
    ) -> Result<(), RecurrentRunBlocker>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrentRunStopReason {
    TargetComplete,
    FrontierExhausted,
    CycleBudgetExhausted,
    Blocked(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrentWorldExpansionReceipt {
    pub cycles_committed: usize,
    pub initial_novel_identity_classes: usize,
    pub final_novel_identity_classes: usize,
    pub target_novel_identity_classes: usize,
    pub candidates_seen: usize,
    pub candidates_rejected: usize,
    pub duplicates_seen: usize,
    pub identity_ambiguous: usize,
    pub reviewed_objects: usize,
    pub new_qids_admitted: usize,
    pub new_articles_admitted: usize,
    pub new_primary_legal_sources_admitted: usize,
    pub new_other_world_objects_admitted: usize,
    pub final_frontier_ref: String,
    pub remaining_open_residual_refs: Vec<String>,
    pub lineage_receipts: usize,
    pub stop_reason: RecurrentRunStopReason,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrentWorldExpansionError {
    Cycle(WorldExpansionCycleError),
}

impl From<WorldExpansionCycleError> for RecurrentWorldExpansionError {
    fn from(value: WorldExpansionCycleError) -> Self {
        Self::Cycle(value)
    }
}

fn receipt(
    session: &WorldExpansionSession,
    initial_novel_identity_classes: usize,
    cycles_committed: usize,
    stop_reason: RecurrentRunStopReason,
) -> RecurrentWorldExpansionReceipt {
    let ledger = &session.ledger;
    RecurrentWorldExpansionReceipt {
        cycles_committed,
        initial_novel_identity_classes,
        final_novel_identity_classes: ledger.total_new_world_objects,
        target_novel_identity_classes: session.policy.target_novel_objects,
        candidates_seen: ledger.candidates_seen,
        candidates_rejected: ledger.candidates_rejected,
        duplicates_seen: ledger.duplicates_seen,
        identity_ambiguous: ledger.identity_ambiguous,
        reviewed_objects: ledger.reviewed_objects,
        new_qids_admitted: ledger.new_qids_admitted,
        new_articles_admitted: ledger.new_articles_admitted,
        new_primary_legal_sources_admitted: ledger.new_primary_legal_sources_admitted,
        new_other_world_objects_admitted: ledger.new_other_world_objects_admitted,
        final_frontier_ref: session.frontier.frontier_ref.clone(),
        remaining_open_residual_refs: session
            .frontier
            .open_residuals()
            .map(|residual| residual.residual_ref.clone())
            .collect(),
        lineage_receipts: session.lineages.len(),
        stop_reason,
        receipt_authority: "candidate_world_expansion_run_only",
    }
}

pub fn run_recurrent_world_expansion<S, K>(
    session: &mut WorldExpansionSession,
    source: &mut S,
    sink: &mut K,
    config: WorldExpansionRunnerConfig,
) -> Result<RecurrentWorldExpansionReceipt, RecurrentWorldExpansionError>
where
    S: WorldExpansionCycleSource,
    K: WorldExpansionCycleSink,
{
    let initial_novel_identity_classes = session.ledger.total_new_world_objects;
    let mut cycles_committed = 0usize;

    loop {
        if session.complete() {
            return Ok(receipt(
                session,
                initial_novel_identity_classes,
                cycles_committed,
                RecurrentRunStopReason::TargetComplete,
            ));
        }
        if session.frontier.open_residuals().next().is_none() {
            return Ok(receipt(
                session,
                initial_novel_identity_classes,
                cycles_committed,
                RecurrentRunStopReason::FrontierExhausted,
            ));
        }
        if cycles_committed >= config.max_cycles {
            return Ok(receipt(
                session,
                initial_novel_identity_classes,
                cycles_committed,
                RecurrentRunStopReason::CycleBudgetExhausted,
            ));
        }

        let prepared = match source.prepare_next_cycle(session) {
            Ok(prepared) => prepared,
            Err(blocker) => {
                return Ok(receipt(
                    session,
                    initial_novel_identity_classes,
                    cycles_committed,
                    RecurrentRunStopReason::Blocked(blocker.blocker_ref),
                ));
            }
        };

        let mut staged = session.clone();
        let cycle = apply_reviewed_cycle_with_identity(
            &mut staged,
            prepared.next_frontier_ref,
            &prepared.routing,
            &prepared.candidates,
            prepared.review_decision,
            prepared.disambiguation_outcome,
            &prepared.identity_resolution,
            &prepared.observation,
        )?;

        if let Err(blocker) = sink.persist_cycle(&cycle) {
            return Ok(receipt(
                session,
                initial_novel_identity_classes,
                cycles_committed,
                RecurrentRunStopReason::Blocked(blocker.blocker_ref),
            ));
        }

        *session = staged;
        cycles_committed += 1;
    }
}
