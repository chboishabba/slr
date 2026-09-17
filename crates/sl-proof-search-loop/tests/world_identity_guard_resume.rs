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

fn prepared(object_ref: &str, identity_class_ref: &str) -> PreparedWorldExpansionCycle {
    let residual_ref = "residual:mabo:r0".to_owned();
    let revision = "wikidata:Q1501525:oldid:2333409615".to_owned();
    PreparedWorldExpansionCycle {
        next_frontier_ref: "frontier:mabo:1".into(),
        routing: ResidualRouting {
            residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            routing_reason_ref: "review:resume".into(),
        },
        candidates: vec![ExpansionCandidate {
            candidate_ref: "candidate:resume".into(),
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
            receipt_ref: "identity:resume".into(),
            identity: WorldObjectIdentity::new(identity_class_ref, object_ref),
            resolution_kind: WorldIdentityResolutionKind::RelatedObject,
            evidence_ref: "evidence:resume".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        observation: PostAcquisitionWorldObservation {
            observation_ref: "observation:resume".into(),
            source_revision_ref: revision,
            triggering_residual_ref: residual_ref,
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 1,
            newly_exposed_residuals: vec![],
            pnf_world_disambiguation_ref: "pnf-world:resume".into(),
            observation_authority: "experimental_candidate_only",
        },
    }
}

fn baseline() -> IdentityCoherenceBaseline {
    let mut identity_class_refs = BTreeSet::new();
    identity_class_refs.insert("world-object:eddie-mabo".to_owned());
    let mut representation_identity_class_refs = BTreeMap::new();
    representation_identity_class_refs.insert(
        "Q975866".to_owned(),
        "world-object:eddie-mabo".to_owned(),
    );
    IdentityCoherenceBaseline {
        identity_class_refs,
        representation_identity_class_refs,
    }
}

struct OneSource(PreparedWorldExpansionCycle);

impl WorldExpansionCycleSource for OneSource {
    fn prepare_next_cycle(
        &mut self,
        _session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        Ok(self.0.clone())
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

fn session() -> WorldExpansionSession {
    WorldExpansionSession::new(
        ProofFrontier {
            consumer_ref: "consumer:mabo-100-identity-classes".into(),
            frontier_ref: "frontier:mabo:0".into(),
            residuals: vec![residual("residual:mabo:r0")],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        },
        WorldExpansionPolicy {
            target_novel_objects: 99,
            minimum_expected_residual_contraction: 1,
        },
    )
}

#[test]
fn durable_known_identity_is_not_recounted_as_novel() {
    let mut session = session();
    let mut source = IdentityCoherentCycleSource::with_baseline(
        OneSource(prepared("Q975866", "world-object:eddie-mabo")),
        baseline(),
    );
    let mut sink = AcceptingSink;

    let receipt = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 1 },
    )
    .unwrap();

    assert_eq!(receipt.cycles_committed, 0);
    assert_eq!(receipt.final_novel_identity_classes, 0);
    assert_eq!(
        receipt.stop_reason,
        RecurrentRunStopReason::Blocked(RecurrentRunBlocker::new(
            RecurrentRunBlockerKind::WorldDiagnosisRequired,
            "identity-coherence:known-identity:world-object:eddie-mabo:requires-non-novel-payment",
        ))
    );
}

#[test]
fn seeded_guard_blocks_conflict_against_durable_identity_assignment() {
    let mut session = session();
    let mut source = IdentityCoherentCycleSource::with_baseline(
        OneSource(prepared("Q975866", "world-object:wrong")),
        baseline(),
    );
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
