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

fn residual(id: &str, status: ResidualStatus) -> ProofResidual {
    ProofResidual {
        residual_ref: id.into(),
        proposition_ref: format!("proposition:{id}"),
        producer_class_ref: "producer:world-expansion".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status,
    }
}

fn frontier() -> ProofFrontier {
    ProofFrontier {
        consumer_ref: "consumer:mabo-100-identity-classes".into(),
        frontier_ref: "frontier:mabo:0".into(),
        residuals: vec![residual("residual:mabo:r0", ResidualStatus::Open)],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

fn prepared(index: usize, next_residual: Option<&str>) -> PreparedWorldExpansionCycle {
    let residual_ref = format!("residual:mabo:r{index}");
    let object_ref = format!("Q{}", 975866 + index);
    let identity_class_ref = format!("world-object:mabo:{index}");
    let source_revision_ref = format!("wikidata:{object_ref}:oldid:{}", 2333409615u64 + index as u64);
    let candidate = ExpansionCandidate {
        candidate_ref: format!("wikidata:candidate:{index}"),
        object_ref: object_ref.clone(),
        object_kind: KnowledgeObjectKind::Qid,
        discovery_parent_ref: "Q1501525".into(),
        triggering_residual_ref: residual_ref.clone(),
        residual_class: ResidualClass::Identity,
        producer_lane: ProducerLane::WikidataIdentity,
        source_revision_ref: Some(source_revision_ref.clone()),
        expected_residual_contraction: 2,
        provenance_quality: 5,
        same_object_confidence: 5,
        expected_new_world_value: 5,
        acquisition_cost: 1,
        admissible: true,
    };
    let identity_resolution = WorldIdentityResolutionReceipt {
        receipt_ref: format!("identity-resolution:mabo:{index}"),
        identity: WorldObjectIdentity::new(identity_class_ref, object_ref),
        resolution_kind: WorldIdentityResolutionKind::RelatedObject,
        evidence_ref: format!("identity-evidence:mabo:{index}"),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let newly_exposed_residuals = next_residual
        .map(|id| vec![residual(id, ResidualStatus::Open)])
        .unwrap_or_default();
    PreparedWorldExpansionCycle {
        next_frontier_ref: format!("frontier:mabo:{}", index + 1),
        routing: ResidualRouting {
            residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            routing_reason_ref: format!("pnf-world:mabo:routing:{index}"),
        },
        candidates: vec![candidate],
        review_decision: ReviewDecision::Reviewed,
        disambiguation_outcome: DisambiguationOutcome::NewRelatedObject,
        identity_resolution,
        observation: PostAcquisitionWorldObservation {
            observation_ref: format!("world-observation:mabo:{index}"),
            source_revision_ref,
            triggering_residual_ref: residual_ref,
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 1,
            newly_exposed_residuals,
            pnf_world_disambiguation_ref: format!("pnf-world:mabo:{index}"),
            observation_authority: "experimental_candidate_only",
        },
    }
}

struct TwoCycleSource {
    next: usize,
}

impl WorldExpansionCycleSource for TwoCycleSource {
    fn prepare_next_cycle(
        &mut self,
        _session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        let cycle = match self.next {
            0 => prepared(0, Some("residual:mabo:r1")),
            1 => prepared(1, None),
            _ => {
                return Err(RecurrentRunBlocker::new(
                    RecurrentRunBlockerKind::NoPreparedCycle,
                    "source:no-more-cycles",
                ))
            }
        };
        self.next += 1;
        Ok(cycle)
    }
}

#[derive(Default)]
struct RecordingSink {
    persisted: usize,
    fail_after: Option<usize>,
}

impl WorldExpansionCycleSink for RecordingSink {
    fn persist_cycle(
        &mut self,
        _receipt: &WorldExpansionCycleReceipt,
    ) -> Result<(), RecurrentRunBlocker> {
        if self.fail_after == Some(self.persisted) {
            return Err(RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::PersistenceBlocked,
                "pg:lineage-write-blocked",
            ));
        }
        self.persisted += 1;
        Ok(())
    }
}

#[test]
fn recurrent_runner_commits_only_after_sink_and_stops_on_frontier_exhaustion() {
    let mut session = WorldExpansionSession::new(frontier(), mabo_world_expansion_policy());
    let mut source = TwoCycleSource { next: 0 };
    let mut sink = RecordingSink::default();
    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 10 },
    )
    .unwrap();

    assert_eq!(receipt.cycles_committed, 2);
    assert_eq!(receipt.final_novel_identity_classes, 2);
    assert_eq!(receipt.candidates_seen, 2);
    assert_eq!(receipt.candidates_rejected, 0);
    assert_eq!(receipt.duplicates_seen, 0);
    assert_eq!(receipt.identity_ambiguous, 0);
    assert_eq!(receipt.reviewed_objects, 2);
    assert_eq!(receipt.new_qids_admitted, 2);
    assert_eq!(receipt.new_articles_admitted, 0);
    assert_eq!(receipt.new_primary_legal_sources_admitted, 0);
    assert_eq!(receipt.new_other_world_objects_admitted, 0);
    assert_eq!(sink.persisted, 2);
    assert_eq!(receipt.stop_reason, RecurrentRunStopReason::FrontierExhausted);
    assert_eq!(session.ledger.total_new_world_objects, 2);
    assert!(session.frontier.open_residuals().next().is_none());
}

#[test]
fn sink_blocker_rolls_back_staged_cycle_and_retains_blocker_kind() {
    let mut session = WorldExpansionSession::new(frontier(), mabo_world_expansion_policy());
    let mut source = TwoCycleSource { next: 0 };
    let mut sink = RecordingSink {
        persisted: 0,
        fail_after: Some(0),
    };
    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 10 },
    )
    .unwrap();

    assert_eq!(receipt.cycles_committed, 0);
    assert_eq!(receipt.final_novel_identity_classes, 0);
    assert_eq!(receipt.candidates_seen, 0);
    assert_eq!(receipt.reviewed_objects, 0);
    assert_eq!(
        receipt.stop_reason,
        RecurrentRunStopReason::Blocked(RecurrentRunBlocker::new(
            RecurrentRunBlockerKind::PersistenceBlocked,
            "pg:lineage-write-blocked",
        ))
    );
    assert_eq!(session.ledger.total_new_world_objects, 0);
    assert_eq!(session.frontier.frontier_ref, "frontier:mabo:0");
}
