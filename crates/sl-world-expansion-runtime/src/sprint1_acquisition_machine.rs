//! Sprint 1 production acquisition-machine contracts.
//!
//! Semantic requests are resolved to physical objects before scheduling.
//! Physical-object dedupe/coalescing therefore happens before the five-wide
//! cold-remote bound. Transport/controller receipts remain candidate-only;
//! truncation requires abstention and semantic payment remains review-owned.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_pg_source_store::{load_gwb_hops, DatabaseConfig, GwbHopLedgerRow};
use sensiblaw_route_selector::{ProducerFamily, RouteCandidate};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const MAX_COLD_REMOTE_PHYSICAL_OBJECTS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AcquisitionPath {
    SpecializedP31P279Slice,
    RouteAwareGeneralSnapshot,
    PersistedExactRevision,
    GovernedLiveFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalAcquisitionRequest {
    pub request_ref: String,
    pub semantic_target_ref: String,
    pub producer: ProducerFamily,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlannedPhysicalObject {
    pub physical_object_ref: String,
    pub cache_key: String,
    pub route_ref: String,
    pub source_ref: String,
    pub source_range: Option<(u64, u64)>,
    pub expected_bytes: Option<u64>,
    pub acquisition_path: AcquisitionPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalPhysicalBinding {
    pub request_ref: String,
    pub semantic_target_ref: String,
    pub resolved_node_ref: String,
    pub physical_object_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPhysicalObjects {
    pub resolved_node_ref: String,
    pub objects: Vec<PlannedPhysicalObject>,
}

pub trait LogicalToPhysicalPlanner {
    fn resolve(
        &mut self,
        request: &LogicalAcquisitionRequest,
    ) -> Result<ResolvedPhysicalObjects, Sprint1AcquisitionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalAcquisitionPlan {
    pub logical_requests: Vec<LogicalAcquisitionRequest>,
    pub bindings: Vec<LogicalPhysicalBinding>,
    pub unique_objects: Vec<PlannedPhysicalObject>,
    pub planned_object_references: usize,
    pub coalesced_object_references: usize,
    pub specialised_slice_objects: usize,
    pub route_aware_general_objects: usize,
    pub max_cold_remote_objects: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalObjectFetch {
    pub physical_object_ref: String,
    pub bytes: usize,
    pub from_cache: bool,
    pub live_fallback: bool,
    pub complete: bool,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
}

pub trait BoundedPhysicalTransport {
    fn cached(
        &mut self,
        object: &PlannedPhysicalObject,
    ) -> Result<Option<PhysicalObjectFetch>, Sprint1AcquisitionError>;

    fn fetch_cold_batch(
        &mut self,
        objects: &[PlannedPhysicalObject],
    ) -> Result<Vec<PhysicalObjectFetch>, Sprint1AcquisitionError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalTransportReceipt {
    pub logical_requests: usize,
    pub resolved_nodes: usize,
    pub planned_object_references: usize,
    pub unique_objects: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub coalesced_gets: usize,
    pub remote_gets: usize,
    pub bytes_fetched: usize,
    pub live_fallbacks: usize,
    pub peak_cold_batch_width: usize,
    pub max_cold_remote_objects: usize,
    pub truncated_objects: usize,
    pub abstain_required: bool,
    pub snapshot_simultaneous: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub receipt_sha256: String,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Sprint1AcquisitionError {
    #[error("empty acquisition coordinate: {0}")]
    EmptyCoordinate(&'static str),
    #[error("logical request resolved to no physical objects: {0}")]
    UnresolvedLogicalRequest(String),
    #[error("planner returned an empty physical object coordinate")]
    EmptyPhysicalObject,
    #[error("cold transport batch exceeded five physical objects: {0}")]
    ColdBatchTooWide(usize),
    #[error("transport returned duplicate physical object: {0}")]
    DuplicateTransportObject(String),
    #[error("transport did not return planned physical object: {0}")]
    MissingTransportObject(String),
    #[error("transport returned unplanned physical object: {0}")]
    UnplannedTransportObject(String),
    #[error("producer family has no registered executor: {0}")]
    MissingProducerExecutor(&'static str),
    #[error("producer executor returned wrong family")]
    ProducerFamilyMismatch,
    #[error("producer evidence must remain candidate-only and non-promoting")]
    PromotingProducerEvidence,
    #[error("hop ledger campaign mismatch")]
    ReplayCampaignMismatch,
    #[error("hop ledger is not consecutive: expected {expected}, observed {observed}")]
    ReplayNonConsecutive { expected: usize, observed: usize },
    #[error("hop ledger prior receipt mismatch at hop {0}")]
    ReplayPriorReceiptMismatch(usize),
    #[error("hop ledger world chain mismatch at hop {0}")]
    ReplayWorldMismatch(usize),
    #[error("hop ledger contains promoting receipt at hop {0}")]
    ReplayPromotingReceipt(usize),
    #[error("provider failure: {0}")]
    Provider(String),
}

fn producer_family_ref(family: ProducerFamily) -> &'static str {
    match family {
        ProducerFamily::ArticleSemantic => "article-semantic",
        ProducerFamily::RevisionTemporal => "revision-temporal",
        ProducerFamily::ParserRepair => "parser-repair",
        ProducerFamily::IdentitySource => "identity-source",
        ProducerFamily::AuthoritySource => "authority-source",
        ProducerFamily::MechanismEvidence => "mechanism-evidence",
        ProducerFamily::MeasurementEvidence => "measurement-evidence",
        ProducerFamily::ComparatorEvidence => "comparator-evidence",
        ProducerFamily::ClassificationEvidence => "classification-evidence",
    }
}

pub fn plan_physical_acquisition<P: LogicalToPhysicalPlanner>(
    requests: &[LogicalAcquisitionRequest],
    planner: &mut P,
) -> Result<PhysicalAcquisitionPlan, Sprint1AcquisitionError> {
    let mut logical_requests = requests.to_vec();
    logical_requests.sort_by(|a, b| a.request_ref.cmp(&b.request_ref));
    logical_requests.dedup_by(|a, b| a.request_ref == b.request_ref);

    let mut bindings = Vec::new();
    let mut unique = BTreeMap::<String, PlannedPhysicalObject>::new();
    let mut planned_object_references = 0usize;

    for request in &logical_requests {
        if request.request_ref.trim().is_empty() {
            return Err(Sprint1AcquisitionError::EmptyCoordinate("request_ref"));
        }
        if request.semantic_target_ref.trim().is_empty() {
            return Err(Sprint1AcquisitionError::EmptyCoordinate("semantic_target_ref"));
        }
        let resolved = planner.resolve(request)?;
        if resolved.resolved_node_ref.trim().is_empty() {
            return Err(Sprint1AcquisitionError::EmptyCoordinate("resolved_node_ref"));
        }
        if resolved.objects.is_empty() {
            return Err(Sprint1AcquisitionError::UnresolvedLogicalRequest(
                request.request_ref.clone(),
            ));
        }

        for object in resolved.objects {
            if object.physical_object_ref.trim().is_empty()
                || object.cache_key.trim().is_empty()
                || object.route_ref.trim().is_empty()
            {
                return Err(Sprint1AcquisitionError::EmptyPhysicalObject);
            }
            planned_object_references = planned_object_references.saturating_add(1);
            bindings.push(LogicalPhysicalBinding {
                request_ref: request.request_ref.clone(),
                semantic_target_ref: request.semantic_target_ref.clone(),
                resolved_node_ref: resolved.resolved_node_ref.clone(),
                physical_object_ref: object.physical_object_ref.clone(),
            });
            unique
                .entry(object.physical_object_ref.clone())
                .or_insert(object);
        }
    }

    bindings.sort_by(|a, b| {
        (&a.request_ref, &a.physical_object_ref).cmp(&(&b.request_ref, &b.physical_object_ref))
    });
    let unique_objects = unique.into_values().collect::<Vec<_>>();
    let specialised_slice_objects = unique_objects
        .iter()
        .filter(|o| o.acquisition_path == AcquisitionPath::SpecializedP31P279Slice)
        .count();
    let route_aware_general_objects = unique_objects
        .iter()
        .filter(|o| o.acquisition_path == AcquisitionPath::RouteAwareGeneralSnapshot)
        .count();

    Ok(PhysicalAcquisitionPlan {
        logical_requests,
        bindings,
        coalesced_object_references: planned_object_references
            .saturating_sub(unique_objects.len()),
        planned_object_references,
        unique_objects,
        specialised_slice_objects,
        route_aware_general_objects,
        max_cold_remote_objects: MAX_COLD_REMOTE_PHYSICAL_OBJECTS,
    })
}

fn transport_digest(
    plan: &PhysicalAcquisitionPlan,
    fetches: &[PhysicalObjectFetch],
    peak: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"slr-sprint1-physical-transport:v1\0");
    for request in &plan.logical_requests {
        for value in [request.request_ref.as_str(), request.semantic_target_ref.as_str()] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        hasher.update([request.producer as u8]);
    }
    for object in &plan.unique_objects {
        for value in [
            object.physical_object_ref.as_str(),
            object.cache_key.as_str(),
            object.route_ref.as_str(),
            object.source_ref.as_str(),
        ] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        if let Some((start, end)) = object.source_range {
            hasher.update([1]);
            hasher.update(start.to_be_bytes());
            hasher.update(end.to_be_bytes());
        } else {
            hasher.update([0]);
        }
        if let Some(bytes) = object.expected_bytes {
            hasher.update([1]);
            hasher.update(bytes.to_be_bytes());
        } else {
            hasher.update([0]);
        }
    }
    for fetch in fetches {
        for value in [
            fetch.physical_object_ref.as_str(),
            fetch.source_revision_ref.as_str(),
            fetch.evidence_digest_ref.as_str(),
        ] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        hasher.update((fetch.bytes as u64).to_be_bytes());
        hasher.update([
            fetch.from_cache as u8,
            fetch.live_fallback as u8,
            fetch.complete as u8,
        ]);
    }
    hasher.update((peak as u64).to_be_bytes());
    format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn execute_physical_acquisition<T: BoundedPhysicalTransport>(
    plan: &PhysicalAcquisitionPlan,
    transport: &mut T,
) -> Result<(Vec<PhysicalObjectFetch>, PhysicalTransportReceipt), Sprint1AcquisitionError> {
    let mut acquired = BTreeMap::<String, PhysicalObjectFetch>::new();
    let mut cold = Vec::new();
    let mut cache_hits = 0usize;

    for object in &plan.unique_objects {
        match transport.cached(object)? {
            Some(fetch) => {
                if fetch.physical_object_ref != object.physical_object_ref {
                    return Err(Sprint1AcquisitionError::UnplannedTransportObject(
                        fetch.physical_object_ref,
                    ));
                }
                if acquired
                    .insert(fetch.physical_object_ref.clone(), fetch)
                    .is_some()
                {
                    return Err(Sprint1AcquisitionError::DuplicateTransportObject(
                        object.physical_object_ref.clone(),
                    ));
                }
                cache_hits = cache_hits.saturating_add(1);
            }
            None => cold.push(object.clone()),
        }
    }

    let mut peak = 0usize;
    for batch in cold.chunks(MAX_COLD_REMOTE_PHYSICAL_OBJECTS) {
        if batch.len() > MAX_COLD_REMOTE_PHYSICAL_OBJECTS {
            return Err(Sprint1AcquisitionError::ColdBatchTooWide(batch.len()));
        }
        peak = peak.max(batch.len());
        let expected = batch
            .iter()
            .map(|o| o.physical_object_ref.as_str())
            .collect::<BTreeSet<_>>();
        let fetched = transport.fetch_cold_batch(batch)?;
        let mut returned = BTreeSet::new();

        for fetch in fetched {
            if !expected.contains(fetch.physical_object_ref.as_str()) {
                return Err(Sprint1AcquisitionError::UnplannedTransportObject(
                    fetch.physical_object_ref,
                ));
            }
            let physical_object_ref = fetch.physical_object_ref.clone();
            if !returned.insert(physical_object_ref.clone())
                || acquired.insert(physical_object_ref.clone(), fetch).is_some()
            {
                return Err(Sprint1AcquisitionError::DuplicateTransportObject(
                    physical_object_ref,
                ));
            }
        }
        for object in batch {
            if !returned.contains(&object.physical_object_ref) {
                return Err(Sprint1AcquisitionError::MissingTransportObject(
                    object.physical_object_ref.clone(),
                ));
            }
        }
    }

    let fetches = acquired.into_values().collect::<Vec<_>>();
    let remote_gets = fetches.iter().filter(|f| !f.from_cache).count();
    let bytes_fetched = fetches
        .iter()
        .filter(|f| !f.from_cache)
        .map(|f| f.bytes)
        .sum();
    let live_fallbacks = fetches.iter().filter(|f| f.live_fallback).count();
    let truncated_objects = fetches.iter().filter(|f| !f.complete).count();

    let receipt = PhysicalTransportReceipt {
        logical_requests: plan.logical_requests.len(),
        resolved_nodes: plan
            .bindings
            .iter()
            .map(|b| b.resolved_node_ref.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        planned_object_references: plan.planned_object_references,
        unique_objects: plan.unique_objects.len(),
        cache_hits,
        cache_misses: plan.unique_objects.len().saturating_sub(cache_hits),
        coalesced_gets: plan.coalesced_object_references,
        remote_gets,
        bytes_fetched,
        live_fallbacks,
        peak_cold_batch_width: peak,
        max_cold_remote_objects: MAX_COLD_REMOTE_PHYSICAL_OBJECTS,
        truncated_objects,
        abstain_required: truncated_objects > 0,
        snapshot_simultaneous: live_fallbacks == 0
            && plan.unique_objects.iter().all(|object| {
                matches!(
                    object.acquisition_path,
                    AcquisitionPath::SpecializedP31P279Slice
                        | AcquisitionPath::RouteAwareGeneralSnapshot
                )
            }),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_sha256: transport_digest(plan, &fetches, peak),
    };
    Ok((fetches, receipt))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProducerExecutionPlan {
    pub residual_ref: String,
    pub move_ref: String,
    pub producer: ProducerFamily,
    pub target_ref: String,
}

/// Lower an actually selected route candidate into the shared Sprint 1
/// producer-execution ABI.  Route selection remains the owner of producer
/// choice; the controller does not re-rank or reinterpret it.
#[must_use]
pub fn producer_execution_plan_from_candidate(
    residual_ref: impl Into<String>,
    candidate: &RouteCandidate,
) -> ProducerExecutionPlan {
    ProducerExecutionPlan {
        residual_ref: residual_ref.into(),
        move_ref: candidate.candidate_id.clone(),
        producer: candidate.producer,
        target_ref: candidate.target_ref.clone(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateProducerEvidence {
    pub producer: ProducerFamily,
    pub evidence_ref: String,
    pub source_revision_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub complete: bool,
}

pub trait ProducerExecutor {
    fn execute(
        &mut self,
        plan: &ProducerExecutionPlan,
    ) -> Result<CandidateProducerEvidence, Sprint1AcquisitionError>;
}

#[derive(Default)]
pub struct Sprint1ProducerController {
    executors: BTreeMap<&'static str, Box<dyn ProducerExecutor>>,
}

impl Sprint1ProducerController {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, family: ProducerFamily, executor: Box<dyn ProducerExecutor>) {
        self.executors.insert(producer_family_ref(family), executor);
    }

    pub fn execute(
        &mut self,
        plan: &ProducerExecutionPlan,
    ) -> Result<CandidateProducerEvidence, Sprint1AcquisitionError> {
        let key = producer_family_ref(plan.producer);
        let executor = self
            .executors
            .get_mut(key)
            .ok_or(Sprint1AcquisitionError::MissingProducerExecutor(key))?;
        let evidence = executor.execute(plan)?;
        if evidence.producer != plan.producer {
            return Err(Sprint1AcquisitionError::ProducerFamilyMismatch);
        }
        if !evidence.candidate_only
            || evidence.creates_semantic_authority
            || evidence.applicability_promoted
            || evidence.claim_truth_promoted
        {
            return Err(Sprint1AcquisitionError::PromotingProducerEvidence);
        }
        Ok(evidence)
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayedCampaignHead {
    pub campaign_ref: String,
    pub completed_hops: usize,
    pub prior_receipt_sha256: Option<String>,
    pub world_sha256: Option<String>,
    pub last_frontier_sha256: Option<String>,
    pub producer_refs: BTreeSet<String>,
    pub rejected_or_blocked_hops: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

/// Reconstruct the production-relevant restart head from the durable hop
/// ledger. Hop N+1 must begin from hop N's reviewed world-after digest.
pub fn replay_campaign_head(
    campaign_ref: &str,
    rows: &[GwbHopLedgerRow],
) -> Result<ReplayedCampaignHead, Sprint1AcquisitionError> {
    if campaign_ref.trim().is_empty() {
        return Err(Sprint1AcquisitionError::EmptyCoordinate("campaign_ref"));
    }
    let mut rows = rows.to_vec();
    rows.sort_by_key(|row| row.hop_index);

    let mut prior_receipt: Option<String> = None;
    let mut world: Option<String> = None;
    let mut producer_refs = BTreeSet::new();
    let mut rejected_or_blocked_hops = 0usize;

    for (expected, row) in rows.iter().enumerate() {
        if row.campaign_ref != campaign_ref {
            return Err(Sprint1AcquisitionError::ReplayCampaignMismatch);
        }
        if row.hop_index != expected {
            return Err(Sprint1AcquisitionError::ReplayNonConsecutive {
                expected,
                observed: row.hop_index,
            });
        }
        if row.prior_receipt_sha256 != prior_receipt {
            return Err(Sprint1AcquisitionError::ReplayPriorReceiptMismatch(
                row.hop_index,
            ));
        }
        if let Some(previous_world) = world.as_deref() {
            if row.world_before_sha256 != previous_world {
                return Err(Sprint1AcquisitionError::ReplayWorldMismatch(row.hop_index));
            }
        }
        if !row.candidate_only
            || row.creates_semantic_authority
            || row.applicability_promoted
            || row.claim_truth_promoted
        {
            return Err(Sprint1AcquisitionError::ReplayPromotingReceipt(
                row.hop_index,
            ));
        }
        if matches!(
            row.outcome_ref.as_str(),
            "blocked" | "rejected" | "rejected-wrong-type"
        ) {
            rejected_or_blocked_hops = rejected_or_blocked_hops.saturating_add(1);
        }
        producer_refs.insert(row.producer_ref.clone());
        prior_receipt = Some(row.receipt_sha256.clone());
        world = Some(row.world_after_sha256.clone());
    }

    Ok(ReplayedCampaignHead {
        campaign_ref: campaign_ref.to_owned(),
        completed_hops: rows.len(),
        prior_receipt_sha256: prior_receipt,
        world_sha256: world,
        last_frontier_sha256: rows.last().map(|row| row.frontier_sha256.clone()),
        producer_refs,
        rejected_or_blocked_hops,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}


/// Load the persisted PostgreSQL campaign ledger and reconstruct the exact
/// production restart head through the same validator used for offline replay.
pub fn load_and_replay_campaign_head(
    config: &DatabaseConfig,
    campaign_ref: &str,
) -> Result<ReplayedCampaignHead, Sprint1AcquisitionError> {
    let rows = load_gwb_hops(config, campaign_ref)
        .map_err(|error| Sprint1AcquisitionError::Provider(format!("pg-hop-ledger:{error}")))?;
    replay_campaign_head(campaign_ref, &rows)
}
