//! Thin production wiring for recurrent world expansion.
//!
//! This crate is intentionally an integration layer: proof-search retains the
//! semantic recurrence, `sl-pg-source-store` retains provider-neutral storage,
//! the consumer residual compiler retains gap/payment semantics, and this crate
//! performs only the reviewed projections between those established surfaces.

use sensiblaw_consumer_residual::{
    ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_pg_source_store::{
    materialize_discovery_lineage, DatabaseConfig, DiscoveryLineageInput,
};
use sensiblaw_proof_search_loop::world_expansion::ProducerLane;
use sensiblaw_proof_search_loop::world_expansion_reentry::DiscoveryLineageReceipt;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    RecurrentRunBlocker, WorldExpansionCycleSink,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionCycleReceipt;
use sensiblaw_proof_search_loop::world_observation::WorldObservation;
use sensiblaw_reviewed_evidence_payment::{
    compile_reviewed_evidence_payment, ReviewedEvidenceCoordinate,
};
use thiserror::Error;

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
        .map_err(|error| RecurrentRunBlocker::new(format!("lineage-projection:{error}")))?;
        materialize_discovery_lineage(&self.config, &[input])
            .map_err(|error| RecurrentRunBlocker::new(format!("pg:discovery-lineage:{error}")))?;
        Ok(())
    }
}
