use std::collections::VecDeque;

use sensiblaw_pg_source_store::GwbHopLedgerRow;
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
    WorldIdentityResolutionReceipt, WorldObjectIdentity,
};
use sensiblaw_world_expansion_runtime::sprint1_acquisition_machine::replay_campaign_head;

fn residual(
    residual_ref: &str,
    class_ref: &str,
    jurisdiction_ref: Option<&str>,
) -> ProofResidual {
    ProofResidual {
        residual_ref: residual_ref.into(),
        proposition_ref: format!("proposition:{residual_ref}"),
        producer_class_ref: class_ref.into(),
        jurisdiction_ref: jurisdiction_ref.map(str::to_owned),
        authority_requirement_ref: None,
        salience: 10,
        dependency_refs: vec![],
        status: ResidualStatus::Open,
    }
}

fn candidate(
    residual_ref: &str,
    candidate_ref: &str,
    object_ref: &str,
    object_kind: KnowledgeObjectKind,
    residual_class: ResidualClass,
    producer_lane: ProducerLane,
    source_revision_ref: &str,
) -> ExpansionCandidate {
    ExpansionCandidate {
        candidate_ref: candidate_ref.into(),
        object_ref: object_ref.into(),
        object_kind,
        discovery_parent_ref: "sprint1:root".into(),
        triggering_residual_ref: residual_ref.into(),
        residual_class,
        producer_lane,
        source_revision_ref: Some(source_revision_ref.into()),
        expected_residual_contraction: 1,
        provenance_quality: 5,
        same_object_confidence: 5,
        expected_new_world_value: 5,
        acquisition_cost: 1,
        admissible: true,
    }
}

fn identity(
    receipt_ref: &str,
    class_ref: &str,
    object_ref: &str,
) -> WorldIdentityResolutionReceipt {
    WorldIdentityResolutionReceipt::same_object(
        receipt_ref,
        WorldObjectIdentity::new(class_ref, object_ref),
        format!("reviewed-identity:{object_ref}"),
    )
}

fn observation(
    index: usize,
    residual_ref: &str,
    source_revision_ref: &str,
    next: Option<ProofResidual>,
) -> PostAcquisitionWorldObservation {
    PostAcquisitionWorldObservation {
        observation_ref: format!("sprint1:observation:{index}"),
        source_revision_ref: source_revision_ref.into(),
        triggering_residual_ref: residual_ref.into(),
        assessment_kind: ResidualAssessmentKind::SatisfiedCandidate,
        observed_residual_contraction: 1,
        newly_exposed_residuals: next.into_iter().collect(),
        pnf_world_disambiguation_ref: format!("sprint1:pnf-world:{index}"),
        observation_authority: "experimental_candidate_only",
    }
}

struct AdaptiveFixtureSource {
    prepared_residuals: Vec<String>,
}

impl AdaptiveFixtureSource {
    fn new() -> Self {
        Self {
            prepared_residuals: Vec::new(),
        }
    }
}

impl WorldExpansionCycleSource for AdaptiveFixtureSource {
    fn prepare_next_cycle(
        &mut self,
        session: &WorldExpansionSession,
    ) -> Result<PreparedWorldExpansionCycle, RecurrentRunBlocker> {
        let open = session
            .frontier
            .open_residuals()
            .next()
            .ok_or_else(|| {
                RecurrentRunBlocker::new(
                    RecurrentRunBlockerKind::NoPreparedCycle,
                    "sprint1:no-open-residual",
                )
            })?
            .clone();

        self.prepared_residuals.push(open.residual_ref.clone());
        let index = self.prepared_residuals.len() - 1;

        let (
            residual_class,
            producer_lane,
            object_ref,
            object_kind,
            source_revision_ref,
            outcome,
            next,
        ) = match open.residual_ref.as_str() {
            "r:classification" => (
                ResidualClass::Identity,
                ProducerLane::WikidataIdentity,
                "Q100",
                KnowledgeObjectKind::Qid,
                "wikidata-p31-p279-slice:fixture:Q100",
                DisambiguationOutcome::NewConceptualParent,
                Some(residual(
                    "r:authority",
                    "producer:authority-source",
                    Some("AU"),
                )),
            ),
            "r:authority" => (
                ResidualClass::Legal,
                ProducerLane::GovernedLegal,
                "case:[2026]-HCA-19",
                KnowledgeObjectKind::PrimaryLegalSource,
                "oalc:[2026]-HCA-19:fixture",
                DisambiguationOutcome::NewEvidentiarySource,
                Some(residual(
                    "r:provenance",
                    "producer:source-provenance",
                    Some("AU"),
                )),
            ),
            "r:provenance" => (
                ResidualClass::Provenance,
                ProducerLane::SourceSpecificProvenance,
                "source:provenance:fixture",
                KnowledgeObjectKind::Other,
                "source-revision:fixture:3",
                DisambiguationOutcome::NewSourceManifestation,
                None,
            ),
            other => {
                return Err(RecurrentRunBlocker::new(
                    RecurrentRunBlockerKind::WorldDiagnosisRequired,
                    format!("sprint1:unexpected-residual:{other}"),
                ))
            }
        };

        let candidate = candidate(
            &open.residual_ref,
            &format!("candidate:{index}"),
            object_ref,
            object_kind,
            residual_class,
            producer_lane,
            source_revision_ref,
        );

        Ok(PreparedWorldExpansionCycle {
            next_frontier_ref: format!("frontier:sprint1:{}", index + 1),
            routing: ResidualRouting {
                residual_ref: open.residual_ref.clone(),
                residual_class,
                routing_reason_ref: format!("sprint1:diagnosis:{index}"),
            },
            candidates: vec![candidate],
            review_decision: ReviewDecision::Reviewed,
            disambiguation_outcome: outcome,
            identity_resolution: identity(
                &format!("identity-resolution:{index}"),
                &format!("identity-class:{index}"),
                object_ref,
            ),
            observation: observation(index, &open.residual_ref, source_revision_ref, next),
        })
    }
}

#[derive(Default)]
struct RecordingSink {
    receipts: Vec<WorldExpansionCycleReceipt>,
}

impl WorldExpansionCycleSink for RecordingSink {
    fn persist_cycle(
        &mut self,
        receipt: &WorldExpansionCycleReceipt,
    ) -> Result<(), RecurrentRunBlocker> {
        self.receipts.push(receipt.clone());
        Ok(())
    }
}

fn digest(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn lane_ref(lane: ProducerLane) -> &'static str {
    match lane {
        ProducerLane::GovernedLegal => "producer:authority-source",
        ProducerLane::WikidataIdentity => "producer:wikidata-classification",
        ProducerLane::WikipediaContext => "producer:wikipedia-context",
        ProducerLane::SourceSpecificProvenance => "producer:source-provenance",
        ProducerLane::Other => "producer:other",
    }
}

#[test]
fn sprint1_capstone_reenters_fresh_frontier_switches_producers_and_replays_same_head() {
    let frontier = ProofFrontier {
        consumer_ref: "consumer:sprint1-capstone".into(),
        frontier_ref: "frontier:sprint1:0".into(),
        residuals: vec![residual(
            "r:classification",
            "producer:wikidata-classification",
            None,
        )],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };
    let mut session = WorldExpansionSession::new(
        frontier,
        WorldExpansionPolicy {
            target_novel_objects: 3,
            minimum_expected_residual_contraction: 1,
        },
    );
    let mut source = AdaptiveFixtureSource::new();
    let mut sink = RecordingSink::default();

    let run = run_recurrent_world_expansion(
        &mut session,
        &mut source,
        &mut sink,
        WorldExpansionRunnerConfig { max_cycles: 8 },
    )
    .unwrap();

    assert_eq!(run.cycles_committed, 3);
    assert_eq!(run.final_novel_identity_classes, 3);
    assert_eq!(run.stop_reason, RecurrentRunStopReason::TargetComplete);
    assert_eq!(
        source.prepared_residuals,
        vec!["r:classification", "r:authority", "r:provenance"]
    );
    assert_eq!(sink.receipts.len(), 3);
    assert_eq!(
        sink.receipts
            .iter()
            .map(|receipt| receipt.step.selected_producer_lane)
            .collect::<Vec<_>>(),
        vec![
            ProducerLane::WikidataIdentity,
            ProducerLane::GovernedLegal,
            ProducerLane::SourceSpecificProvenance,
        ]
    );
    assert!(session.frontier.open_residuals().next().is_none());

    let mut rows = VecDeque::new();
    let mut prior_receipt: Option<String> = None;
    let worlds = [('a', 'b'), ('b', 'c'), ('c', 'd')];

    for (index, receipt) in sink.receipts.iter().enumerate() {
        let receipt_sha256 = (char::from_u32('1' as u32 + index as u32).unwrap())
            .to_string()
            .repeat(64);
        rows.push_back(GwbHopLedgerRow {
            campaign_ref: "campaign:sprint1-capstone".into(),
            hop_index: index,
            prior_receipt_sha256: prior_receipt.clone(),
            world_before_sha256: digest(worlds[index].0),
            frontier_sha256: digest(char::from_u32('e' as u32 + index as u32).unwrap()),
            selected_move_ref: receipt.step.selected_candidate_ref.clone(),
            investigation_kind_ref: "sprint1-capstone".into(),
            producer_ref: lane_ref(receipt.step.selected_producer_lane).into(),
            source_revision_ref: receipt
                .step
                .selected_source_revision_ref
                .clone()
                .unwrap(),
            evidence_digest_ref: digest('9'),
            review_ref: format!("review:sprint1:{index}"),
            outcome_ref: "paid".into(),
            residual_effect_ref: "close-reviewed".into(),
            world_after_sha256: digest(worlds[index].1),
            closed_residual_refs: vec![receipt.step.residual_ref.clone()],
            opened_residual_refs: receipt.reentry.new_residual_refs.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            receipt_authority: "gwb_adaptive_runtime_review_only",
            receipt_sha256: receipt_sha256.clone(),
        });
        prior_receipt = Some(receipt_sha256);
    }

    let replay = replay_campaign_head(
        "campaign:sprint1-capstone",
        &rows.into_iter().collect::<Vec<_>>(),
    )
    .unwrap();

    assert_eq!(replay.completed_hops, 3);
    assert_eq!(replay.world_sha256, Some(digest('d')));
    assert_eq!(replay.producer_refs.len(), 3);
    assert!(replay.candidate_only);
    assert!(!replay.creates_semantic_authority);
}
