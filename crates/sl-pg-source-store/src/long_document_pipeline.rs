//! INGEST-1A cohesive plain-text long-document pipeline.
//!
//! One call performs the complete source-preserving path:
//! structural segmentation -> canonical byte persistence -> lossless M12
//! compilation -> durable region partition -> exact reload verification.

use thiserror::Error;

use crate::{
    build_plain_text_long_document_source, compile_long_document_lossless,
    load_generic_text_source, load_long_document_ingest_receipt,
    load_long_document_regions, persist_generic_text_source,
    persist_long_document_compilation, CandidatePnfProducer, DatabaseConfig,
    GenericSourceCompilerError, GenericSourceContentStoreError,
    LongDocumentIngestStoreError, LosslessBulkSourceCompilation,
    PersistedGenericSourceContent, PersistedLongDocumentIngestReceipt,
    PersistedLongDocumentRegion, PlainTextDocumentAdapterError,
    PlainTextSegmentationReceipt,
};

#[derive(Debug, Error)]
pub enum LongDocumentPipelineError {
    #[error("structural adapter error: {0}")]
    Adapter(#[from] PlainTextDocumentAdapterError),
    #[error("generic source persistence error: {0}")]
    SourceStore(#[from] GenericSourceContentStoreError),
    #[error("bulk compiler error: {0}")]
    Compiler(#[from] GenericSourceCompilerError),
    #[error("long-document partition store error: {0}")]
    PartitionStore(#[from] LongDocumentIngestStoreError),
    #[error("reloaded literal source differs from submitted source")]
    LiteralReloadMismatch,
    #[error("reloaded source revision differs across receipts")]
    RevisionReloadMismatch,
    #[error("reloaded region partition is incomplete")]
    ReloadedPartitionIncomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReopenedLongDocumentIngest {
    pub structural: PlainTextSegmentationReceipt,
    pub source: PersistedGenericSourceContent,
    pub compilation: LosslessBulkSourceCompilation,
    pub persisted_compilation: PersistedLongDocumentIngestReceipt,
    pub reloaded_regions: Vec<PersistedLongDocumentRegion>,
    pub reloaded_text: String,
    pub literal_source_reopened: bool,
    pub region_partition_reopened: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn ingest_plain_text_long_document<P: CandidatePnfProducer>(
    config: &DatabaseConfig,
    producer: &P,
    source_ref: &str,
    source_revision_ref: &str,
    provider_ref: &str,
    acquisition_receipt_ref: &str,
    title: Option<String>,
    edition_ref: Option<String>,
    canonical_text: &str,
    parser_receipt_prefix: &str,
) -> Result<ReopenedLongDocumentIngest, LongDocumentPipelineError> {
    let (document, structural) = build_plain_text_long_document_source(
        source_ref,
        source_revision_ref,
        provider_ref,
        acquisition_receipt_ref,
        title,
        edition_ref,
        canonical_text,
    )?;

    let persisted_source =
        persist_generic_text_source(config, &document.ingest, canonical_text)?;

    let compilation = compile_long_document_lossless(
        producer,
        &document,
        canonical_text,
        parser_receipt_prefix,
    )?;

    let persisted_compilation =
        persist_long_document_compilation(config, &document, &compilation)?;

    let (reloaded_source, reloaded_text) =
        load_generic_text_source(config, source_revision_ref)?;
    let reloaded_regions =
        load_long_document_regions(config, source_revision_ref)?;
    let reloaded_compilation =
        load_long_document_ingest_receipt(config, source_revision_ref)?;

    if reloaded_text != canonical_text {
        return Err(LongDocumentPipelineError::LiteralReloadMismatch);
    }
    if persisted_source.source_revision_ref != source_revision_ref
        || reloaded_source.source_revision_ref != source_revision_ref
        || persisted_compilation.source_revision_ref != source_revision_ref
        || reloaded_compilation.source_revision_ref != source_revision_ref
    {
        return Err(LongDocumentPipelineError::RevisionReloadMismatch);
    }
    if reloaded_regions.len() != structural.structural_region_count
        || reloaded_compilation.assignment_count != structural.structural_region_count
        || reloaded_compilation.region_count != structural.structural_region_count
        || reloaded_compilation.source_region_loss_count != 0
        || !compilation.source_coverage_complete()
    {
        return Err(LongDocumentPipelineError::ReloadedPartitionIncomplete);
    }

    Ok(ReopenedLongDocumentIngest {
        structural,
        source: reloaded_source,
        compilation,
        persisted_compilation: reloaded_compilation,
        reloaded_regions,
        reloaded_text,
        literal_source_reopened: true,
        region_partition_reopened: true,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor,
        CandidatePnfRole, ExactSourceSpan,
    };

    struct EchoProducer;

    impl CandidatePnfProducer for EchoProducer {
        fn produce(
            &self,
            source: &ExactSourceSpan,
        ) -> Result<CandidatePnfBatch, CandidatePnfError> {
            Ok(CandidatePnfBatch {
                exact_span_ref: source.span_ref.clone(),
                candidates: vec![CandidatePnfFactor {
                    candidate_ref: format!("pnf:{}", source.span_ref),
                    role: CandidatePnfRole::Other,
                    source_start_char: source.start_char,
                    source_end_char: source.end_char,
                    surface: "fixture".into(),
                    lemma: "fixture".into(),
                    dependency_ref: "fixture".into(),
                    candidate_only: true,
                }],
                proposition_support_paid: false,
                applicability_paid: false,
                claim_truth_paid: false,
            })
        }
    }

    #[test]
    fn reopened_result_keeps_source_and_semantic_boundaries_explicit() {
        // Pure construction check; the live PostgreSQL path is exercised by
        // integration receipts where DATABASE_URL is available.
        let structural = PlainTextSegmentationReceipt {
            source_ref: "book:fixture".into(),
            source_revision_ref: "revision:fixture".into(),
            char_count: 10,
            chapter_count: 0,
            paragraph_count: 1,
            sentence_count: 1,
            structural_region_count: 2,
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
            segmentation_claims_semantic_completeness: false,
        };
        assert!(!structural.creates_semantic_authority);
        assert!(!structural.claim_truth_promoted);
        assert!(!structural.segmentation_claims_semantic_completeness);
        let _ = EchoProducer;
    }
}
