//! Source-family-neutral M12 persistence for an already-compiled,
 //! losslessly accounted *generic* source revision. No GWB/book lane,
 //! semantic admission, or extra parser execution is introduced.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use thiserror::Error;

use crate::{
    candidate_pnf_store::persist_statement_candidate_pnf_with_client,
    statement_trace_store::persist_source_statement_with_client,
    DatabaseConfig, LosslessBulkSourceCompilation,
};

#[derive(Debug, Error)]
pub enum GenericCandidatePersistenceError {
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    SourceStore(#[from] crate::GenericSourceContentStoreError),
    #[error(transparent)]
    Statement(#[from] crate::StatementTraceStoreError),
    #[error(transparent)]
    Candidate(#[from] crate::CandidatePnfStoreError),
    #[error("compilation source or canonical document identity differs from source store")]
    SourceIdentityMismatch,
    #[error("lossless compilation region partition is not complete")]
    IncompletePartition,
    #[error("candidate span/literal does not match reopened canonical text")]
    CanonicalSpanMismatch,
    #[error("canonical statement span already exists with conflicting metadata")]
    SpanIdentityConflict,
    #[error("canonical character offsets exceed persisted SQL span ABI")]
    SpanOverflow,
    #[error("candidate persistence reopened with inconsistent identity")]
    CandidateReopenMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericCandidatePersistenceReceipt {
    pub source_revision_ref: String,
    pub document_ref: String,
    pub statement_count: usize,
    pub candidate_batch_count: usize,
    pub candidate_factor_count: usize,
    pub residual_region_count: usize,
    pub structural_or_transport_region_count: usize,
    pub exact_reopen_validated: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

/// A generic, source-revision-scoped persistence method. The source-family
/// adapter must have already persisted its own provenance and exact regions.
/// Each successful M12 statement is inserted through the existing immutable
/// statement/candidate stores and reopened by their established helpers.
///
/// Do not interpret this receipt as transactionally committing an entire
/// corpus or authenticating measurements; individual batches use established
/// DB transactions and are idempotently resumable.
pub fn persist_lossless_generic_candidates(
    config: &DatabaseConfig,
    compilation: &LosslessBulkSourceCompilation,
) -> Result<GenericCandidatePersistenceReceipt, GenericCandidatePersistenceError> {
    if !compilation.source_coverage_complete()
        || !compilation.candidate_only
        || compilation.creates_semantic_authority
        || compilation.applicability_promoted
        || compilation.claim_truth_promoted
        || compilation.parse_failure_deletes_source
    {
        return Err(GenericCandidatePersistenceError::IncompletePartition);
    }
    let (source, canonical_text) = crate::load_generic_text_source(
        config, &compilation.source_revision_ref,
    )?;
    if source.source_ref != compilation.source_ref
        || source.source_revision_ref != compilation.source_revision_ref
    {
        return Err(GenericCandidatePersistenceError::SourceIdentityMismatch);
    }

    let expected_offsets = compilation.compiled.iter().flat_map(|candidate| {
        [
            u64::from(candidate.statement.span.start_char),
            u64::from(candidate.statement.span.end_char),
        ]
    }).collect::<BTreeSet<_>>();
    let mut byte_offsets = BTreeMap::new();
    for (char_index, (byte_offset, _)) in canonical_text.char_indices().enumerate() {
        if expected_offsets.contains(&(char_index as u64)) {
            byte_offsets.insert(char_index as u64, byte_offset);
        }
    }
    let char_end = canonical_text.chars().count() as u64;
    if expected_offsets.contains(&char_end) {
        byte_offsets.insert(char_end, canonical_text.len());
    }

    // Check every item against immutable source and canonical bytes *before*
    // attempting any persistent write.
    for candidate in &compilation.compiled {
        let statement = &candidate.statement;
        if statement.source_revision_ref != source.source_revision_ref
            || statement.document_ref != source.document_ref
            || statement.creates_semantic_authority
            || statement.claim_truth_promoted
            || !statement.candidate_only
            || !candidate.candidate_only
            || candidate.semantic_admission_paid
            || candidate.proposition_support_paid
            || candidate.applicability_paid
            || candidate.claim_truth_paid
        {
            return Err(GenericCandidatePersistenceError::SourceIdentityMismatch);
        }
        let start = byte_offsets.get(&u64::from(statement.span.start_char));
        let end = byte_offsets.get(&u64::from(statement.span.end_char));
        match (start, end) {
            (Some(&a), Some(&b))
                if a < b && canonical_text[a..b] == statement.literal_text => {}
            _ => return Err(GenericCandidatePersistenceError::CanonicalSpanMismatch),
        }
        if i32::try_from(statement.span.end_char).is_err() {
            return Err(GenericCandidatePersistenceError::SpanOverflow);
        }
        candidate.validate().map_err(|_| GenericCandidatePersistenceError::CanonicalSpanMismatch)?;
    }

    crate::install_statement_trace_schema(config)?;
    crate::install_candidate_pnf_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut batches = 0usize;
    let mut factors = 0usize;
    for candidate in &compilation.compiled {
        let statement = &candidate.statement;
        client.execute(
            "INSERT INTO corpus.span
                (span_ref, document_ref, start_char, end_char,
                 start_token, end_token, span_type_ref)
             VALUES ($1,$2,$3,$4,NULL,NULL,'generic_semantic_candidate')
             ON CONFLICT (span_ref) DO NOTHING",
            &[
                &statement.span.span_ref,
                &source.document_ref,
                &(statement.span.start_char as i32),
                &(statement.span.end_char as i32),
            ],
        )?;
        let row = client.query_one(
            "SELECT document_ref, start_char, end_char
             FROM corpus.span WHERE span_ref=$1",
            &[&statement.span.span_ref],
        )?;
        if row.get::<_, String>(0) != source.document_ref
            || row.get::<_, i32>(1) != statement.span.start_char as i32
            || row.get::<_, i32>(2) != statement.span.end_char as i32
        {
            return Err(GenericCandidatePersistenceError::SpanIdentityConflict);
        }
        let persisted = persist_source_statement_with_client(&mut client, statement)?;
        if persisted.statement_ref != statement.statement_ref {
            return Err(GenericCandidatePersistenceError::CandidateReopenMismatch);
        }
        let batch = persist_statement_candidate_pnf_with_client(&mut client, candidate)?;
        if batch.statement_ref != statement.statement_ref
            || batch.exact_span_ref != statement.span.span_ref
            || batch.factors.len() != candidate.pnf.candidates.len()
            || !batch.candidate_only
            || batch.semantic_admission_paid
            || batch.claim_truth_paid
        {
            return Err(GenericCandidatePersistenceError::CandidateReopenMismatch);
        }
        batches += 1;
        factors += batch.factors.len();
    }
    if batches != compilation.compiled_statement_count
        || factors != compilation.candidate_pnf_count
    {
        return Err(GenericCandidatePersistenceError::CandidateReopenMismatch);
    }
    Ok(GenericCandidatePersistenceReceipt {
        source_revision_ref: source.source_revision_ref,
        document_ref: source.document_ref,
        statement_count: batches,
        candidate_batch_count: batches,
        candidate_factor_count: factors,
        residual_region_count: compilation.residuals.len(),
        structural_or_transport_region_count:
            compilation.transport_or_nonsemantic_region_count,
        exact_reopen_validated: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}
