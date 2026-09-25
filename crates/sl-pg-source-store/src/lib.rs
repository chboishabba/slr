mod workbench_projection;
pub use workbench_projection::*;
mod cache_first;
mod chat_source_store;
mod chronology_contestation_store;
mod candidate_pnf;
mod candidate_pnf_store;
mod corpus_reconciliation_store;
mod reconciliation_review_store;
mod reconciliation_materialization;
mod scale1_auto_event_store;
mod context_federation;
mod discovery_identity_baseline;
mod event_discovery_store;
mod gwb_ambiguity_state;
mod gwb_campaign_commit;
mod gwb_chronology_capstone;
mod gwb_hop_ledger;
mod discovery_lineage;
mod latent_world;
mod legal_ir_materialization;
mod m13_empirical_schema;
mod non_novel_identity_alias;
mod operational_state_store;
mod proposition_rows;
mod reviewed_pnf;
mod reviewed_source_expansion;
mod review_workstation_store;
mod statement_pnf_spine;
mod generic_source_compiler;
mod jmail_adapter;
mod plain_text_document_adapter;
mod long_document_ingest_store;
mod generic_source_content_store;
mod long_document_pipeline;
mod db_native_parser;
mod db_native_long_document;
mod statement_trace_store;

pub use chronology_contestation_store::{
    install_chronology_contestation_schema, load_claims_for_event,
    load_contestation_relations_for_claims, load_proposition_roots_for_claims,
    load_temporal_assertions_for_event,
    persist_claim_leaf, persist_contestation_relation, persist_event_claim_link,
    persist_event_temporal_link, persist_proposition_root, persist_temporal_assertion,
    ChronologyContestationStoreError, EventClaimLink, EventTemporalLink,
};
pub use chat_source_store::{
    install_chat_source_schema, load_chat_archive_export_jsonl,
    load_chat_message_source, load_chat_messages_for_conversation,
    materialize_chat_statement, persist_chat_archive_message,
    ChatArchiveExportRow, ChatSourceStoreError, PersistedChatMessageSource,
    CHAT_ARCHIVE_EXPORT_SCHEMA,
};
pub use cache_first::{
    resolve_cache_first, AcquiredSourceBundle, CacheFirstAcquirer, CacheFirstError,
    CacheFirstResolution, CacheLookupDemand, CachedResolvedDocument,
    ExactResolutionReceiptOwned, ResolvedExternalDocumentOwned,
};
pub use candidate_pnf::{
    CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor, CandidatePnfProducer,
    CandidatePnfRole, ExactSourceSpan, IndexedSpacyTsvAdapter, SpacyTsvAdapter,
};
pub use candidate_pnf_store::{
    canonical_candidate_pnf_batch_ref, install_candidate_pnf_schema,
    load_candidate_pnf_batch, load_candidate_pnf_batches_for_statement,
    persist_statement_candidate_pnf, CandidatePnfStoreError,
    PersistedCandidatePnfBatch, CANDIDATE_PNF_SCHEMA_SQL,
};
pub use corpus_reconciliation_store::{
    install_corpus_reconciliation_schema, reconcile_source_candidate_semantics,
    CorpusReconciliationError, CorpusReconciliationReceipt,
    CORPUS_RECONCILIATION_SCHEMA_SQL,
};
pub use reconciliation_review_store::{
    enqueue_reconciliation_review_items, ReconciliationReviewError,
    ReconciliationReviewReceipt,
};
pub use reconciliation_materialization::{
    install_reconciliation_materialization_schema,
    materialize_accepted_reconciliation_proposition,
    ReconciliationMaterializationError,
    ReviewedReconciliationPropositionReceipt,
    RECONCILIATION_MATERIALIZATION_SCHEMA_SQL,
};
pub use scale1_auto_event_store::{
    discover_scale1_auto_event_proposals, Scale1AutoEventError,
    Scale1AutoEventReceipt, SCALE1_AUTO_EVENT_SCHEMA_SQL,
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
    GwbChronologyCapstoneReceipt, GwbClaimLeaf, GwbClaimLeafKind,
    GwbClaimReviewState, GwbContestationRelation,
    GwbContestationRelationKind, GwbPropositionRoot, GwbReviewedEventJoin,
    GwbReviewedStatement, GwbReviewAction, GwbReviewItem, GwbReviewItemKind,
    GwbReviewStatus, GwbStatementDisposition, GwbStatementOrigin,
    GwbTemporalAssertion, GwbTemporalForm, GWB_CAPSTONE_SCHEMA,
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
pub use event_discovery_store::{
    install_event_discovery_schema, load_event_join_proposal,
    load_pending_event_join_proposals, persist_event_join_proposal,
    persist_event_join_proposal_with_review, EventDiscoveryStoreError,
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
pub use m13_empirical_schema::*;
pub use legal_ir_materialization::{
    materialize_reviewed_proposition_support, LegalIrMaterializationError, MaterializedLegalIrRefs,
    ReviewedPropositionSupport,
};
pub use non_novel_identity_alias::{
    identity_alias_row, materialize_non_novel_identity_aliases, NonNovelIdentityAliasError,
    NonNovelIdentityAliasInput, NonNovelIdentityAliasMaterializationReceipt,
    NonNovelIdentityAliasRow,
};
pub use operational_state_store::{
    install_operational_state_schema, load_operational_event,
    load_operational_events_for_date, load_operational_outstanding_for_date,
    load_operational_outstanding_state,
    load_operational_semantic_links_for_event,
    load_operational_semantic_links_for_target, load_statibaker_activity_ledger,
    materialize_statibaker_activity_ledger, persist_operational_event,
    persist_operational_outstanding_state, persist_operational_semantic_link,
    validate_statibaker_ledger,
    OperationalStateStoreError, StatiBakerActivityEvent, StatiBakerActivityLedger,
    StatiBakerLedgerProvenance, StatiBakerOperationalImportReceipt,
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
    apply_persisted_review_command, install_review_workstation_schema,
    load_review_queue, persist_review_item, persist_review_receipt,
    ReviewWorkstationStoreError,
};
pub use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewCommand, ReviewEffect, ReviewItem, ReviewReceipt, ReviewStatus,
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
pub use generic_source_compiler::{
    compile_long_document, compile_long_document_lossless,
    compile_long_document_lossless_for_document_ref, compile_mail_message,
    compile_mail_message_lossless, BulkSourceCompilation,
    GenericSourceCompilerError, LosslessBulkSourceCompilation,
    RegionCompilationAssignment, RegionCompilationDisposition,
    RegionCompilationResidual,
};
pub use jmail_adapter::{
    jmail_record_to_mail_source, segment_jmail_body, JmailAdapterError,
    JmailEmailRecord,
};
pub use plain_text_document_adapter::{
    build_plain_text_long_document_source, build_plain_text_long_source_with_family,
    PlainTextDocumentAdapterError, PlainTextSegmentationReceipt,
};
pub use long_document_ingest_store::{
    install_long_document_ingest_schema, load_long_document_ingest_receipt,
    load_long_document_regions, load_long_document_structure,
    persist_long_document_compilation, persist_long_document_structure,
    LongDocumentIngestStoreError, PersistedLongDocumentIngestReceipt,
    PersistedLongDocumentRegion, PersistedLongDocumentStructureRegion,
    LONG_DOCUMENT_INGEST_SCHEMA_SQL,
};
pub use generic_source_content_store::{
    canonical_generic_source_revision_ref, install_generic_source_revision_schema,
    load_generic_source_envelope,
    load_generic_text_source, persist_generic_text_source,
    GenericSourceContentStoreError,
    PersistedGenericSourceContent, GENERIC_SOURCE_REVISION_SCHEMA_SQL,
};
pub use long_document_pipeline::{
    ingest_plain_text_long_document, LongDocumentPipelineError,
    ReopenedLongDocumentIngest,
};
pub use db_native_parser::{
    claim_parser_jobs, complete_parser_run, compilation_key,
    defer_parser_job_retry, enqueue_parser_regions, install_db_native_parser_schema,
    load_claimed_job_text,
    parser_run_ref, parser_run_state, persist_parser_residual,
    persist_parser_success, persist_parser_success_with_entities,
    renew_parser_job_lease, start_parser_run, ClaimedParserJob,
    DbNativeParserError, DbNativeParserSnapshot, ParserArtifactRecord,
    ParserEntityRecord, ParserRegionJobSpec, ParserRunReceipt, ParserRunState,
    ParserTokenRecord,
    PersistedParserOutputReceipt, DB_NATIVE_PARSER_SCHEMA_SQL,
};
pub use db_native_long_document::{
    finalize_db_native_long_document, prepare_db_native_long_document,
    prepare_db_native_long_source, DbNativeLongDocumentError,
    DbNativeLongDocumentReceipt, PreparedDbNativeLongDocument,
};
pub use statement_trace_store::{
    canonical_observation_event_link_ref, canonical_statement_observation_link_ref,
    canonical_statement_ref, install_statement_trace_schema,
    load_observation_event_links_for_event,
    load_observation_event_links_for_observation, load_source_statement,
    load_source_statements_for_document,
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

pub use sensiblaw_core::chat_source::{
    ArchivedChatMessage, ChatBranchMembership, ChatContentKind,
    ChatMessageRole, ChatSourceError, ChatStatementCandidateSpan,
};

#[cfg(test)]
mod public_gwb_capstone_manifest_api_tests {
    #[test]
    fn exposes_every_manifest_member_needed_by_the_gwb_fixture_seeder() {
        use super::{
            GwbClaimLeaf, GwbClaimLeafKind, GwbClaimReviewState,
            GwbContestationRelation, GwbContestationRelationKind,
            GwbPropositionRoot, GwbReviewAction, GwbReviewStatus,
            GwbStatementDisposition, GwbStatementOrigin, GwbTemporalAssertion,
            GwbTemporalForm,
        };

        let _ = std::any::TypeId::of::<GwbClaimLeaf>();
        let _ = std::any::TypeId::of::<GwbClaimLeafKind>();
        let _ = std::any::TypeId::of::<GwbClaimReviewState>();
        let _ = std::any::TypeId::of::<GwbContestationRelation>();
        let _ = std::any::TypeId::of::<GwbContestationRelationKind>();
        let _ = std::any::TypeId::of::<GwbPropositionRoot>();
        let _ = std::any::TypeId::of::<GwbReviewAction>();
        let _ = std::any::TypeId::of::<GwbReviewStatus>();
        let _ = std::any::TypeId::of::<GwbStatementDisposition>();
        let _ = std::any::TypeId::of::<GwbStatementOrigin>();
        let _ = std::any::TypeId::of::<GwbTemporalAssertion>();
        let _ = std::any::TypeId::of::<GwbTemporalForm>();
    }
}
