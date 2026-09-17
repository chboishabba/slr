mod candidate_pnf;
mod context_federation;
mod discovery_lineage;
mod latent_world;
mod legal_ir_materialization;
mod proposition_rows;
mod reviewed_pnf;

pub use candidate_pnf::{
    CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor, CandidatePnfProducer,
    CandidatePnfRole, ExactSourceSpan, SpacyTsvAdapter,
};
pub use context_federation::{
    materialize_reviewed_context_edges, review_mabo_wikidata_candidate, reviewed_context_edge,
    ContextFederationError, ContextMaterializationReceipt, ContextReviewDecision,
    ReviewedContextEdge, SourceFamily,
};
pub use discovery_lineage::{
    discovery_lineage_row, materialize_discovery_lineage, DiscoveryLineageError,
    DiscoveryLineageInput, DiscoveryLineageMaterializationReceipt, DiscoveryLineageRow,
};
pub use latent_world::{
    load_latent_world_rows, load_latent_world_rows_with_budget, LatentWorldBudget,
    LatentWorldEdgeRow, LatentWorldError, LatentWorldRows,
};
pub use legal_ir_materialization::{
    materialize_reviewed_proposition_support, LegalIrMaterializationError, MaterializedLegalIrRefs,
    ReviewedPropositionSupport,
};
pub use proposition_rows::{
    load_mabo_proposition_rows, load_proposition_rows, PropositionObservationRow, PropositionRows,
};
pub use reviewed_pnf::{
    materialize_reviewed_pnf_revision, validate_reviewed_pnf_revision,
    ReviewedPnfMaterializationError, ReviewedPnfMaterializationReceipt, ReviewedPnfRevision,
    ReviewedPnfValidationError,
};

// Keep the established PostgreSQL source-store implementation byte-for-byte
// while focused reader/materialisation projections remain separate concerns.
include!("storage_core.rs");