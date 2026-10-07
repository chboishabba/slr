//! Provider-backed exact source slice -> M12 statement/candidate-PNF handoff.
//!
//! SOURCE-MATERIALISATION-1 stops before review. This module preserves the
//! exact provider source/span ancestry while entering the already-existing M12
//! candidate spine. It cannot assign evidence role, normative order,
//! applicability, proposition support or claim truth.
//!
//! Provider/OALC `corpus.span` coordinates deliberately retain the historical
//! legal-source UTF-8 byte-offset ABI. The generic long-document lane has a
//! separate character-coordinate ancestry path in `statement_trace_store`.
//! Do not silently translate one coordinate system into the other.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    canonical_statement_ref, compile_statement_pnf, load_provider_legal_source_registration,
    load_provider_materialization, persist_source_statement, persist_statement_candidate_pnf,
    verify_provider_rematerialization, CandidatePnfProducer, CandidatePnfStoreError,
    DatabaseConfig, ExactSourceSpan, ProviderLegalSourceRegistrationError,
    ProviderMaterializationError, SourceStatementEnvelope, StatementOrigin,
    StatementPnfSpineError, StatementTraceStoreError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCandidatePnfReceipt {
    pub materialization_ref: String,
    pub source_slice_ref: String,
    pub legal_source_revision_ref: String,
    pub statement_ref: String,
    pub candidate_batch_ref: String,
    pub parser_receipt_ref: String,
    pub candidate_factor_count: usize,
    pub byte_coordinate_contract: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub proposition_support_paid: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum ProviderCandidatePnfError {
    #[error(transparent)]
    Provider(#[from] ProviderMaterializationError),
    #[error(transparent)]
    LegalSource(#[from] ProviderLegalSourceRegistrationError),
    #[error(transparent)]
    Postgres(#[from] postgres::Error),
    #[error(transparent)]
    Compile(#[from] StatementPnfSpineError),
    #[error(transparent)]
    Statement(#[from] StatementTraceStoreError),
    #[error(transparent)]
    CandidateStore(#[from] CandidatePnfStoreError),
    #[error("provider bytes are evicted; candidate-PNF compilation requires verified resident bytes")]
    BytesNotResident,
    #[error("resident provider payload is not valid UTF-8")]
    InvalidUtf8,
    #[error("provider materialisation and curated legal-source registration disagree")]
    LegalSourceRegistrationMismatch,
    #[error("persisted provider source slice does not exist")]
    MissingSourceSlice,
    #[error("persisted source slice does not belong to the provider materialisation")]
    SourceSliceOwnerMismatch,
    #[error("persisted source slice has invalid UTF-8 byte coordinates")]
    InvalidByteSpan,
    #[error("persisted source slice digest does not match the exact literal")]
    SliceDigestMismatch,
    #[error("parser receipt reference is empty")]
    MissingParserReceipt,
    #[error("persisted statement/candidate batch changed during reopen")]
    ReopenMismatch,
}

fn literal_for_byte_span(text: &str, start: u32, end: u32) -> Option<&str> {
    if start >= end {
        return None;
    }
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    if end > text.len() || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }
    Some(&text[start..end])
}

fn sha256_bytes(value: &[u8]) -> [u8; 32] {
    Sha256::digest(value).into()
}

/// Reopen one exact provider-backed legal/source slice and persist it through
/// the existing immutable source-statement and candidate-PNF stores.
///
/// The provider payload must still be resident and must reproduce the durable
/// materialisation digest. A curated, non-promoting provider legal-source
/// registration must already exist for the same materialisation/revision. The
/// selected source-slice digest is independently checked over its exact UTF-8
/// byte slice before the parser is called. Review and all higher-authority
/// payments remain downstream.
pub fn persist_provider_slice_candidate_pnf<P: CandidatePnfProducer>(
    config: &DatabaseConfig,
    materialization_ref: &str,
    source_slice_ref: &str,
    producer: &P,
    parser_receipt_ref: &str,
) -> Result<ProviderCandidatePnfReceipt, ProviderCandidatePnfError> {
    if parser_receipt_ref.trim().is_empty() {
        return Err(ProviderCandidatePnfError::MissingParserReceipt);
    }
    let materialization = load_provider_materialization(config, materialization_ref)?;
    if !materialization.bytes_resident {
        return Err(ProviderCandidatePnfError::BytesNotResident);
    }

    let legal = load_provider_legal_source_registration(
        config,
        &materialization.external_source_revision_ref,
    )?;
    if legal.materialization_ref != materialization.materialization_ref
        || legal.source_revision_ref != materialization.external_source_revision_ref
        || legal.document_ref != materialization.document_ref
        || legal.canonical_text_sha256 != materialization.canonical_sha256_hex
        || !legal.compile_eligible
        || legal.bytes_residency_required_for_identity
        || legal.creates_semantic_authority
        || legal.creates_legal_authority
        || legal.applicability_promoted
        || legal.claim_truth_promoted
    {
        return Err(ProviderCandidatePnfError::LegalSourceRegistrationMismatch);
    }

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT s.external_source_revision_ref,s.span_ref,s.slice_sha256,
                      p.document_ref,p.start_char,p.end_char,c.payload
               FROM corpus.external_source_slice s
               JOIN corpus.span p ON p.span_ref=s.span_ref
               JOIN corpus.document d ON d.document_ref=p.document_ref
               JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
               WHERE s.source_slice_ref=$1"#,
            &[&source_slice_ref],
        )?
        .ok_or(ProviderCandidatePnfError::MissingSourceSlice)?;

    let source_revision_ref: String = row.get(0);
    let span_ref: String = row.get(1);
    let slice_sha256: Vec<u8> = row.get(2);
    let document_ref: String = row.get(3);
    let start_i32: i32 = row.get(4);
    let end_i32: i32 = row.get(5);
    let payload: Option<Vec<u8>> = row.get(6);

    if source_revision_ref != materialization.external_source_revision_ref
        || document_ref != materialization.document_ref
    {
        return Err(ProviderCandidatePnfError::SourceSliceOwnerMismatch);
    }
    let payload = payload.ok_or(ProviderCandidatePnfError::BytesNotResident)?;
    let canonical_text =
        String::from_utf8(payload).map_err(|_| ProviderCandidatePnfError::InvalidUtf8)?;
    verify_provider_rematerialization(&materialization.canonical_sha256_hex, &canonical_text)?;

    let start = u32::try_from(start_i32)
        .map_err(|_| ProviderCandidatePnfError::InvalidByteSpan)?;
    let end = u32::try_from(end_i32)
        .map_err(|_| ProviderCandidatePnfError::InvalidByteSpan)?;
    let literal = literal_for_byte_span(&canonical_text, start, end)
        .ok_or(ProviderCandidatePnfError::InvalidByteSpan)?;
    if slice_sha256.as_slice() != sha256_bytes(literal.as_bytes()).as_slice() {
        return Err(ProviderCandidatePnfError::SliceDigestMismatch);
    }
    drop(client);

    let mut statement = SourceStatementEnvelope {
        statement_ref: String::new(),
        document_ref,
        source_revision_ref,
        span: ExactSourceSpan {
            span_ref,
            start_char: start,
            end_char: end,
        },
        literal_text: literal.to_owned(),
        origin: StatementOrigin::InitialIntake,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    statement.statement_ref = canonical_statement_ref(&statement);

    let candidate = compile_statement_pnf(producer, statement, parser_receipt_ref.to_owned())?;
    let persisted_statement = persist_source_statement(config, &candidate.statement)?;
    let persisted_batch = persist_statement_candidate_pnf(config, &candidate)?;

    if persisted_statement.statement_ref != candidate.statement.statement_ref
        || persisted_statement.source_revision_ref != legal.source_revision_ref
        || persisted_statement.exact_span_ref != candidate.statement.span.span_ref
        || persisted_statement.literal_text != candidate.statement.literal_text
        || persisted_batch.statement_ref != candidate.statement.statement_ref
        || persisted_batch.exact_span_ref != candidate.statement.span.span_ref
        || persisted_batch.parser_receipt_ref != parser_receipt_ref
        || persisted_batch.factors.len() != candidate.pnf.candidates.len()
        || !persisted_batch.candidate_only
        || persisted_batch.semantic_admission_paid
        || persisted_batch.proposition_support_paid
        || persisted_batch.applicability_paid
        || persisted_batch.claim_truth_paid
    {
        return Err(ProviderCandidatePnfError::ReopenMismatch);
    }

    Ok(ProviderCandidatePnfReceipt {
        materialization_ref: materialization.materialization_ref,
        source_slice_ref: source_slice_ref.to_owned(),
        legal_source_revision_ref: legal.source_revision_ref,
        statement_ref: persisted_statement.statement_ref,
        candidate_batch_ref: persisted_batch.batch_ref,
        parser_receipt_ref: persisted_batch.parser_receipt_ref,
        candidate_factor_count: persisted_batch.factors.len(),
        byte_coordinate_contract: true,
        candidate_only: true,
        creates_semantic_authority: false,
        proposition_support_paid: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor, CandidatePnfRole};

    struct OneFactor;

    impl CandidatePnfProducer for OneFactor {
        fn produce(&self, source: &ExactSourceSpan) -> Result<CandidatePnfBatch, CandidatePnfError> {
            Ok(CandidatePnfBatch {
                exact_span_ref: source.span_ref.clone(),
                candidates: vec![CandidatePnfFactor {
                    candidate_ref: format!("candidate:{}", source.span_ref),
                    role: CandidatePnfRole::Predicate,
                    source_start_char: source.start_char,
                    source_end_char: source.end_char,
                    surface: "fixture".into(),
                    lemma: "fixture".into(),
                    dependency_ref: "root".into(),
                    candidate_only: true,
                }],
                proposition_support_paid: false,
                applicability_paid: false,
                claim_truth_paid: false,
            })
        }
    }

    #[test]
    fn provider_legal_span_preserves_utf8_byte_coordinate_contract() {
        let text = "Aé中Z";
        // UTF-8 byte layout: A=[0,1), é=[1,3), 中=[3,6), Z=[6,7).
        assert_eq!(literal_for_byte_span(text, 1, 6), Some("é中"));
        assert_eq!(literal_for_byte_span(text, 3, 7), Some("中Z"));
        assert_eq!(literal_for_byte_span(text, 2, 6), None);
        assert_eq!(literal_for_byte_span(text, 3, 8), None);
    }

    #[test]
    fn provider_handoff_receipt_is_candidate_only_and_non_promoting() {
        let receipt = ProviderCandidatePnfReceipt {
            materialization_ref: "provider-materialization:1".into(),
            source_slice_ref: "source-slice:1".into(),
            legal_source_revision_ref: "external-source-revision:1".into(),
            statement_ref: "statement:1".into(),
            candidate_batch_ref: "pnf-batch:1".into(),
            parser_receipt_ref: "parser:1".into(),
            candidate_factor_count: 1,
            byte_coordinate_contract: true,
            candidate_only: true,
            creates_semantic_authority: false,
            proposition_support_paid: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        assert!(receipt.byte_coordinate_contract);
        assert!(receipt.candidate_only);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.proposition_support_paid);
        assert!(!receipt.applicability_promoted);
        assert!(!receipt.claim_truth_promoted);
        let _ = OneFactor;
    }
}
