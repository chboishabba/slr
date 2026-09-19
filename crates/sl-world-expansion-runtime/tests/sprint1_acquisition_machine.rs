use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_pg_source_store::GwbHopLedgerRow;
use sensiblaw_route_selector::ProducerFamily;
use sensiblaw_world_expansion_runtime::sprint1_acquisition_machine::{
    execute_physical_acquisition, plan_physical_acquisition,
    producer_execution_plan_from_candidate, replay_campaign_head,
    AcquisitionPath, BoundedPhysicalTransport, CandidateProducerEvidence,
    LogicalAcquisitionRequest, LogicalToPhysicalPlanner, PhysicalObjectFetch,
    PlannedPhysicalObject, ProducerExecutionPlan, ProducerExecutor,
    ResolvedPhysicalObjects, Sprint1AcquisitionError, Sprint1ProducerController,
    MAX_COLD_REMOTE_PHYSICAL_OBJECTS,
};

fn digest(ch: char) -> String {
    format!("sha256:{}", ch.to_string().repeat(64))
}

fn object(name: &str, path: AcquisitionPath) -> PlannedPhysicalObject {
    PlannedPhysicalObject {
        physical_object_ref: format!("object:{name}"),
        cache_key: format!("cache:{name}"),
        route_ref: format!("nodeRouteIndex:{name}"),
        source_ref: "hf://wikidata".into(),
        source_range: None,
        expected_bytes: Some(128),
        acquisition_path: path,
    }
}

#[derive(Default)]
struct FixturePlanner {
    rows: BTreeMap<String, Vec<PlannedPhysicalObject>>,
}

impl LogicalToPhysicalPlanner for FixturePlanner {
    fn resolve(
        &mut self,
        request: &LogicalAcquisitionRequest,
    ) -> Result<ResolvedPhysicalObjects, Sprint1AcquisitionError> {
        Ok(ResolvedPhysicalObjects {
            resolved_node_ref: format!("node:{}", request.semantic_target_ref),
            objects: self
                .rows
                .get(&request.semantic_target_ref)
                .cloned()
                .unwrap_or_default(),
        })
    }
}

#[derive(Default)]
struct FixtureTransport {
    cached_refs: BTreeSet<String>,
    cold_batch_widths: Vec<usize>,
    cold_calls: usize,
    incomplete_refs: BTreeSet<String>,
    live_refs: BTreeSet<String>,
}

impl FixtureTransport {
    fn fetch(&self, object: &PlannedPhysicalObject, from_cache: bool) -> PhysicalObjectFetch {
        PhysicalObjectFetch {
            physical_object_ref: object.physical_object_ref.clone(),
            bytes: if from_cache { 0 } else { 128 },
            from_cache,
            live_fallback: self.live_refs.contains(&object.physical_object_ref),
            complete: !self.incomplete_refs.contains(&object.physical_object_ref),
            source_revision_ref: format!("revision:{}", object.physical_object_ref),
            evidence_digest_ref: digest('a'),
        }
    }
}

impl BoundedPhysicalTransport for FixtureTransport {
    fn cached(
        &mut self,
        object: &PlannedPhysicalObject,
    ) -> Result<Option<PhysicalObjectFetch>, Sprint1AcquisitionError> {
        Ok(self
            .cached_refs
            .contains(&object.physical_object_ref)
            .then(|| self.fetch(object, true)))
    }

    fn fetch_cold_batch(
        &mut self,
        objects: &[PlannedPhysicalObject],
    ) -> Result<Vec<PhysicalObjectFetch>, Sprint1AcquisitionError> {
        assert!(objects.len() <= MAX_COLD_REMOTE_PHYSICAL_OBJECTS);
        self.cold_batch_widths.push(objects.len());
        self.cold_calls += 1;
        Ok(objects.iter().map(|object| self.fetch(object, false)).collect())
    }
}

fn request(id: &str, qid: &str, producer: ProducerFamily) -> LogicalAcquisitionRequest {
    LogicalAcquisitionRequest {
        request_ref: id.into(),
        semantic_target_ref: qid.into(),
        producer,
    }
}

#[test]
fn physical_plan_dedupes_before_concurrency() {
    let shared = object("shared", AcquisitionPath::SpecializedP31P279Slice);
    let mut planner = FixturePlanner::default();
    planner
        .rows
        .insert("Q1".into(), vec![shared.clone(), object("a", AcquisitionPath::RouteAwareGeneralSnapshot)]);
    planner
        .rows
        .insert("Q2".into(), vec![shared, object("b", AcquisitionPath::RouteAwareGeneralSnapshot)]);

    let plan = plan_physical_acquisition(
        &[
            request("r1", "Q1", ProducerFamily::ClassificationEvidence),
            request("r2", "Q2", ProducerFamily::ClassificationEvidence),
        ],
        &mut planner,
    )
    .unwrap();

    assert_eq!(plan.planned_object_references, 4);
    assert_eq!(plan.unique_objects.len(), 3);
    assert_eq!(plan.coalesced_object_references, 1);
    assert_eq!(plan.specialised_slice_objects, 1);
    assert_eq!(plan.route_aware_general_objects, 2);
    assert_eq!(plan.max_cold_remote_objects, 5);
}

#[test]
fn cache_hit_performs_zero_remote_gets() {
    let cached = object("cached", AcquisitionPath::SpecializedP31P279Slice);
    let mut planner = FixturePlanner::default();
    planner.rows.insert("Q1".into(), vec![cached.clone()]);
    let plan = plan_physical_acquisition(
        &[request("r1", "Q1", ProducerFamily::ClassificationEvidence)],
        &mut planner,
    )
    .unwrap();

    let mut transport = FixtureTransport::default();
    transport.cached_refs.insert(cached.physical_object_ref);
    let (_, receipt) = execute_physical_acquisition(&plan, &mut transport).unwrap();

    assert_eq!(receipt.cache_hits, 1);
    assert_eq!(receipt.cache_misses, 0);
    assert_eq!(receipt.remote_gets, 0);
    assert_eq!(receipt.bytes_fetched, 0);
    assert_eq!(transport.cold_calls, 0);
}

#[test]
fn six_cold_objects_are_scheduled_five_then_one() {
    let mut planner = FixturePlanner::default();
    planner.rows.insert(
        "Q1".into(),
        (0..6)
            .map(|i| object(&format!("{i}"), AcquisitionPath::RouteAwareGeneralSnapshot))
            .collect(),
    );
    let plan = plan_physical_acquisition(
        &[request("r1", "Q1", ProducerFamily::ClassificationEvidence)],
        &mut planner,
    )
    .unwrap();

    let mut transport = FixtureTransport::default();
    let (_, receipt) = execute_physical_acquisition(&plan, &mut transport).unwrap();

    assert_eq!(transport.cold_batch_widths, vec![5, 1]);
    assert_eq!(receipt.remote_gets, 6);
    assert_eq!(receipt.peak_cold_batch_width, 5);
    assert_eq!(receipt.max_cold_remote_objects, 5);
}

#[test]
fn truncation_abstains_and_live_fallback_breaks_snapshot_simultaneity() {
    let a = object("a", AcquisitionPath::RouteAwareGeneralSnapshot);
    let b = object("b", AcquisitionPath::GovernedLiveFallback);
    let mut planner = FixturePlanner::default();
    planner.rows.insert("Q1".into(), vec![a, b.clone()]);
    let plan = plan_physical_acquisition(
        &[request("r1", "Q1", ProducerFamily::ClassificationEvidence)],
        &mut planner,
    )
    .unwrap();

    let mut transport = FixtureTransport::default();
    transport.incomplete_refs.insert(b.physical_object_ref.clone());
    transport.live_refs.insert(b.physical_object_ref);
    let (_, receipt) = execute_physical_acquisition(&plan, &mut transport).unwrap();

    assert_eq!(receipt.truncated_objects, 1);
    assert!(receipt.abstain_required);
    assert_eq!(receipt.live_fallbacks, 1);
    assert!(!receipt.snapshot_simultaneous);
    assert!(!receipt.creates_semantic_authority);
}

struct FamilyExecutor(ProducerFamily);

impl ProducerExecutor for FamilyExecutor {
    fn execute(
        &mut self,
        plan: &ProducerExecutionPlan,
    ) -> Result<CandidateProducerEvidence, Sprint1AcquisitionError> {
        Ok(CandidateProducerEvidence {
            producer: self.0,
            evidence_ref: format!("evidence:{}", plan.move_ref),
            source_revision_ref: format!("revision:{}", plan.target_ref),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            complete: true,
        })
    }
}

#[test]
fn one_controller_executes_classification_identity_and_authority() {
    let mut controller = Sprint1ProducerController::new();
    controller.register(
        ProducerFamily::ClassificationEvidence,
        Box::new(FamilyExecutor(ProducerFamily::ClassificationEvidence)),
    );
    controller.register(
        ProducerFamily::IdentitySource,
        Box::new(FamilyExecutor(ProducerFamily::IdentitySource)),
    );
    controller.register(
        ProducerFamily::AuthoritySource,
        Box::new(FamilyExecutor(ProducerFamily::AuthoritySource)),
    );

    for family in [
        ProducerFamily::ClassificationEvidence,
        ProducerFamily::IdentitySource,
        ProducerFamily::AuthoritySource,
    ] {
        let evidence = controller
            .execute(&ProducerExecutionPlan {
                residual_ref: format!("residual:{family:?}"),
                move_ref: format!("move:{family:?}"),
                producer: family,
                target_ref: "target".into(),
            })
            .unwrap();
        assert_eq!(evidence.producer, family);
        assert!(evidence.candidate_only);
        assert!(!evidence.creates_semantic_authority);
    }
}

fn hop(
    index: usize,
    prior: Option<&str>,
    world_before: char,
    world_after: char,
    producer_ref: &str,
    outcome_ref: &str,
    receipt: char,
) -> GwbHopLedgerRow {
    GwbHopLedgerRow {
        campaign_ref: "campaign:test".into(),
        hop_index: index,
        prior_receipt_sha256: prior.map(str::to_owned),
        world_before_sha256: digest(world_before),
        frontier_sha256: digest(char::from_u32('d' as u32 + index as u32).unwrap()),
        selected_move_ref: format!("move:{index}"),
        investigation_kind_ref: "test".into(),
        producer_ref: producer_ref.into(),
        source_revision_ref: format!("source:{index}"),
        evidence_digest_ref: digest('e'),
        review_ref: format!("review:{index}"),
        outcome_ref: outcome_ref.into(),
        residual_effect_ref: format!("effect:{index}"),
        world_after_sha256: digest(world_after),
        closed_residual_refs: vec![],
        opened_residual_refs: vec![],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "gwb_adaptive_runtime_review_only",
        receipt_sha256: receipt.to_string().repeat(64),
    }
}

#[test]
fn restart_replay_recovers_exact_world_head_and_producer_history() {
    let r0 = hop(
        0,
        None,
        'a',
        'b',
        "producer:wikidata-classification",
        "paid",
        '1',
    );
    let r1 = hop(
        1,
        Some(&r0.receipt_sha256),
        'b',
        'c',
        "producer:wikimedia-identity",
        "rejected-wrong-type",
        '2',
    );
    let r2 = hop(
        2,
        Some(&r1.receipt_sha256),
        'c',
        'f',
        "producer:authority-source",
        "paid",
        '3',
    );

    let head = replay_campaign_head("campaign:test", &[r2.clone(), r0, r1]).unwrap();
    assert_eq!(head.completed_hops, 3);
    assert_eq!(head.world_sha256, Some(r2.world_after_sha256));
    assert_eq!(head.prior_receipt_sha256, Some(r2.receipt_sha256));
    assert_eq!(head.producer_refs.len(), 3);
    assert_eq!(head.rejected_or_blocked_hops, 1);
    assert!(head.candidate_only);
    assert!(!head.creates_semantic_authority);
}

#[test]
fn replay_fails_closed_when_world_chain_is_not_restart_equivalent() {
    let r0 = hop(0, None, 'a', 'b', "producer:a", "paid", '1');
    let r1 = hop(
        1,
        Some(&r0.receipt_sha256),
        'z',
        'c',
        "producer:b",
        "paid",
        '2',
    );
    assert!(matches!(
        replay_campaign_head("campaign:test", &[r0, r1]),
        Err(Sprint1AcquisitionError::ReplayWorldMismatch(1))
    ));
}


#[test]
fn persisted_exact_revision_is_not_snapshot_simultaneous() {
    let persisted = object("pg", AcquisitionPath::PersistedExactRevision);
    let mut planner = FixturePlanner::default();
    planner.rows.insert("Q1".into(), vec![persisted]);
    let plan = plan_physical_acquisition(
        &[request("r1", "Q1", ProducerFamily::IdentitySource)],
        &mut planner,
    )
    .unwrap();

    let mut transport = FixtureTransport::default();
    let (_, receipt) = execute_physical_acquisition(&plan, &mut transport).unwrap();

    assert_eq!(receipt.live_fallbacks, 0);
    assert!(!receipt.snapshot_simultaneous);
}


#[test]
fn selected_route_lowers_without_reinterpreting_producer_family() {
    let candidate = sensiblaw_route_selector::RouteCandidate {
        candidate_id: "candidate:authority".into(),
        producer: ProducerFamily::AuthoritySource,
        route_family: sensiblaw_route_selector::RouteFamily::PrimarySourceSearch,
        source_ref: "source".into(),
        target_ref: "[2026] HCA 19".into(),
        property_ref: String::new(),
        cross_language_gap_coverage: 0,
        source_surface_support: 1,
        root_qid_support: 1,
        typed_property_support: 0,
        route_specificity: 5,
        yield_history_observed: 0,
        prior_contracted_old_gaps: 0,
        prior_retired_obligations: 0,
        prior_new_gap_atoms: 0,
        prior_network_requests: 0,
    };
    let plan = producer_execution_plan_from_candidate("residual:authority", &candidate);
    assert_eq!(plan.move_ref, candidate.candidate_id);
    assert_eq!(plan.producer, ProducerFamily::AuthoritySource);
    assert_eq!(plan.target_ref, "[2026] HCA 19");
}
