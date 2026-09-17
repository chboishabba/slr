use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::ResidualAssessmentKind;
use sensiblaw_proof_search_loop::world_expansion::{
    DisambiguationOutcome, ExpansionCandidate, KnowledgeObjectKind, ProducerLane, ResidualClass,
    ReviewDecision,
};
use sensiblaw_proof_search_loop::world_expansion_reentry::PostAcquisitionWorldObservation;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    PreparedWorldExpansionCycle, RecurrentRunBlocker, RecurrentRunBlockerKind,
    WorldExpansionCycleSource,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionSession;
use sensiblaw_proof_search_loop::world_expansion_step::ResidualRouting;
use sensiblaw_proof_search_loop::world_identity::{
    WorldIdentityResolutionKind, WorldIdentityResolutionReceipt, WorldObjectIdentity,
};
use sensiblaw_world_expansion_runtime::{
    KnownIdentityPaymentSink, ReviewedCycleQueueSource, ReviewedPreparedCycle,
};

fn prepared() -> PreparedWorldExpansionCycle {
    let residual_ref = "residual:mabo:world-identity:Q975866".to_owned();
    PreparedWorldExpansionCycle {
        next_frontier_ref: "frontier:mabo:reviewed:1".into(),
        routing: ResidualRouting {
            residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            routing_reason_ref: "consumer:mabo-context-world-identity".into(),
        },
        candidates: vec![ExpansionCandidate {
            candidate_ref: "wikidata:Q1501525:P710:Q975866".into(),
            object_ref: "Q975866".into(),
            object_kind: KnowledgeObjectKind::Qid,
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: residual_ref.clone(),
            residual_class: ResidualClass::Identity,
            producer_lane: ProducerLane::WikidataIdentity,
            source_revision_ref: Some("wikidata:Q1501525:oldid:2333409615".into()),
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
            receipt_ref: "review:mabo:eddie".into(),
            identity: WorldObjectIdentity::new("world-object:eddie-mabo", "Q975866"),
            resolution_kind: WorldIdentityResolutionKind::SameObjectDifferentRepresentation,
            evidence_ref: "review:mabo:eddie".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        },
        observation: PostAcquisitionWorldObservation {
            observation_ref: "query:mabo:P710:Q975866".into(),
            source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
            triggering_residual_ref: residual_ref,
            assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
            observed_residual_contraction: 1,
            newly_exposed_residuals: Vec::<ProofResidual>::new(),
            pnf_world_disambiguation_ref: "review:mabo:eddie".into(),
            observation_authority: "experimental_candidate_only",
        },
    }
}

fn session() -> WorldExpansionSession {
    use sensiblaw_proof_search_loop::frontier::ProofFrontier;
    use sensiblaw_proof_search_loop::world_expansion::WorldExpansionPolicy;

    WorldExpansionSession::new(
        ProofFrontier {
            consumer_ref: "consumer:mabo-context-world-identity".into(),
            frontier_ref: "frontier:mabo:0".into(),
            residuals: vec![ProofResidual {
                residual_ref: "residual:mabo:world-identity:Q975866".into(),
                proposition_ref: "mabo:world-identity:Q975866".into(),
                producer_class_ref: "producer:world-expansion".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: None,
                salience: 100,
                dependency_refs: vec![],
                status: ResidualStatus::Open,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        },
        WorldExpansionPolicy {
            target_novel_objects: 100,
            minimum_expected_residual_contraction: 1,
        },
    )
}

#[derive(Debug, Default)]
struct RecordingPaymentSink {
    attempts: usize,
    persisted: Vec<Vec<u8>>,
    fail_first: bool,
}

impl KnownIdentityPaymentSink for RecordingPaymentSink {
    fn persist_reviewed_payment(&mut self, wire: &[u8]) -> Result<(), RecurrentRunBlocker> {
        self.attempts += 1;
        if self.fail_first {
            self.fail_first = false;
            return Err(RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::PersistenceBlocked,
                "payment:blocked-once",
            ));
        }
        self.persisted.push(wire.to_vec());
        Ok(())
    }
}

#[test]
fn empty_reviewed_queue_blocks_as_identity_review_required() {
    let mut source = ReviewedCycleQueueSource::new(Vec::new(), RecordingPaymentSink::default());
    let blocker = source
        .prepare_next_cycle(&session())
        .expect_err("no reviewed cycle must not become campaign work");

    assert_eq!(blocker.kind, RecurrentRunBlockerKind::IdentityReviewRequired);
    assert_eq!(blocker.blocker_ref, "campaign:identity-review-required");
    assert_eq!(source.pending_cycles(), 0);
}

#[test]
fn reviewed_payment_is_persisted_before_cycle_is_released() {
    let cycle = ReviewedPreparedCycle {
        prepared: prepared(),
        reviewed_payment_wire: b"review-payment".to_vec(),
    };
    let mut source = ReviewedCycleQueueSource::new(vec![cycle.clone()], RecordingPaymentSink::default());

    let released = source.prepare_next_cycle(&session()).unwrap();

    assert_eq!(released, cycle.prepared);
    assert_eq!(source.pending_cycles(), 0);
    assert_eq!(source.payment_sink().attempts, 1);
    assert_eq!(source.payment_sink().persisted, vec![b"review-payment".to_vec()]);
}

#[test]
fn failed_payment_persistence_leaves_cycle_queued_for_retry() {
    let cycle = ReviewedPreparedCycle {
        prepared: prepared(),
        reviewed_payment_wire: b"review-payment".to_vec(),
    };
    let sink = RecordingPaymentSink {
        fail_first: true,
        ..RecordingPaymentSink::default()
    };
    let mut source = ReviewedCycleQueueSource::new(vec![cycle.clone()], sink);

    let blocker = source.prepare_next_cycle(&session()).unwrap_err();
    assert_eq!(blocker.kind, RecurrentRunBlockerKind::PersistenceBlocked);
    assert_eq!(source.pending_cycles(), 1);
    assert_eq!(source.payment_sink().attempts, 1);
    assert!(source.payment_sink().persisted.is_empty());

    let released = source.prepare_next_cycle(&session()).unwrap();
    assert_eq!(released, cycle.prepared);
    assert_eq!(source.pending_cycles(), 0);
    assert_eq!(source.payment_sink().attempts, 2);
    assert_eq!(source.payment_sink().persisted.len(), 1);
}
