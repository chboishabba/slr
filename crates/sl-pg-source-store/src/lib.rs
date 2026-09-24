mod workbench_projection;
pub use workbench_projection::*;
mod cache_first;
mod chronology_contestation_store;
mod candidate_pnf;
mod context_federation;
mod discovery_identity_baseline;
mod gwb_ambiguity_state;
mod gwb_campaign_commit;
mod gwb_chronology_capstone;
mod gwb_hop_ledger;
mod discovery_lineage;
mod latent_world;
mod legal_ir_materialization;
mod non_novel_identity_alias;
mod proposition_rows;
mod reviewed_pnf;
mod reviewed_source_expansion;
mod review_workstation_store;
mod statement_pnf_spine;
mod statement_trace_store;

pub use chronology_contestation_store::{
    install_chronology_contestation_schema, load_claims_for_event,
    load_contestation_relations_for_claims, load_proposition_roots_for_claims,
    load_temporal_assertions_for_event,
    persist_claim_leaf, persist_contestation_relation, persist_event_claim_link,
    persist_event_temporal_link, persist_proposition_root, persist_temporal_assertion,
    ChronologyContestationStoreError, EventClaimLink, EventTemporalLink,
};
pub use cache_first::{
    resolve_cache_first, AcquiredSourceBundle, CacheFirstAcquirer, CacheFirstError,
    CacheFirstResolution, CacheLookupDemand, CachedResolvedDocument,
    ExactResolutionReceiptOwned, ResolvedExternalDocumentOwned,
};
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
pub use gwb_chronology_capstone::{
    load_gwb_chronology_capstone_manifest, materialize_gwb_chronology_capstone,
    GwbChronologyCapstoneError, GwbChronologyCapstoneManifest,
    GwbChronologyCapstoneReceipt, GWB_CAPSTONE_SCHEMA,
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
    gwb_hop_ledger_row, load_gwb_hops, load_gwb_reviewed_move_refs,
    load_latest_gwb_hop,
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
    load_latent_world_rows, load_latent_world_rows_with_budget,
    load_latent_world_rows_with_context_revision_slice, ContextRevisionWorldSlice,
    LatentWorldBudget, LatentWorldEdgeRow, LatentWorldError, LatentWorldRows,
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
pub use review_workstation_store::{
    install_review_workstation_schema, load_review_queue, persist_review_item,
    persist_review_receipt, ReviewWorkstationStoreError,
};
pub use reviewed_source_expansion::{
    load_reviewed_context_expansion_sources,
    load_reviewed_context_source_revision_coordinates,
    load_reviewed_source_expansion_rows,
    materialize_reviewed_context_expansion, materialize_reviewed_source_expansions,
    reviewed_source_expansion_row,
    ReviewedContextSourceRevisionCoordinate, ReviewedSourceExpansionError,
    ReviewedSourceExpansionInput, ReviewedSourceExpansionMaterializationReceipt,
    ReviewedSourceExpansionRow,
};
pub use statement_pnf_spine::{
    compile_initial_intake_statement, compile_research_reentry_statement,
    compile_statement_pnf, review_statement_parse, ParseReviewDisposition,
    ReviewedStatementPnf, SourceStatementEnvelope, StatementCandidatePnf,
    StatementOrigin, StatementParseReview, StatementPnfSpineError,
};
pub use statement_trace_store::{
    canonical_observation_event_link_ref, canonical_statement_observation_link_ref,
    canonical_statement_ref, install_statement_trace_schema,
    load_observation_event_links_for_event,
    load_observation_event_links_for_observation, load_source_statement,
    load_statement_observation_links_for_observation,
    load_statement_observation_links_for_statement,
    persist_observation_event_link, persist_source_statement,
    persist_statement_observation_link, ObservationEventLink,
    PersistedSourceStatement, StatementObservationDisposition,
    StatementObservationLink, StatementTraceStoreError,
};

// Keep the established PostgreSQL source-store implementation byte-for-byte
// while focused reader/materialisation projections remain separate concerns.
include!("storage_core.rs");
mod forecast_verification;
pub use forecast_verification::*;
