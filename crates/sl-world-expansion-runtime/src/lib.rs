//! Thin production wiring for recurrent world expansion.
//!
//! This crate is intentionally an integration layer: proof-search retains the
//! semantic recurrence, `sl-pg-source-store` retains provider-neutral storage,
//! and this crate performs the one-way projection between their receipt shapes.

use sensiblaw_pg_source_store::{
    materialize_discovery_lineage, DatabaseConfig, DiscoveryLineageInput,
};
use sensiblaw_proof_search_loop::world_expansion::ProducerLane;
use sensiblaw_proof_search_loop::world_expansion_reentry::DiscoveryLineageReceipt;
use sensiblaw_proof_search_loop::world_expansion_runner::{
    RecurrentRunBlocker, WorldExpansionCycleSink,
};
use sensiblaw_proof_search_loop::world_expansion_session::WorldExpansionCycleReceipt;
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorldExpansionRuntimeError {
    #[error("reviewed identity class is required for durable recurrent lineage")]
    MissingIdentityClass,
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
