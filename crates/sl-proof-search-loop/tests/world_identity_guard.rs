use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{
    mabo_world_expansion_policy, DisambiguationOutcome, ExpansionCandidate, KnowledgeObjectKind,
    ProducerLane, ResidualClass, ReviewDecision,
};
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    run_recurrent_world_expansion, PreparedWorldExpansionCycle, RecurrentRunBlocker,
    RecurrentRunBlockerKind, RecurrentRunStopReason, WorldExpansionCycleSink,
    WorldExpansionCycleSource, WorldExpansionRunnerConfig,
};
use sensiblaw_proof_search_loop::world_expansion_session::{
    WorldExpansionCycleReceipt, WorldExpansionSession,
};
use sensiblaw_proof_search_loop::world_expansion_step::ResidualRouting;
use sensiblaw_proof_search_loop::world_identity::{
    WorldIdentityResolutionKind, WorldIdentityResolutionReceipt, WorldObjectIdentity,
};
use sensiblaw_proof_search_loop::world_identity_guard::IdentityCoherentCycleSource;

fn residual(id: &str) -> ProofResidual {
    ProofResidual {
        residual_ref: id.into(),
        proposition_ref: format!("proposition:{id}"),
        producer_class_ref: "producer:world-expansion".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status: ResidualStatus::Open,
    }
}

fn prepared(
    index: usize,
    object_ref: &str,
    identity_class_ref: &str,
    next_residual: Option<&str>,
) -> PreparedWorldExpansionCycle {
    let residual_ref = format!("residual:mabo:r{index}");
    let source_revision_ref = format!("wikidata:Q1501525:oldid:{}", 2333409615u64 + index as u64);
    PreparedWorldExpansionCycle {
        next_frontier_ref: format!("frontier:mabo:{}", index + 1),
        routing: ResidualRouting {
            residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            routing_reason_ref: format!("review:mabo:{index}"),
        },
        candidates: vec![ExpansionCandidate {
            candidate_ref: format!("candidate:{index}"),
            object_ref: object_ref.into(),
            object_kind: KnowledgeObjectKind::Qid,
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            producer_lane: ProducerLane::WikidataIdentity,
            source_revision_ref: Some(source_revision_ref.clone()),
            expected_residual_contraction: 1,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 1,
            admissible: true,
        }],
        review_decision: ReviewDecision::Reviewed,
        disambiguation_outcome: DisambiguationOutcome::NewRelatedObject,
        identity_resolution: WorldIdentityResolutionReceipt {
            receipt_ref: format!("identity:{index}"),
            identity: WorldObjectIdentity::new(identity_class_ref, object_ref),
            resolution_kind: WorldIdentityResolutionKind::RelatedObject,
            evidence_ref: format!("identity-evidence:{index}"),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        observation: PostAcquisitionWorldObservation {
            observation_ref: format!("observation:{index}"),
            source_revision_ref,
            triggering_residual_ref: residual_ref,
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 1,
            newly_exposed_residuals: next_residual.map(residual).into_iter().collect(),
            pnf_world_disambiguation_ref: format!("pnf-world:{index}"),
            observation_authority: "experimental_candidate_only",
        },
    }
}

struct ConflictingSource {
    next: usize,
}

impl WorldExpansionCycleSource for ConflictingSource {
    fn prepare_next_cycle(
        &mut self,
        _session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        let cycle = match self.next {
            0 => prepared(0, "Q975866", "world-object:eddie-mabo", Some("residual:mabo:r1")),
            1 => prepared(1, "Q975866", "world-object:wrong-second-class", None),
            _ => {
                return Err(RecurrentRunBlocker::new(
                    RecurrentRunBlockerKind::NoPreparedCycle,
                    "source:exhausted",
                ))
            }
        };
        self.next += 1;
        Ok(cycle)
    }
}

#[derive(Default)]
struct AcceptingSink;

impl WorldExpansionCycleSink for AcceptingSink {
    fn persist_cycle(
        &mut self,
        _receipt: &WorldExpansionCycleReceipt,
    ) -> Result<(), RecurrentRunBlocker> {
        Ok(())
    }
}

#[test]
fn conflicting_identity_class_for_committed_representation_blocks_second_cycle() {
    let frontier = ProofFrontier {
        consumer_ref: "consumer:mabo-100-identity-classes".into(),
        frontier_ref: "frontier:mabo:0".into(),
        residuals: vec![residual("residual:mabo:r0")],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };
    let mut session = WorldExpansionSession::new(frontier, mabo_world_expansion_policy());
    let mut source = IdentityCoherentCycleSource::new(ConflictingSource { next: 0 });
    let mut sink = AcceptingSink;

    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 10 },
    )
    .unwrap();

    assert_eq!(receipt.cycles_committed, 1);
    assert_eq!(receipt.final_novel_identity_classes, 1);
    assert_eq!(
        receipt.stop_reason,
        RecurrentRunStopReason::Blocked(RecurrentRunBlocker::new(
            RecurrentRunBlockerKind::IdentityReviewRequired,
            "identity-coherence:representation:Q975866:existing:world-object:eddie-mabo:proposed:world-object:wrong-second-class",
        ))
    );
}
