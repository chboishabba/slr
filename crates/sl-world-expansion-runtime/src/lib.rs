//! Thin production wiring for recurrent world expansion.
//!
//! This crate is intentionally an integration layer: proof-search retains the
//! semantic recurrence, `sl-pg-source-store` retains provider-neutral storage,
//! the consumer residual compiler retains gap/payment semantics, and this crate
//! performs only the reviewed projections between those established surfaces.

pub mod gwb_ambiguity_campaign;
pub mod gwb_review;
pub mod gwb_supervised_type_closure;
pub mod gwb_analysis;
pub mod sprint1_acquisition_machine;
pub mod sprint1_producers;
pub mod sprint2_provider_normalisation;
pub mod zelph_hf_physical;
pub mod mabo_generic_legal_follow;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::Cursor;

use sensiblaw_consumer_residual::{
    ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind, RequirementNeed, RequirementScope,
};
use sensiblaw_pg_source_store::{
    materialize_discovery_lineage, DatabaseConfig, DiscoveryIdentityBaseline,
    DiscoveryLineageInput, LatentWorldRows,
};
use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::world_expansion::{
    ProducerLane, ResidualClass, WorldExpansionPolicy,
};
use sensiblaw_proof_search_loop::world_expansion_reentry::{
    DiscoveryLineageReceipt, PostAcquisitionWorldObservation,
};
use sensiblaw_proof_search_loop::world_expansion_runner::{
    PreparedWorldExpansionCycle, RecurrentRunBlocker, RecurrentRunBlockerKind,
    WorldExpansionCycleSink, WorldExpansionCycleSource,
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
pub struct MaboWikidataSourceRevision {
    pub qid: String,
    pub oldid: u64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaboCampaignPreparationError {
    #[error("unsupported source revision: {0}")]
    UnsupportedSourceRevision(String),
    #[error("invalid QID: {0}")]
    InvalidQid(String),
    #[error("invalid oldid: {0}")]
    InvalidOldId(String),
}

/// Parse only the exact pinned manifestation form used by the reviewed-context
/// persistence lane: `wikidata:<QID>:oldid:<positive revision>`.
pub fn parse_mabo_wikidata_source_revision_ref(
    source_revision_ref: &str,
) -> Result<MaboWikidataSourceRevision, MaboCampaignPreparationError> {
    let fields = source_revision_ref.split(':').collect::<Vec<_>>();
    if fields.len() != 4 || fields[0] != "wikidata" || fields[2] != "oldid" {
        return Err(MaboCampaignPreparationError::UnsupportedSourceRevision(
            source_revision_ref.to_owned(),
        ));
    }
    let qid = fields[1];
    let is_valid_qid = qid
        .strip_prefix('Q')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()));
    if !is_valid_qid {
        return Err(MaboCampaignPreparationError::InvalidQid(qid.to_owned()));
    }
    let oldid = fields[3]
        .parse::<u64>()
        .map_err(|_| MaboCampaignPreparationError::InvalidOldId(fields[3].to_owned()))?;
    if oldid == 0 {
        return Err(MaboCampaignPreparationError::InvalidOldId(fields[3].to_owned()));
    }
    Ok(MaboWikidataSourceRevision {
        qid: qid.to_owned(),
        oldid,
    })
}
