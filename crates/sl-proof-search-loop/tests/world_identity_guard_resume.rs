use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{
    DisambiguationOutcome, ExpansionCandidate, KnowledgeObjectKind, ProducerLane, ResidualClass,
    ReviewDecision, WorldExpansionPolicy,
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
use sensiblaw_proof_search_loop::world_identity_guard::{
    IdentityCoherentCycleSource, IdentityCoherenceBaseline,
};

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

fn prepared(index: usize, object_ref: &str, identity_class_ref: &str, next: Option<&str>) -> PreparedWorldExpansionCycle {
    let residual_ref = format!("residual:mabo:r{index}");
    let revision = format!("wikidata:Q1501525:oldid:{}", 2333409615u64 + index as u64);
    PreparedWorldExpansionCycle {
        next_frontier_ref: format!("frontier:mabo:{}", index + 1),
        routing: ResidualRouting {
            residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            routing_reason_ref: format!("review:{index}"),
        },
        candidates: vec![ExpansionCandidate {
            candidate_ref: format!("candidate:{index}"),
            object_ref: object_ref.into(),
            object_kind: KnowledgeObjectKind::Qid,
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            producer_lane: ProducerLane::WikidataIdentity,
            source_revision_ref: Some(revision.clone()),
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
            evidence_ref: format!("evidence:{index}"),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        observation: PostAcquisitionWorldObservation {
            observation_ref: format!("observation:{index}"),
            source_revision_ref: revision,
            triggering_residual_ref: residual_ref,
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 1,
            newly_exposed_residuals: next.map(residual).into_iter().collect(),
            pnf_world_disambiguation_ref: format!("pnf-world:{index}"),
            observation_authority: "experimental_candidate_only",
        },
    }
}

struct DuplicateThenNovelSource {
    next: usize,
}

impl WorldExpansionCycleSource for DuplicateThenNovelSource {
    fn prepare_next_cycle(
        &mut self,
        _session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        let cycle = match self.next {
            0 => prepared(0, "Q975866", "world-object:eddie-mabo", Some("residual:mabo:r0")),
            1 => prepared(0, "Q3778295", "world-object:justice-brennan", None),
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
fn seeded_guard_skips_durable_duplicate_and_commits_next_novel_identity() {
    let mut identity_class_refs = BTreeSet::new();
    identity_class_refs.insert("world-object:eddie-mabo".to_owned());
    let mut representation_identity_class_refs = BTreeMap::new();
    representation_identity_class_refs.insert(
        "Q975866".to_owned(),
        "world-object:eddie-mabo".to_owned(),
    );
    let baseline = IdentityCoherenceBaseline {
        identity_class_refs,
        representation_identity_class_refs,
    };

    let frontier = ProofFrontier {
        consumer_ref: "consumer:mabo-100-identity-classes".into(),
        frontier_ref: "frontier:mabo:0".into(),
        residuals: vec![residual("residual:mabo:r0")],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };
    let mut session = WorldExpansionSession::new(
        frontier,
        WorldExpansionPolicy {
            target_novel_objects: 99,
            minimum_expected_residual_contraction: 1,
        },
    );
    let mut source = IdentityCoherentCycleSource::with_baseline(
        DuplicateThenNovelSource { next: 0 },
        baseline,
    );
    let mut sink = AcceptingSink;

    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 1 },
    )
    .unwrap();

    assert_eq!(receipt.cycles_committed, 1);
    assert_eq!(receipt.final_novel_identity_classes, 1);
    assert_eq!(session.ledger.total_new_world_objects, 1);
}

#[test]
fn seeded_guard_blocks_conflict_against_durable_identity_assignment() {
    let mut identity_class_refs = BTreeSet::new();
    identity_class_refs.insert("world-object:eddie-mabo".to_owned());
    let mut representation_identity_class_refs = BTreeMap::new();
    representation_identity_class_refs.insert(
        "Q975866".to_owned(),
        "world-object:eddie-mabo".to_owned(),
    );
    let baseline = IdentityCoherenceBaseline {
        identity_class_refs,
        representation_identity_class_refs,
    };

    struct ConflictSource;
    impl WorldExpansionCycleSource for ConflictSource {
        fn prepare_next_cycle(
            &mut self,
            _session: &WorldExpansionSession,
        ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
            Ok(prepared(0, "Q975866", "world-object:wrong", None))
        }
    }

    let frontier = ProofFrontier {
        consumer_ref: "consumer:mabo-100-identity-classes".into(),
        frontier_ref: "frontier:mabo:0".into(),
        residuals: vec![residual("residual:mabo:r0")],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };
    let mut session = WorldExpansionSession::new(
        frontier,
        WorldExpansionPolicy {
            target_novel_objects: 99,
            minimum_expected_residual_contraction: 1,
        },
    );
    let mut source = IdentityCoherentCycleSource::with_baseline(ConflictSource, baseline);
    let mut sink = AcceptingSink;

    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 1 },
    )
    .unwrap();

    assert_eq!(receipt.cycles_committed, 0);
    assert_eq!(
        receipt.stop_reason,
        RecurrentRunStopReason::Blocked(RecurrentRunBlocker::new(
            RecurrentRunBlockerKind::IdentityReviewRequired,
            "identity-coherence:representation:Q975866:existing:world-object:eddie-mabo:proposed:world-object:wrong",
        ))
    );
}
