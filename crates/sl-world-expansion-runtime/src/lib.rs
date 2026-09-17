//! Thin production wiring for recurrent world expansion.
//!
//! This crate is intentionally an integration layer: proof-search retains the
//! semantic recurrence, `sl-pg-source-store` retains provider-neutral storage,
//! the consumer residual compiler retains gap/payment semantics, and this crate
//! performs only the reviewed projections between those established surfaces.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Cursor;

use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_pg_source_store::{
    materialize_discovery_lineage, DatabaseConfig, DiscoveryIdentityBaseline,
    DiscoveryLineageInput, LatentWorldRows,
};
use sensiblaw_proof_search_loop::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::{
    ProducerLane, ResidualClass, WorldExpansionPolicy,
};
use sensiblaw_proof_search_loop::world_expansion_reentry::{
    DiscoveryLineageReceipt, PostAcquisitionWorldObservation,
};
use sensiblaw_proof_search_loop::world_expansion_runner::{
    RecurrentRunBlocker, RecurrentRunBlockerKind, WorldExpansionCycleSink,
};
use sensiblaw_proof_search_loop::world_expansion_session::{
    WorldExpansionCycleReceipt, WorldExpansionSession,
};
use sensiblaw_proof_search_loop::world_identity_guard::IdentityCoherenceBaseline;
use sensiblaw_proof_search_loop::world_known_identity_payment::{
    reenter_known_identity_payment, KnownIdentityPaymentError, KnownIdentityPaymentReceipt,
};
use sensiblaw_proof_search_loop::world_observation::WorldObservation;
use sensiblaw_reviewed_evidence_payment::{
    compile_reviewed_evidence_payment, ReviewedEvidenceCoordinate,
};
use sensiblaw_world_store::WorldStore;
use thiserror::Error;

pub const MABO_NOVEL_IDENTITY_TARGET: usize = 100;
pub const MABO_CONTEXT_IDENTITY_CONSUMER: &str = "consumer:mabo-context-world-identity";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboIdentityDiagnosisRow {
    pub representation_ref: String,
    pub relation_type_refs: Vec<String>,
    pub source_revision_refs: Vec<String>,
    pub requirement_id: String,
    pub residual_ref: String,
    pub residual_class: ResidualClass,
    /// Mirrors DASHI's plural-lens discovery vocabulary. This is a discovery
    /// route, never an evidence-payment or identity-review claim.
    pub discovery_route_ref: &'static str,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaboConsumerDiagnosis {
    pub consumer_spec: ConsumerSpec,
    pub residuals: Vec<ProofResidual>,
    pub rows: Vec<MaboIdentityDiagnosisRow>,
    pub reviewed_context_edges_considered: usize,
    pub known_identity_representations: usize,
    pub duplicate_target_edges: usize,
    pub out_of_scope_or_wrong_type_edges: usize,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Default)]
struct PendingIdentityDiagnosis {
    relation_type_refs: BTreeSet<String>,
    source_revision_refs: BTreeSet<String>,
}

fn reviewed_wikidata_revision(provenance_refs: &[String]) -> Option<String> {
    provenance_refs.iter().find_map(|provenance| {
        provenance
            .strip_prefix("context:wikidata:")
            .map(ToOwned::to_owned)
    })
}

/// Compile the persisted latent-world surface into an explicit consumer-indexed
/// identity-review frontier.
///
/// Only reviewed Wikidata context edges are admissible to this first consumer.
/// A reviewed relation may *propose* a SameObject requirement for its target;
/// it never pays that requirement. Targets already represented in the durable
/// identity baseline are quotiented before requirements are emitted.
#[must_use]
pub fn diagnose_mabo_context_world_identity(
    world: &LatentWorldRows,
    baseline: &DiscoveryIdentityBaseline,
) -> MaboConsumerDiagnosis {
    let mut pending: BTreeMap<String, PendingIdentityDiagnosis> = BTreeMap::new();
    let mut reviewed_context_edges_considered = 0usize;
    let mut known_identity_representations = 0usize;
    let mut duplicate_target_edges = 0usize;
    let mut out_of_scope_or_wrong_type_edges = 0usize;
    let mut known_seen = BTreeSet::new();

    for edge in &world.edges {
        if !edge.relation_ref.starts_with("context:wikidata:") {
            out_of_scope_or_wrong_type_edges += 1;
            continue;
        }
        let Some(source_revision_ref) = reviewed_wikidata_revision(&edge.provenance_refs) else {
            // A relation type that merely looks like reviewed Wikidata context
            // is not enough. The reviewed source-revision receipt is required.
            out_of_scope_or_wrong_type_edges += 1;
            continue;
        };
        reviewed_context_edges_considered += 1;

        if baseline
            .representation_identity_class_refs
            .contains_key(&edge.to_ref)
        {
            if known_seen.insert(edge.to_ref.clone()) {
                known_identity_representations += 1;
            }
            continue;
        }

        let entry = pending.entry(edge.to_ref.clone()).or_default();
        if !entry.relation_type_refs.is_empty() {
            duplicate_target_edges += 1;
        }
        entry.relation_type_refs.insert(edge.relation_ref.clone());
        entry.source_revision_refs.insert(source_revision_ref);
    }

    let mut requirements = Vec::with_capacity(pending.len());
    let mut residuals = Vec::with_capacity(pending.len());
    let mut rows = Vec::with_capacity(pending.len());

    for (representation_ref, pending) in pending {
        let requirement_id = format!("world-identity:{representation_ref}");
        let residual_ref = format!("residual:mabo:world-identity:{representation_ref}");
        let source_revision_refs = pending.source_revision_refs.into_iter().collect::<Vec<_>>();
        let scope = source_revision_refs
            .first()
            .cloned()
            .map(RequirementScope::SourceManifestation)
            .unwrap_or(RequirementScope::AnySource);

        requirements.push(ConsumerRequirement {
            requirement_id: requirement_id.clone(),
            need: RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::SameObject),
            scope,
        });
        residuals.push(ProofResidual {
            residual_ref: residual_ref.clone(),
            proposition_ref: format!("mabo:world-identity:{representation_ref}"),
            producer_class_ref: "producer:world-expansion".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: None,
            salience: 100,
            dependency_refs: vec![],
            status: ResidualStatus::Open,
        });
        rows.push(MaboIdentityDiagnosisRow {
            representation_ref,
            relation_type_refs: pending.relation_type_refs.into_iter().collect(),
            source_revision_refs,
            requirement_id,
            residual_ref,
            residual_class: ResidualClass::Identity,
            discovery_route_ref: "residual-observation",
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        });
    }

    MaboConsumerDiagnosis {
        consumer_spec: ConsumerSpec {
            consumer_id: MABO_CONTEXT_IDENTITY_CONSUMER.into(),
            surface_id: "surface:mabo:reviewed-context-world".into(),
            requirements,
        },
        residuals,
        rows,
        reviewed_context_edges_considered,
        known_identity_representations,
        duplicate_target_edges,
        out_of_scope_or_wrong_type_edges,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorldExpansionRuntimeError {
    #[error("reviewed identity class is required for durable recurrent lineage")]
    MissingIdentityClass,
    #[error("world observation is invalid or promoting")]
    InvalidWorldObservation,
    #[error("consumer requirement is not present")]
    RequirementNotFound,
    #[error("explicit reviewed evidence coordinate does not match consumer requirement")]
    EvidenceCoordinateMismatch,
    #[error("world observation revision does not match consumer requirement scope")]
    EvidenceScopeMismatch,
    #[error("reviewed evidence payment failed: {0}")]
    ReviewedEvidencePayment(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownIdentityPaymentRuntimeError {
    Semantic(KnownIdentityPaymentError),
    Persistence(RecurrentRunBlocker),
}

impl From<KnownIdentityPaymentError> for KnownIdentityPaymentRuntimeError {
    fn from(value: KnownIdentityPaymentError) -> Self {
        Self::Semantic(value)
    }
}

#[must_use]
pub fn identity_coherence_baseline(
    baseline: &DiscoveryIdentityBaseline,
) -> IdentityCoherenceBaseline {
    IdentityCoherenceBaseline {
        identity_class_refs: baseline.identity_class_refs.clone(),
        representation_identity_class_refs: baseline.representation_identity_class_refs.clone(),
    }
}

#[must_use]
pub fn mabo_remaining_world_expansion_policy(
    baseline: &DiscoveryIdentityBaseline,
) -> WorldExpansionPolicy {
    WorldExpansionPolicy {
        target_novel_objects: MABO_NOVEL_IDENTITY_TARGET
            .saturating_sub(baseline.identity_class_refs.len()),
        minimum_expected_residual_contraction: 1,
    }
}

#[must_use]
pub fn durable_mabo_campaign_total(
    baseline: &DiscoveryIdentityBaseline,
    newly_committed_identity_classes: usize,
) -> usize {
    baseline
        .identity_class_refs
        .len()
        .saturating_add(newly_committed_identity_classes)
}

const fn producer_lane_ref(lane: ProducerLane) -> &'static str {
    match lane {
        ProducerLane::GovernedLegal => "governed-legal",
        ProducerLane::WikidataIdentity => "wikidata-identity",
        ProducerLane::WikipediaContext => "wikipedia-context",
        ProducerLane::SourceSpecificProvenance => "source-specific-provenance",
        ProducerLane::Other => "other",
    }
}

pub fn discovery_lineage_input(
    lineage: &DiscoveryLineageReceipt,
    identity_class_ref: &str,
) -> Result<DiscoveryLineageInput, WorldExpansionRuntimeError> {
    if identity_class_ref.trim().is_empty() {
        return Err(WorldExpansionRuntimeError::MissingIdentityClass);
    }
    Ok(DiscoveryLineageInput {
        object_ref: lineage.object_ref.clone(),
        identity_class_ref: identity_class_ref.to_owned(),
        discovery_parent_ref: lineage.discovery_parent_ref.clone(),
        triggering_residual_ref: lineage.triggering_residual_ref.clone(),
        selected_candidate_ref: lineage.selected_candidate_ref.clone(),
        producer_lane_ref: producer_lane_ref(lineage.producer_lane).into(),
        source_revision_ref: lineage.source_revision_ref.clone(),
        pnf_world_disambiguation_ref: lineage.pnf_world_disambiguation_ref.clone(),
        expected_residual_contraction: lineage.expected_residual_contraction,
        observed_residual_contraction: lineage.observed_residual_contraction,
        new_residual_refs: lineage.new_residual_refs.clone(),
        candidate_only: lineage.candidate_only,
        creates_semantic_authority: lineage.creates_semantic_authority,
        applicability_promoted: lineage.applicability_promoted,
        claim_truth_promoted: lineage.claim_truth_promoted,
        receipt_authority: lineage.receipt_authority.into(),
    })
}

pub fn reviewed_evidence_from_world_observation(
    spec: &ConsumerSpec,
    requirement_id: &str,
    coordinate: EvidenceCoordinateKind,
    review_ref: &str,
    observation: &WorldObservation,
) -> Result<ReviewedEvidenceCoordinate, WorldExpansionRuntimeError> {
    observation
        .validate()
        .map_err(|_| WorldExpansionRuntimeError::InvalidWorldObservation)?;
    let requirement = spec
        .requirements
        .iter()
        .find(|requirement| requirement.requirement_id == requirement_id)
        .ok_or(WorldExpansionRuntimeError::RequirementNotFound)?;
    match requirement.need {
        RequirementNeed::EvidenceCoordinate(required) if required == coordinate => {}
        _ => return Err(WorldExpansionRuntimeError::EvidenceCoordinateMismatch),
    }
    match &requirement.scope {
        RequirementScope::AnySource => {}
        RequirementScope::SourceManifestation(expected)
            if expected == &observation.source_revision_ref => {}
        RequirementScope::SourceManifestation(_) => {
            return Err(WorldExpansionRuntimeError::EvidenceScopeMismatch);
        }
    }

    Ok(ReviewedEvidenceCoordinate {
        review_ref: review_ref.to_owned(),
        consumer_id: spec.consumer_id.clone(),
        requirement_id: requirement_id.to_owned(),
        coordinate,
        source_ref: Some(observation.source_revision_ref.clone()),
        evidence_ref: observation.request_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn reviewed_observation_payment_stream(
    spec: &ConsumerSpec,
    requirement_id: &str,
    coordinate: EvidenceCoordinateKind,
    review_ref: &str,
    observation: &WorldObservation,
    iteration_index: i64,
) -> Result<Vec<u8>, WorldExpansionRuntimeError> {
    let reviewed = reviewed_evidence_from_world_observation(
        spec,
        requirement_id,
        coordinate,
        review_ref,
        observation,
    )?;
    let mut bytes = Vec::new();
    compile_reviewed_evidence_payment(spec, &reviewed, &mut bytes, iteration_index)
        .map_err(|error| WorldExpansionRuntimeError::ReviewedEvidencePayment(error.to_string()))?;
    Ok(bytes)
}

pub trait KnownIdentityPaymentSink {
    fn persist_reviewed_payment(
        &mut self,
        wire: &[u8],
    ) -> Result<(), RecurrentRunBlocker>;
}

pub fn apply_known_identity_payment_transaction<K>(
    session: &mut WorldExpansionSession,
    next_frontier_ref: impl Into<String>,
    identity_class_ref: &str,
    observation: &PostAcquisitionWorldObservation,
    reviewed_payment_wire: &[u8],
    sink: &mut K,
) -> Result<KnownIdentityPaymentReceipt, KnownIdentityPaymentRuntimeError>
where
    K: KnownIdentityPaymentSink,
{
    let receipt = reenter_known_identity_payment(
        session,
        next_frontier_ref,
        identity_class_ref,
        observation,
    )?;
    sink.persist_reviewed_payment(reviewed_payment_wire)
        .map_err(KnownIdentityPaymentRuntimeError::Persistence)?;
    session.frontier = receipt.next_frontier.clone();
    Ok(receipt)
}

pub struct WorldStoreReviewedPaymentSink {
    store: WorldStore,
}

impl WorldStoreReviewedPaymentSink {
    #[must_use]
    pub fn new(store: WorldStore) -> Self {
        Self { store }
    }
}

impl KnownIdentityPaymentSink for WorldStoreReviewedPaymentSink {
    fn persist_reviewed_payment(
        &mut self,
        wire: &[u8],
    ) -> Result<(), RecurrentRunBlocker> {
        self.store
            .ingest_wire(Cursor::new(wire))
            .map_err(|error| {
                RecurrentRunBlocker::new(
                    RecurrentRunBlockerKind::PersistenceBlocked,
                    format!("world-store:reviewed-payment:{error}"),
                )
            })?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PgDiscoveryLineageSink {
    config: DatabaseConfig,
}

impl PgDiscoveryLineageSink {
    #[must_use]
    pub fn new(config: DatabaseConfig) -> Self {
        Self { config }
    }
}

impl WorldExpansionCycleSink for PgDiscoveryLineageSink {
    fn persist_cycle(
        &mut self,
        receipt: &WorldExpansionCycleReceipt,
    ) -> Result<(), RecurrentRunBlocker> {
        let input = discovery_lineage_input(
            &receipt.reentry.lineage,
            &receipt.step.admission.identity_class_ref,
        )
        .map_err(|error| {
            RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::PersistenceBlocked,
                format!("lineage-projection:{error}"),
            )
        })?;
        materialize_discovery_lineage(&self.config, &[input]).map_err(|error| {
            RecurrentRunBlocker::new(
                RecurrentRunBlockerKind::PersistenceBlocked,
                format!("pg:discovery-lineage:{error}"),
            )
        })?;
        Ok(())
    }
}
