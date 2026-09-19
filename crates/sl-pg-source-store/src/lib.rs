mod candidate_pnf;
mod context_federation;
mod discovery_identity_baseline;
mod gwb_ambiguity_state;
mod gwb_campaign_commit;
mod gwb_hop_ledger;
mod discovery_lineage;
mod latent_world;
mod legal_ir_materialization;
mod non_novel_identity_alias;
mod proposition_rows;
mod reviewed_pnf;
mod reviewed_source_expansion;

pub use candidate_pnf::{
    CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor, CandidatePnfProducer,
    CandidatePnfRole, ExactSourceSpan, SpacyTsvAdapter,
};
pub use context_federation::{
    bounded_wikidata_relation_type, mabo_wikidata_property_ref,
    materialize_reviewed_context_edges, review_bounded_wikidata_candidate,
    review_mabo_wikidata_candidate, reviewed_context_edge, ContextFederationError,
    ContextMaterializationReceipt, ContextReviewDecision, ReviewedContextEdge, SourceFamily,
};
pub use gwb_campaign_commit::{
    materialize_gwb_campaign_commit, GwbCampaignCommitError, GwbCampaignCommitInput,
    GwbCampaignCommitReceipt,
};
pub use gwb_ambiguity_state::{
    close_gwb_ambiguity_residuals, gwb_ambiguity_state_row,
    load_open_gwb_ambiguity_residuals, materialize_gwb_ambiguity_residuals,
    GwbAmbiguityMaterializationReceipt, GwbAmbiguityStateError,
    GwbAmbiguityStateInput, GwbAmbiguityStateRow,
};
pub use gwb_hop_ledger::{
    gwb_hop_ledger_row, load_gwb_reviewed_move_refs, load_latest_gwb_hop,
    materialize_gwb_hop,
    GwbHopLedgerError, GwbHopLedgerInput, GwbHopLedgerMaterializationReceipt,
    GwbHopLedgerRow,
};
pub use discovery_identity_baseline::{
    collapse_discovery_campaign_identity_classes, collapse_discovery_identity_baseline,
    load_discovery_campaign_identity_classes, load_discovery_identity_baseline,
    DiscoveryCampaignIdentityRow, DiscoveryIdentityBaseline, DiscoveryIdentityBaselineError,
    DiscoveryIdentityBaselineRow,
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
pub use non_novel_identity_alias::{
    identity_alias_row, materialize_non_novel_identity_aliases, NonNovelIdentityAliasError,
    NonNovelIdentityAliasInput, NonNovelIdentityAliasMaterializationReceipt,
    NonNovelIdentityAliasRow,
};
pub use proposition_rows::{
    load_mabo_proposition_rows, load_proposition_rows, PropositionObservationRow, PropositionRows,
};
pub use reviewed_pnf::{
    materialize_reviewed_pnf_revision, validate_reviewed_pnf_revision,
    ReviewedPnfMaterializationError, ReviewedPnfMaterializationReceipt, ReviewedPnfRevision,
    ReviewedPnfValidationError,
};
pub use reviewed_source_expansion::{
    load_reviewed_context_expansion_sources, materialize_reviewed_context_expansion,
    materialize_reviewed_source_expansions, reviewed_source_expansion_row,
    ReviewedSourceExpansionError, ReviewedSourceExpansionInput,
    ReviewedSourceExpansionMaterializationReceipt, ReviewedSourceExpansionRow,
};

// Keep the established PostgreSQL source-store implementation byte-for-byte
// while focused reader/materialisation projections remain separate concerns.
include!("storage_core.rs");