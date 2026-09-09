//! Consumer-relative source-acquisition admission for atomic legal gaps.
//!
//! This layer sits in front of the existing proof-search scheduler.  It decides
//! only whether an unresolved source coordinate is live for the selected
//! consumer.  It never schedules around an explicit downstream atomic blocker.

use sensiblaw_proof_search_scheduler::{
    schedule, CandidateMove, CandidateMoveReceipt, ProofGap, ScheduleError, SchedulerPolicy,
};

use crate::{AtomicCaseRegistry, AtomicGate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceNeedDisposition {
    AcquireForLiveConsumer,
    DeferForCounterfactualConsumer,
    BlockedDownstream,
    OptionalEnrichment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtomicBlocker {
    pub case_context: String,
    pub atom_id: String,
    pub blocker_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerSourceDemand {
    pub consumer_ref: String,
    pub residual_ref: String,
    pub missing_proposition_ref: String,
    pub required_producer_ref: String,
    pub jurisdiction_ref: Option<String>,
    pub disposition: SourceNeedDisposition,
    pub blocker: Option<AtomicBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourcePriorityError {
    MissingRequiredBlocker,
    UnknownBlockerAtom,
    BlockerIsNotNegative,
    Scheduler(ScheduleError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsumerSourceScheduleOutcome {
    Scheduled(CandidateMoveReceipt),
    Deferred(SourceNeedDisposition),
}

fn blocker_is_registered_negative(
    registry: &AtomicCaseRegistry,
    blocker: &AtomicBlocker,
) -> Result<(), SourcePriorityError> {
    let entry = registry
        .get(&blocker.case_context, &blocker.atom_id)
        .ok_or(SourcePriorityError::UnknownBlockerAtom)?;
    if entry.gate != AtomicGate::FailsThisAtom {
        return Err(SourcePriorityError::BlockerIsNotNegative);
    }
    Ok(())
}

pub fn proof_gap_if_live(
    registry: &AtomicCaseRegistry,
    demand: &ConsumerSourceDemand,
) -> Result<Option<ProofGap>, SourcePriorityError> {
    match demand.disposition {
        SourceNeedDisposition::AcquireForLiveConsumer => Ok(Some(ProofGap {
            consumer_ref: demand.consumer_ref.clone(),
            residual_ref: demand.residual_ref.clone(),
            missing_proposition_ref: demand.missing_proposition_ref.clone(),
            required_producer_ref: demand.required_producer_ref.clone(),
            jurisdiction_ref: demand.jurisdiction_ref.clone(),
        })),
        SourceNeedDisposition::BlockedDownstream => {
            let blocker = demand
                .blocker
                .as_ref()
                .ok_or(SourcePriorityError::MissingRequiredBlocker)?;
            blocker_is_registered_negative(registry, blocker)?;
            Ok(None)
        }
        SourceNeedDisposition::DeferForCounterfactualConsumer
        | SourceNeedDisposition::OptionalEnrichment => Ok(None),
    }
}

pub fn schedule_consumer_source_demand(
    registry: &AtomicCaseRegistry,
    demand: &ConsumerSourceDemand,
    candidates: &[CandidateMove],
    policy: SchedulerPolicy,
) -> Result<ConsumerSourceScheduleOutcome, SourcePriorityError> {
    match proof_gap_if_live(registry, demand)? {
        Some(gap) => schedule(&gap, candidates, policy)
            .map(ConsumerSourceScheduleOutcome::Scheduled)
            .map_err(SourcePriorityError::Scheduler),
        None => Ok(ConsumerSourceScheduleOutcome::Deferred(demand.disposition)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_proof_search_scheduler::{
        ExecutionCostVector, ExecutionStrategy, ProofValueVector,
    };

    use crate::{cullen_gold_registry, CULLEN_CONTEXT};

    const BREACH_BLOCKER: &str =
        "atom:NSW:CLA:s5B1c:reasonable-person-would-take-proposed-precautions";

    fn s8_actual_disposition_demand() -> ConsumerSourceDemand {
        ConsumerSourceDemand {
            consumer_ref: "consumer:Cullen:actual-liability-disposition".into(),
            residual_ref: "residual:Cullen:s8-route-a-or-b".into(),
            missing_proposition_ref: "prop:Cullen:exact-s8-subroute".into(),
            required_producer_ref: "producer:NSW-vicarious-route-source".into(),
            jurisdiction_ref: Some("NSW".into()),
            disposition: SourceNeedDisposition::BlockedDownstream,
            blocker: Some(AtomicBlocker {
                case_context: CULLEN_CONTEXT.into(),
                atom_id: BREACH_BLOCKER.into(),
                blocker_reference: "registered s 5B(1)(c) failure blocks underlying tort route".into(),
            }),
        }
    }

    fn s8_counterfactual_demand() -> ConsumerSourceDemand {
        ConsumerSourceDemand {
            consumer_ref: "consumer:Cullen:counterfactual-vicarious-route".into(),
            residual_ref: "residual:Cullen:s8-route-a-or-b".into(),
            missing_proposition_ref: "prop:Cullen:exact-s8-subroute".into(),
            required_producer_ref: "producer:NSW-vicarious-route-source".into(),
            jurisdiction_ref: Some("NSW".into()),
            disposition: SourceNeedDisposition::AcquireForLiveConsumer,
            blocker: None,
        }
    }

    fn persisted_candidate() -> CandidateMove {
        CandidateMove {
            move_ref: "move:Cullen:persisted-vicarious-act".into(),
            strategy: ExecutionStrategy::PersistedAuthorityReceipt,
            source_ref: Some("source:NSW:Law-Reform-Vicarious-Liability-Act-1983".into()),
            provider_operation_ref: "persisted-authority-read".into(),
            cost: ExecutionCostVector {
                local_bytes_read_cost: 1,
                semantic_assessment_cost: 1,
                ..ExecutionCostVector::default()
            },
            value: ProofValueVector {
                expected_proof_reduction: 2,
                authority_fitness: 3,
                discriminative_value: 2,
                ..ProofValueVector::default()
            },
            admissible: true,
            calibration_ref: "fixture:Cullen:vicarious-route".into(),
        }
    }

    #[test]
    fn same_unresolved_coordinate_is_blocked_for_actual_disposition() {
        let registry = cullen_gold_registry();
        assert_eq!(
            proof_gap_if_live(&registry, &s8_actual_disposition_demand()).unwrap(),
            None
        );
    }

    #[test]
    fn same_coordinate_becomes_live_for_explicit_counterfactual_consumer() {
        let registry = cullen_gold_registry();
        let gap = proof_gap_if_live(&registry, &s8_counterfactual_demand())
            .unwrap()
            .expect("counterfactual consumer should make route selection live");
        assert_eq!(gap.residual_ref, "residual:Cullen:s8-route-a-or-b");
    }

    #[test]
    fn blocked_coordinate_never_reaches_scheduler() {
        let registry = cullen_gold_registry();
        let outcome = schedule_consumer_source_demand(
            &registry,
            &s8_actual_disposition_demand(),
            &[persisted_candidate()],
            SchedulerPolicy::default(),
        )
        .unwrap();
        assert_eq!(
            outcome,
            ConsumerSourceScheduleOutcome::Deferred(SourceNeedDisposition::BlockedDownstream)
        );
    }

    #[test]
    fn live_counterfactual_coordinate_reuses_existing_scheduler() {
        let registry = cullen_gold_registry();
        let outcome = schedule_consumer_source_demand(
            &registry,
            &s8_counterfactual_demand(),
            &[persisted_candidate()],
            SchedulerPolicy::default(),
        )
        .unwrap();
        match outcome {
            ConsumerSourceScheduleOutcome::Scheduled(receipt) => {
                assert_eq!(receipt.residual_ref, "residual:Cullen:s8-route-a-or-b");
                assert_eq!(
                    receipt.selected_strategy,
                    ExecutionStrategy::PersistedAuthorityReceipt
                );
            }
            ConsumerSourceScheduleOutcome::Deferred(_) => {
                panic!("counterfactual route coordinate should have been scheduled")
            }
        }
    }
}
