//! SCALE-1 DB-native long-document compiler.
//!
//! Production flow:
//!   canonical text -> persistent source revision + structural manifest
//!   -> deterministic parser run + region jobs
//!   -> distributed workers lease jobs and persist outputs/residuals
//!   -> M12 consumes a DB snapshot
//!   -> durable exhaustive compilation receipt.
//!
//! No TSV/JSON file is required as a runtime handoff. TSV/JSON remain optional
//! parser import/export/debug formats.

use sensiblaw_core::source_ingest::{DocumentRegionKind, SourceFamily};
use thiserror::Error;

use crate::{
    build_plain_text_long_document_source,
    canonical_candidate_pnf_batch_ref, canonical_statement_ref,
    compile_initial_intake_statement,
    compile_long_document_lossless_for_document_ref,
    complete_parser_run, enqueue_parser_regions, install_candidate_pnf_schema,
    install_statement_trace_schema, load_candidate_pnf_batch,
    reconcile_source_candidate_semantics, enqueue_reconciliation_review_items,
    load_generic_source_envelope,
    load_generic_text_source, load_long_document_regions,
    load_long_document_structure, persist_generic_text_source,
    persist_long_document_compilation, persist_long_document_structure,
    persist_source_statement, persist_statement_candidate_pnf,
    start_parser_run, CandidatePnfError, CandidatePnfStoreError,
    CorpusReconciliationError, CorpusReconciliationReceipt,
    ReconciliationReviewError, ReconciliationReviewReceipt,
    DatabaseConfig, DbNativeParserError,
    DbNativeParserSnapshot, GenericSourceCompilerError,
    GenericSourceContentStoreError, LongDocumentIngestStoreError,
    ParserRegionJobSpec, ParserRunReceipt, ParserRunState,
    PersistedGenericSourceContent, PersistedLongDocumentIngestReceipt,
    PersistedLongDocumentRegion, PlainTextDocumentAdapterError,
    PlainTextSegmentationReceipt, SourceStatementEnvelope, StatementOrigin,
    StatementPnfSpineError, StatementTraceStoreError, ExactSourceSpan,
};

#[derive(Debug, Error)]
pub enum DbNativeLongDocumentError {
    #[error("plain-text structural adapter error: {0}")]
    Adapter(#[from] PlainTextDocumentAdapterError),
    #[error("source persistence error: {0}")]
    SourceStore(#[from] GenericSourceContentStoreError),
    #[error("long-document persistence error: {0}")]
    DocumentStore(#[from] LongDocumentIngestStoreError),
    #[error("DB-native parser error: {0}")]
    Parser(#[from] DbNativeParserError),
    #[error("M12 compilation error: {0}")]
    Compiler(#[from] GenericSourceCompilerError),
    #[error("statement/PNF compile error: {0}")]
    StatementCompiler(#[from] StatementPnfSpineError),
    #[error("statement trace persistence error: {0}")]
    StatementStore(#[from] StatementTraceStoreError),
    #[error("candidate PNF persistence error: {0}")]
    CandidatePnfStore(#[from] CandidatePnfStoreError),
    #[error("corpus reconciliation error: {0}")]
    Reconciliation(#[from] CorpusReconciliationError),
    #[error("reconciliation review projection error: {0}")]
    ReconciliationReview(#[from] ReconciliationReviewError),
    #[error("persisted source is not a document content source")]
    NotDocumentContentSource,
    #[error("parser run did not attempt every semantic-eligible region")]
    IncompleteSemanticAttemptCoverage,
    #[error("reopened literal source differs from persisted canonical source")]
    LiteralReloadMismatch,
    #[error("final durable region partition is incomplete")]
    IncompleteDurablePartition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedDbNativeLongDocument {
    pub structural: PlainTextSegmentationReceipt,
    pub source: PersistedGenericSourceContent,
    pub parser_run: ParserRunReceipt,
    pub semantic_region_count: usize,
    pub structural_region_count: usize,
    pub newly_enqueued_job_count: usize,
    pub reused_existing_job_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DbNativeLongDocumentReceipt {
    pub source: PersistedGenericSourceContent,
    pub parser_run_ref: String,
    pub parser_state: ParserRunState,
    pub total_structural_regions: usize,
    pub semantic_eligible_regions: usize,
    pub parser_success_regions: usize,
    pub parser_residual_regions: usize,
    pub unattempted_semantic_regions: usize,
    pub compiled_statement_count: usize,
    pub candidate_pnf_count: usize,
    pub persisted_statement_count: usize,
    pub persisted_candidate_batch_count: usize,
    pub persisted_candidate_factor_count: usize,
    pub candidate_pnf_reopen_complete: bool,
    pub reconciliation: CorpusReconciliationReceipt,
    pub reconciliation_review: ReconciliationReviewReceipt,
    pub structural_only_regions: usize,
    pub source_region_loss_count: usize,
    pub reloaded_regions: Vec<PersistedLongDocumentRegion>,
    pub persisted_compilation: PersistedLongDocumentIngestReceipt,
    pub canonical_bytes_reload_identically: bool,
    pub every_region_reloaded: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_db_native_long_document(
    config: &DatabaseConfig,
    source_ref: &str,
    source_revision_ref: &str,
    provider_ref: &str,
    acquisition_receipt_ref: &str,
    title: Option<String>,
    edition_ref: Option<String>,
    canonical_text: &str,
    parser_family: &str,
    parser_version: &str,
    model_ref: &str,
    parser_config_json: &str,
) -> Result<PreparedDbNativeLongDocument, DbNativeLongDocumentError> {
    let (document, structural) = build_plain_text_long_document_source(
        source_ref,
        source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        title,
        edition_ref,
        canonical_text,
    )?;

    let source = persist_generic_text_source(config, &document.ingest, canonical_text)?;
    persist_long_document_structure(config, &document)?;

    let parser_run = start_parser_run(
        config,
        source_revision_ref,
        parser_family,
        parser_version,
        model_ref,
        parser_config_json,
    )?;

    let semantic_jobs = document
        .regions
        .iter()
        .filter(|region| region.kind == DocumentRegionKind::Sentence)
        .map(|region| ParserRegionJobSpec {
            region_ref: region.region_ref.clone(),
            start_char: region.start_char,
            end_char: region.end_char,
        })
        .collect::<Vec<_>>();

    let newly_enqueued_job_count =
        enqueue_parser_regions(config, &parser_run, &semantic_jobs)?;
    let reused_existing_job_count =
        semantic_jobs.len().saturating_sub(newly_enqueued_job_count);

    Ok(PreparedDbNativeLongDocument {
        structural,
        source,
        parser_run,
        semantic_region_count: semantic_jobs.len(),
        structural_region_count: document
            .regions
            .len()
            .saturating_sub(semantic_jobs.len()),
        newly_enqueued_job_count,
        reused_existing_job_count,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn finalize_db_native_long_document(
    config: &DatabaseConfig,
    parser_run_ref: &str,
) -> Result<DbNativeLongDocumentReceipt, DbNativeLongDocumentError> {
    let parser_state = complete_parser_run(config, parser_run_ref)?;
    let snapshot = DbNativeParserSnapshot::load(config, parser_run_ref)?;
    let source_revision_ref = snapshot.source_revision_ref().to_owned();

    let (source, canonical_text) =
        load_generic_text_source(config, &source_revision_ref)?;
    let envelope = load_generic_source_envelope(config, &source_revision_ref)?;
    if envelope.family != SourceFamily::Document
        || !envelope.semantic_text_allowed()
    {
        return Err(DbNativeLongDocumentError::NotDocumentContentSource);
    }

    let (document, structural) = build_plain_text_long_document_source(
        &envelope.source_ref,
        &envelope.source_revision_ref,
        &envelope.provider_ref,
        &envelope.acquisition_receipt_ref,
        None,
        None,
        &canonical_text,
    )?;

    let persisted_structure =
        load_long_document_structure(config, &source_revision_ref)?;
    if persisted_structure.len() != document.regions.len() {
        return Err(DbNativeLongDocumentError::IncompleteDurablePartition);
    }

    let attempted_regions = parser_state.succeeded + parser_state.residual;
    if attempted_regions != structural.sentence_count
        || parser_state.unattempted_semantic_regions != 0
        || snapshot.compiled_region_count() != parser_state.succeeded
        || snapshot.residual_region_count() != parser_state.residual
    {
        return Err(DbNativeLongDocumentError::IncompleteSemanticAttemptCoverage);
    }

    // Compile against the canonical corpus.document identity used by the
    // durable statement trace, not merely the logical source_ref.
    let compilation = compile_long_document_lossless_for_document_ref(
        &snapshot,
        &document,
        &source.document_ref,
        &canonical_text,
        &format!("db-parser:{parser_run_ref}"),
    )?;

    install_statement_trace_schema(config)?;
    install_candidate_pnf_schema(config)?;

    let mut persisted_statement_count = 0usize;
    let mut persisted_candidate_batch_count = 0usize;
    let mut persisted_candidate_factor_count = 0usize;
    let mut candidate_pnf_reopen_complete = true;

    for region in document
        .regions
        .iter()
        .filter(|region| region.kind == DocumentRegionKind::Sentence)
    {
        let start = usize::try_from(region.start_char)
            .map_err(|_| DbNativeLongDocumentError::IncompleteDurablePartition)?;
        let end = usize::try_from(region.end_char)
            .map_err(|_| DbNativeLongDocumentError::IncompleteDurablePartition)?;
        let literal_text = canonical_text
            .chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect::<String>();
        let start_char = u32::try_from(region.start_char)
            .map_err(|_| DbNativeLongDocumentError::IncompleteDurablePartition)?;
        let end_char = u32::try_from(region.end_char)
            .map_err(|_| DbNativeLongDocumentError::IncompleteDurablePartition)?;

        let mut statement = SourceStatementEnvelope {
            statement_ref: String::new(),
            document_ref: source.document_ref.clone(),
            source_revision_ref: source_revision_ref.clone(),
            span: ExactSourceSpan {
                span_ref: region.region_ref.clone(),
                start_char,
                end_char,
            },
            literal_text,
            origin: StatementOrigin::InitialIntake,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        statement.statement_ref = canonical_statement_ref(&statement);

        let parser_receipt_ref =
            format!("db-parser:{parser_run_ref}:{}", region.region_ref);
        match compile_initial_intake_statement(
            &snapshot,
            statement,
            parser_receipt_ref,
        ) {
            Ok(candidate) => {
                persist_source_statement(config, &candidate.statement)?;
                persisted_statement_count += 1;

                let expected_batch_ref = canonical_candidate_pnf_batch_ref(&candidate);
                let persisted = persist_statement_candidate_pnf(config, &candidate)?;
                persisted_candidate_batch_count += 1;
                persisted_candidate_factor_count += persisted.factors.len();

                let reopened = load_candidate_pnf_batch(config, &expected_batch_ref)?;
                candidate_pnf_reopen_complete &= reopened
                    .as_ref()
                    .is_some_and(|batch| batch == &persisted);
            }
            Err(StatementPnfSpineError::CandidatePnf(
                CandidatePnfError::PersistedParserResidual { .. },
            )) => {
                // Attempted parser residuals remain durable in parser_job and
                // intentionally do not manufacture empty statement/PNF rows.
            }
            Err(error) => return Err(error.into()),
        }
    }

    if persisted_statement_count != parser_state.succeeded
        || persisted_candidate_batch_count != parser_state.succeeded
        || persisted_candidate_factor_count != compilation.candidate_pnf_count
        || !candidate_pnf_reopen_complete
    {
        return Err(DbNativeLongDocumentError::IncompleteDurablePartition);
    }

    let reconciliation =
        reconcile_source_candidate_semantics(
            config,
            &source_revision_ref,
            parser_run_ref,
        )?;

    let reconciliation_review =
        enqueue_reconciliation_review_items(
            config,
            &source_revision_ref,
            vec![],
        )?;

    let persisted_compilation =
        persist_long_document_compilation(config, &document, &compilation)?;

    let reloaded_regions =
        load_long_document_regions(config, &source_revision_ref)?;
    let (reloaded_source, reloaded_text) =
        load_generic_text_source(config, &source_revision_ref)?;
    if source != reloaded_source || reloaded_text != canonical_text {
        return Err(DbNativeLongDocumentError::LiteralReloadMismatch);
    }
    if reloaded_regions.len() != structural.structural_region_count
        || persisted_compilation.assignment_count != structural.structural_region_count
        || persisted_compilation.region_count != structural.structural_region_count
        || persisted_compilation.source_region_loss_count != 0
        || !compilation.source_coverage_complete()
    {
        return Err(DbNativeLongDocumentError::IncompleteDurablePartition);
    }

    Ok(DbNativeLongDocumentReceipt {
        source,
        parser_run_ref: parser_run_ref.to_owned(),
        parser_state: parser_state.clone(),
        total_structural_regions: structural.structural_region_count,
        semantic_eligible_regions: structural.sentence_count,
        parser_success_regions: parser_state.succeeded,
        parser_residual_regions: parser_state.residual,
        unattempted_semantic_regions: parser_state.unattempted_semantic_regions,
        compiled_statement_count: compilation.compiled_statement_count,
        candidate_pnf_count: compilation.candidate_pnf_count,
        persisted_statement_count,
        persisted_candidate_batch_count,
        persisted_candidate_factor_count,
        candidate_pnf_reopen_complete,
        reconciliation,
        reconciliation_review,
        structural_only_regions: compilation.transport_or_nonsemantic_region_count,
        source_region_loss_count: compilation.source_region_loss_count,
        reloaded_regions,
        persisted_compilation,
        canonical_bytes_reload_identically: true,
        every_region_reloaded: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_receipt_boundary_distinguishes_extraction_from_admission() {
        let state = ParserRunState {
            parser_run_ref: "run".into(),
            queued: 0,
            leased: 0,
            succeeded: 10,
            residual: 2,
            unattempted_semantic_regions: 0,
            complete: true,
        };
        assert!(state.complete);
        assert_eq!(state.succeeded + state.residual, 12);
        assert_eq!(state.unattempted_semantic_regions, 0);
    }
}
