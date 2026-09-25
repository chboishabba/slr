//! INGEST-1 executable convergence into the existing M12 statement/PNF spine.
//!
//! This module does not parse chapters or email quoting heuristically. Provider
//! adapters own structural segmentation; this layer proves that once exact
//! regions are supplied, long documents and structured mail enter the same M12
//! compiler without source loss or semantic promotion.

use sensiblaw_core::source_ingest::{
    DocumentRegionKind, LongDocumentSource, MailBodySegmentKind, MailMessageSource,
    SourceIngestError,
};
use thiserror::Error;

use crate::{
    compile_initial_intake_statement, CandidatePnfProducer, ExactSourceSpan,
    SourceStatementEnvelope, StatementCandidatePnf, StatementOrigin,
    StatementPnfSpineError,
};

#[derive(Debug, Error)]
pub enum GenericSourceCompilerError {
    #[error("source ingest error: {0:?}")]
    SourceIngest(SourceIngestError),
    #[error("statement/PNF error: {0:?}")]
    StatementPnf(StatementPnfSpineError),
    #[error("region {region_ref} is outside source text length {text_len}")]
    RegionOutsideText {
        region_ref: String,
        text_len: usize,
    },
    #[error("region {0} cannot be represented by the u32 M12 span ABI")]
    SpanTooLarge(String),
    #[error("region {0} resolved to empty literal text")]
    EmptyLiteral(String),
}

impl From<SourceIngestError> for GenericSourceCompilerError {
    fn from(value: SourceIngestError) -> Self {
        Self::SourceIngest(value)
    }
}

impl From<StatementPnfSpineError> for GenericSourceCompilerError {
    fn from(value: StatementPnfSpineError) -> Self {
        Self::StatementPnf(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkSourceCompilation {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub exact_region_count: usize,
    pub statement_count: usize,
    pub candidate_pnf_count: usize,
    pub skipped_transport_or_nonsemantic_count: usize,
    pub compiled: Vec<StatementCandidatePnf>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub source_omitted_when_parse_fails: bool,
}

fn text_by_char_range(
    text: &str,
    region_ref: &str,
    start_char: u64,
    end_char: u64,
) -> Result<String, GenericSourceCompilerError> {
    let text_len = text.chars().count();
    let start = usize::try_from(start_char).map_err(|_| {
        GenericSourceCompilerError::RegionOutsideText {
            region_ref: region_ref.to_owned(),
            text_len,
        }
    })?;
    let end = usize::try_from(end_char).map_err(|_| {
        GenericSourceCompilerError::RegionOutsideText {
            region_ref: region_ref.to_owned(),
            text_len,
        }
    })?;
    if start >= end || end > text_len {
        return Err(GenericSourceCompilerError::RegionOutsideText {
            region_ref: region_ref.to_owned(),
            text_len,
        });
    }
    let literal = text
        .chars()
        .skip(start)
        .take(end - start)
        .collect::<String>();
    if literal.is_empty() {
        return Err(GenericSourceCompilerError::EmptyLiteral(
            region_ref.to_owned(),
        ));
    }
    Ok(literal)
}

fn exact_span(
    span_ref: &str,
    start_char: u64,
    end_char: u64,
) -> Result<ExactSourceSpan, GenericSourceCompilerError> {
    let start_char = u32::try_from(start_char)
        .map_err(|_| GenericSourceCompilerError::SpanTooLarge(span_ref.to_owned()))?;
    let end_char = u32::try_from(end_char)
        .map_err(|_| GenericSourceCompilerError::SpanTooLarge(span_ref.to_owned()))?;
    Ok(ExactSourceSpan {
        span_ref: span_ref.to_owned(),
        start_char,
        end_char,
    })
}

fn compile_region<P: CandidatePnfProducer>(
    producer: &P,
    document_ref: &str,
    source_revision_ref: &str,
    span_ref: &str,
    start_char: u64,
    end_char: u64,
    literal_text: String,
    parser_receipt_ref: String,
) -> Result<StatementCandidatePnf, GenericSourceCompilerError> {
    let mut statement = SourceStatementEnvelope {
        statement_ref: String::new(),
        document_ref: document_ref.to_owned(),
        source_revision_ref: source_revision_ref.to_owned(),
        span: exact_span(span_ref, start_char, end_char)?,
        literal_text,
        origin: StatementOrigin::InitialIntake,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    statement.statement_ref = crate::canonical_statement_ref(&statement);
    Ok(compile_initial_intake_statement(
        producer,
        statement,
        parser_receipt_ref,
    )?)
}

pub fn compile_long_document<P: CandidatePnfProducer>(
    producer: &P,
    document: &LongDocumentSource,
    canonical_text: &str,
    parser_receipt_prefix: &str,
) -> Result<BulkSourceCompilation, GenericSourceCompilerError> {
    document.validate()?;
    let _canonical_regions = document.canonical_regions()?;

    let mut compiled = Vec::new();
    for region in document
        .regions
        .iter()
        .filter(|region| region.kind == DocumentRegionKind::Sentence)
    {
        let literal = text_by_char_range(
            canonical_text,
            &region.region_ref,
            region.start_char,
            region.end_char,
        )?;
        compiled.push(compile_region(
            producer,
            &document.ingest.source_ref,
            &document.ingest.source_revision_ref,
            &region.region_ref,
            region.start_char,
            region.end_char,
            literal,
            format!("{parser_receipt_prefix}:{}", region.region_ref),
        )?);
    }

    let candidate_pnf_count = compiled
        .iter()
        .map(|statement| statement.pnf.candidates.len())
        .sum();

    Ok(BulkSourceCompilation {
        source_ref: document.ingest.source_ref.clone(),
        source_revision_ref: document.ingest.source_revision_ref.clone(),
        exact_region_count: document.regions.len(),
        statement_count: compiled.len(),
        candidate_pnf_count,
        skipped_transport_or_nonsemantic_count: document
            .regions
            .len()
            .saturating_sub(compiled.len()),
        compiled,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        source_omitted_when_parse_fails: false,
    })
}

pub fn compile_mail_message<P: CandidatePnfProducer>(
    producer: &P,
    message: &MailMessageSource,
    canonical_body_text: &str,
    parser_receipt_prefix: &str,
) -> Result<BulkSourceCompilation, GenericSourceCompilerError> {
    message.validate()?;

    let authored = message
        .segments
        .iter()
        .filter(|segment| segment.is_independent_authorship_candidate())
        .collect::<Vec<_>>();

    let mut compiled = Vec::with_capacity(authored.len());
    for segment in authored {
        let literal = text_by_char_range(
            canonical_body_text,
            &segment.segment_ref,
            segment.start_char,
            segment.end_char,
        )?;
        compiled.push(compile_region(
            producer,
            &message.message_ref,
            &message.body_revision_ref,
            &segment.segment_ref,
            segment.start_char,
            segment.end_char,
            literal,
            format!("{parser_receipt_prefix}:{}", segment.segment_ref),
        )?);
    }

    let candidate_pnf_count = compiled
        .iter()
        .map(|statement| statement.pnf.candidates.len())
        .sum();

    Ok(BulkSourceCompilation {
        source_ref: message.message_ref.clone(),
        source_revision_ref: message.body_revision_ref.clone(),
        exact_region_count: message.segments.len(),
        statement_count: compiled.len(),
        candidate_pnf_count,
        skipped_transport_or_nonsemantic_count: message
            .segments
            .len()
            .saturating_sub(compiled.len()),
        compiled,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        source_omitted_when_parse_fails: false,
    })
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionCompilationDisposition {
    CompiledCandidate { statement_ref: String },
    ParserResidual {
        parser_receipt_ref: String,
        error_ref: String,
    },
    TransportOnly,
    StructuralOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCompilationAssignment {
    pub region_ref: String,
    pub source_revision_ref: String,
    pub disposition: RegionCompilationDisposition,
    pub source_region_preserved: bool,
    pub semantic_authority_created: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionCompilationResidual {
    pub region_ref: String,
    pub source_revision_ref: String,
    pub parser_receipt_ref: String,
    pub error_ref: String,
    pub source_region_preserved: bool,
    pub semantic_authority_created: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessBulkSourceCompilation {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub exact_region_count: usize,
    pub semantic_candidate_region_count: usize,
    pub compiled_statement_count: usize,
    pub candidate_pnf_count: usize,
    pub residuals: Vec<RegionCompilationResidual>,
    pub assignments: Vec<RegionCompilationAssignment>,
    pub transport_or_nonsemantic_region_count: usize,
    pub source_region_accounted_count: usize,
    pub source_region_loss_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub parse_failure_deletes_source: bool,
}

impl LosslessBulkSourceCompilation {
    #[must_use]
    pub fn source_coverage_complete(&self) -> bool {
        let unique = self
            .assignments
            .iter()
            .map(|assignment| assignment.region_ref.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let compiled_assignments = self
            .assignments
            .iter()
            .filter(|assignment| {
                matches!(
                    assignment.disposition,
                    RegionCompilationDisposition::CompiledCandidate { .. }
                )
            })
            .count();
        let residual_assignments = self
            .assignments
            .iter()
            .filter(|assignment| {
                matches!(
                    assignment.disposition,
                    RegionCompilationDisposition::ParserResidual { .. }
                )
            })
            .count();
        let nonsemantic_assignments = self
            .assignments
            .iter()
            .filter(|assignment| {
                matches!(
                    assignment.disposition,
                    RegionCompilationDisposition::TransportOnly
                        | RegionCompilationDisposition::StructuralOnly
                )
            })
            .count();

        self.source_region_accounted_count == self.exact_region_count
            && self.assignments.len() == self.exact_region_count
            && unique.len() == self.exact_region_count
            && compiled_assignments == self.compiled_statement_count
            && residual_assignments == self.residuals.len()
            && compiled_assignments + residual_assignments
                == self.semantic_candidate_region_count
            && nonsemantic_assignments == self.transport_or_nonsemantic_region_count
            && self.assignments.iter().all(|assignment| {
                assignment.source_revision_ref == self.source_revision_ref
                    && assignment.source_region_preserved
                    && !assignment.semantic_authority_created
                    && !assignment.claim_truth_promoted
            })
            && self.source_region_loss_count == 0
    }

    #[must_use]
    pub fn semantic_parse_success_count(&self) -> usize {
        self.compiled_statement_count
    }

    #[must_use]
    pub fn semantic_parse_failure_count(&self) -> usize {
        self.residuals.len()
    }
}

fn residual_for(
    region_ref: &str,
    source_revision_ref: &str,
    parser_receipt_ref: String,
    error: &GenericSourceCompilerError,
) -> RegionCompilationResidual {
    RegionCompilationResidual {
        region_ref: region_ref.to_owned(),
        source_revision_ref: source_revision_ref.to_owned(),
        parser_receipt_ref,
        error_ref: format!("{error}"),
        source_region_preserved: true,
        semantic_authority_created: false,
        claim_truth_promoted: false,
    }
}

pub fn compile_long_document_lossless<P: CandidatePnfProducer>(
    producer: &P,
    document: &LongDocumentSource,
    canonical_text: &str,
    parser_receipt_prefix: &str,
) -> Result<LosslessBulkSourceCompilation, GenericSourceCompilerError> {
    compile_long_document_lossless_with_statement_document_ref(
        producer,
        document,
        &document.ingest.source_ref,
        canonical_text,
        parser_receipt_prefix,
    )
}

pub fn compile_long_document_lossless_for_document_ref<P: CandidatePnfProducer>(
    producer: &P,
    document: &LongDocumentSource,
    statement_document_ref: &str,
    canonical_text: &str,
    parser_receipt_prefix: &str,
) -> Result<LosslessBulkSourceCompilation, GenericSourceCompilerError> {
    if statement_document_ref.trim().is_empty() {
        return Err(GenericSourceCompilerError::StatementPnf(
            StatementPnfSpineError::EmptyCoordinate("document_ref"),
        ));
    }
    compile_long_document_lossless_with_statement_document_ref(
        producer,
        document,
        statement_document_ref,
        canonical_text,
        parser_receipt_prefix,
    )
}

fn compile_long_document_lossless_with_statement_document_ref<P: CandidatePnfProducer>(
    producer: &P,
    document: &LongDocumentSource,
    statement_document_ref: &str,
    canonical_text: &str,
    parser_receipt_prefix: &str,
) -> Result<LosslessBulkSourceCompilation, GenericSourceCompilerError> {
    document.validate()?;
    let canonical_regions = document.canonical_regions()?;
    if canonical_regions.len() != document.regions.len() {
        return Err(GenericSourceCompilerError::SourceIngest(
            SourceIngestError::ContentSpanRequired,
        ));
    }

    let sentence_regions = document
        .regions
        .iter()
        .filter(|region| region.kind == DocumentRegionKind::Sentence)
        .collect::<Vec<_>>();

    let mut compiled = Vec::new();
    let mut residuals = Vec::new();
    let mut assignments = document
        .regions
        .iter()
        .filter(|region| region.kind != DocumentRegionKind::Sentence)
        .map(|region| RegionCompilationAssignment {
            region_ref: region.region_ref.clone(),
            source_revision_ref: region.source_revision_ref.clone(),
            disposition: RegionCompilationDisposition::StructuralOnly,
            source_region_preserved: true,
            semantic_authority_created: false,
            claim_truth_promoted: false,
        })
        .collect::<Vec<_>>();

    for region in &sentence_regions {
        let parser_receipt_ref = format!("{parser_receipt_prefix}:{}", region.region_ref);
        let result = text_by_char_range(
            canonical_text,
            &region.region_ref,
            region.start_char,
            region.end_char,
        )
        .and_then(|literal| {
            compile_region(
                producer,
                statement_document_ref,
                &document.ingest.source_revision_ref,
                &region.region_ref,
                region.start_char,
                region.end_char,
                literal,
                parser_receipt_ref.clone(),
            )
        });

        match result {
            Ok(value) => {
                assignments.push(RegionCompilationAssignment {
                    region_ref: region.region_ref.clone(),
                    source_revision_ref: document.ingest.source_revision_ref.clone(),
                    disposition: RegionCompilationDisposition::CompiledCandidate {
                        statement_ref: value.statement.statement_ref.clone(),
                    },
                    source_region_preserved: true,
                    semantic_authority_created: false,
                    claim_truth_promoted: false,
                });
                compiled.push(value);
            }
            Err(error) => {
                assignments.push(RegionCompilationAssignment {
                    region_ref: region.region_ref.clone(),
                    source_revision_ref: document.ingest.source_revision_ref.clone(),
                    disposition: RegionCompilationDisposition::ParserResidual {
                        parser_receipt_ref: parser_receipt_ref.clone(),
                        error_ref: format!("{error}"),
                    },
                    source_region_preserved: true,
                    semantic_authority_created: false,
                    claim_truth_promoted: false,
                });
                residuals.push(residual_for(
                    &region.region_ref,
                    &document.ingest.source_revision_ref,
                    parser_receipt_ref,
                    &error,
                ));
            }
        }
    }

    let candidate_pnf_count = compiled
        .iter()
        .map(|statement| statement.pnf.candidates.len())
        .sum::<usize>();
    let transport_or_nonsemantic_region_count =
        document.regions.len().saturating_sub(sentence_regions.len());
    assignments.sort_by(|left, right| left.region_ref.cmp(&right.region_ref));
    let source_region_accounted_count = assignments.len();

    Ok(LosslessBulkSourceCompilation {
        source_ref: document.ingest.source_ref.clone(),
        source_revision_ref: document.ingest.source_revision_ref.clone(),
        exact_region_count: document.regions.len(),
        semantic_candidate_region_count: sentence_regions.len(),
        compiled_statement_count: compiled.len(),
        candidate_pnf_count,
        residuals,
        assignments,
        transport_or_nonsemantic_region_count,
        source_region_accounted_count,
        source_region_loss_count: document
            .regions
            .len()
            .saturating_sub(source_region_accounted_count),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        parse_failure_deletes_source: false,
    })
}

pub fn compile_mail_message_lossless<P: CandidatePnfProducer>(
    producer: &P,
    message: &MailMessageSource,
    canonical_body_text: &str,
    parser_receipt_prefix: &str,
) -> Result<LosslessBulkSourceCompilation, GenericSourceCompilerError> {
    message.validate()?;

    let semantic_segments = message
        .segments
        .iter()
        .filter(|segment| segment.is_independent_authorship_candidate())
        .collect::<Vec<_>>();

    let mut compiled = Vec::new();
    let mut residuals = Vec::new();
    let mut assignments = message
        .segments
        .iter()
        .filter(|segment| !segment.is_independent_authorship_candidate())
        .map(|segment| RegionCompilationAssignment {
            region_ref: segment.segment_ref.clone(),
            source_revision_ref: segment.body_revision_ref.clone(),
            disposition: match segment.kind {
                MailBodySegmentKind::QuotedPriorMessage
                | MailBodySegmentKind::ForwardedMessage => {
                    RegionCompilationDisposition::TransportOnly
                }
                _ => RegionCompilationDisposition::StructuralOnly,
            },
            source_region_preserved: true,
            semantic_authority_created: false,
            claim_truth_promoted: false,
        })
        .collect::<Vec<_>>();

    for segment in &semantic_segments {
        let parser_receipt_ref =
            format!("{parser_receipt_prefix}:{}", segment.segment_ref);
        let result = text_by_char_range(
            canonical_body_text,
            &segment.segment_ref,
            segment.start_char,
            segment.end_char,
        )
        .and_then(|literal| {
            compile_region(
                producer,
                &message.message_ref,
                &message.body_revision_ref,
                &segment.segment_ref,
                segment.start_char,
                segment.end_char,
                literal,
                parser_receipt_ref.clone(),
            )
        });

        match result {
            Ok(value) => {
                assignments.push(RegionCompilationAssignment {
                    region_ref: segment.segment_ref.clone(),
                    source_revision_ref: message.body_revision_ref.clone(),
                    disposition: RegionCompilationDisposition::CompiledCandidate {
                        statement_ref: value.statement.statement_ref.clone(),
                    },
                    source_region_preserved: true,
                    semantic_authority_created: false,
                    claim_truth_promoted: false,
                });
                compiled.push(value);
            }
            Err(error) => {
                assignments.push(RegionCompilationAssignment {
                    region_ref: segment.segment_ref.clone(),
                    source_revision_ref: message.body_revision_ref.clone(),
                    disposition: RegionCompilationDisposition::ParserResidual {
                        parser_receipt_ref: parser_receipt_ref.clone(),
                        error_ref: format!("{error}"),
                    },
                    source_region_preserved: true,
                    semantic_authority_created: false,
                    claim_truth_promoted: false,
                });
                residuals.push(residual_for(
                    &segment.segment_ref,
                    &message.body_revision_ref,
                    parser_receipt_ref,
                    &error,
                ));
            }
        }
    }

    let candidate_pnf_count = compiled
        .iter()
        .map(|statement| statement.pnf.candidates.len())
        .sum::<usize>();
    let transport_or_nonsemantic_region_count =
        message.segments.len().saturating_sub(semantic_segments.len());
    assignments.sort_by(|left, right| left.region_ref.cmp(&right.region_ref));
    let source_region_accounted_count = assignments.len();

    Ok(LosslessBulkSourceCompilation {
        source_ref: message.message_ref.clone(),
        source_revision_ref: message.body_revision_ref.clone(),
        exact_region_count: message.segments.len(),
        semantic_candidate_region_count: semantic_segments.len(),
        compiled_statement_count: compiled.len(),
        candidate_pnf_count,
        residuals,
        assignments,
        transport_or_nonsemantic_region_count,
        source_region_accounted_count,
        source_region_loss_count: message
            .segments
            .len()
            .saturating_sub(source_region_accounted_count),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        parse_failure_deletes_source: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::source_ingest::{
        DocumentRegion, IngestRoleClass, MailBodySegment, MailBodySegmentKind,
        MailParticipant, SourceFamily, SourceIngestEnvelope,
    };

    use crate::{CandidatePnfBatch, CandidatePnfError, CandidatePnfFactor, CandidatePnfRole};

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

    fn ingest(family: SourceFamily) -> SourceIngestEnvelope {
        SourceIngestEnvelope {
            source_ref: "source:fixture".into(),
            source_revision_ref: "revision:fixture".into(),
            provider_ref: "provider:fixture".into(),
            family,
            role_class: IngestRoleClass::ContentSource,
            content_digest_ref: "sha256:fixture".into(),
            acquisition_receipt_ref: "receipt:fixture".into(),
            media_type_ref: "text/plain".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn long_document_compiles_every_sentence_region_through_m12() {
        let text = "One sentence. Two sentence.";
        let document = LongDocumentSource {
            ingest: ingest(SourceFamily::Document),
            title: Some("Fixture book".into()),
            edition_ref: Some("edition:fixture".into()),
            regions: vec![
                DocumentRegion {
                    region_ref: "chapter:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Chapter,
                    start_char: 0,
                    end_char: 27,
                },
                DocumentRegion {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 13,
                },
                DocumentRegion {
                    region_ref: "sentence:2".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 14,
                    end_char: 27,
                },
            ],
        };

        let receipt =
            compile_long_document(&EchoProducer, &document, text, "parse:book").unwrap();
        assert_eq!(receipt.exact_region_count, 3);
        assert_eq!(receipt.statement_count, 2);
        assert_eq!(receipt.candidate_pnf_count, 2);
        assert_eq!(receipt.skipped_transport_or_nonsemantic_count, 1);
        assert_eq!(receipt.compiled[0].statement.literal_text, "One sentence.");
        assert_eq!(receipt.compiled[1].statement.literal_text, "Two sentence.");
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.claim_truth_promoted);
    }

    #[test]
    fn mail_compiles_authored_span_but_not_quote_or_signature() {
        let body = "New claim. Old quoted claim. -- Alice";
        let message = MailMessageSource {
            ingest: ingest(SourceFamily::Mail),
            message_ref: "mail:1".into(),
            account_or_collection_ref: "collection:jmail".into(),
            provider_message_id: Some("jmail:1".into()),
            internet_message_id: Some("<mail-1@example.test>".into()),
            thread_ref: Some("thread:1".into()),
            reply_to_ref: Some("mail:0".into()),
            reference_message_refs: vec!["mail:0".into()],
            from: vec![MailParticipant {
                display_name: Some("Alice".into()),
                address: "alice@example.test".into(),
            }],
            to: vec![],
            cc: vec![],
            bcc: vec![],
            sent_time_ref: Some("time:1".into()),
            received_time_ref: None,
            subject_revision_ref: None,
            body_revision_ref: "revision:fixture".into(),
            attachment_refs: vec![],
            raw_source_ref: "raw:mail:1".into(),
            segments: vec![
                MailBodySegment {
                    segment_ref: "segment:authored".into(),
                    body_revision_ref: "revision:fixture".into(),
                    kind: MailBodySegmentKind::AuthoredHere,
                    start_char: 0,
                    end_char: 10,
                    derived_from_message_ref: None,
                    canonical_text_lineage_ref: Some("lineage:new".into()),
                },
                MailBodySegment {
                    segment_ref: "segment:quote".into(),
                    body_revision_ref: "revision:fixture".into(),
                    kind: MailBodySegmentKind::QuotedPriorMessage,
                    start_char: 11,
                    end_char: 28,
                    derived_from_message_ref: Some("mail:0".into()),
                    canonical_text_lineage_ref: Some("lineage:old".into()),
                },
                MailBodySegment {
                    segment_ref: "segment:signature".into(),
                    body_revision_ref: "revision:fixture".into(),
                    kind: MailBodySegmentKind::Signature,
                    start_char: 29,
                    end_char: 37,
                    derived_from_message_ref: None,
                    canonical_text_lineage_ref: None,
                },
            ],
        };

        let receipt =
            compile_mail_message(&EchoProducer, &message, body, "parse:mail").unwrap();
        assert_eq!(receipt.exact_region_count, 3);
        assert_eq!(receipt.statement_count, 1);
        assert_eq!(receipt.candidate_pnf_count, 1);
        assert_eq!(receipt.skipped_transport_or_nonsemantic_count, 2);
        assert_eq!(receipt.compiled[0].statement.literal_text, "New claim.");
        assert!(!receipt.source_omitted_when_parse_fails);
    }


    struct FailSecondProducer;

    impl CandidatePnfProducer for FailSecondProducer {
        fn produce(
            &self,
            source: &ExactSourceSpan,
        ) -> Result<CandidatePnfBatch, CandidatePnfError> {
            if source.span_ref == "sentence:2" {
                return Err(CandidatePnfError::MalformedRow {
                    line: 1,
                    detail: "fixture parser failure".into(),
                });
            }
            EchoProducer.produce(source)
        }
    }

    #[test]
    fn long_document_lossless_compiler_preserves_source_after_parse_failure() {
        let text = "One sentence. Two sentence. Three sentence.";
        let document = LongDocumentSource {
            ingest: ingest(SourceFamily::Document),
            title: Some("Fixture book".into()),
            edition_ref: Some("edition:fixture".into()),
            regions: vec![
                DocumentRegion {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 13,
                },
                DocumentRegion {
                    region_ref: "sentence:2".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Sentence,
                    start_char: 14,
                    end_char: 27,
                },
                DocumentRegion {
                    region_ref: "sentence:3".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Sentence,
                    start_char: 28,
                    end_char: 43,
                },
            ],
        };

        let receipt = compile_long_document_lossless(
            &FailSecondProducer,
            &document,
            text,
            "parse:book",
        )
        .unwrap();

        assert_eq!(receipt.exact_region_count, 3);
        assert_eq!(receipt.semantic_candidate_region_count, 3);
        assert_eq!(receipt.compiled_statement_count, 2);
        assert_eq!(receipt.semantic_parse_failure_count(), 1);
        assert_eq!(receipt.residuals[0].region_ref, "sentence:2");
        assert!(receipt.residuals[0].source_region_preserved);
        assert_eq!(receipt.source_region_accounted_count, 3);
        assert_eq!(receipt.assignments.len(), 3);
        assert!(receipt.assignments.iter().any(|assignment| {
            assignment.region_ref == "sentence:2"
                && matches!(
                    assignment.disposition,
                    RegionCompilationDisposition::ParserResidual { .. }
                )
        }));
        assert_eq!(receipt.source_region_loss_count, 0);
        assert!(receipt.source_coverage_complete());
        assert!(!receipt.parse_failure_deletes_source);
        assert!(!receipt.claim_truth_promoted);
    }


    #[test]
    fn explicit_partition_has_no_unaccounted_fourth_bucket() {
        let text = "One sentence. Two sentence.";
        let document = LongDocumentSource {
            ingest: ingest(SourceFamily::Document),
            title: None,
            edition_ref: None,
            regions: vec![
                DocumentRegion {
                    region_ref: "chapter:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Chapter,
                    start_char: 0,
                    end_char: 27,
                },
                DocumentRegion {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 13,
                },
                DocumentRegion {
                    region_ref: "sentence:2".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 14,
                    end_char: 27,
                },
            ],
        };

        let receipt =
            compile_long_document_lossless(&EchoProducer, &document, text, "parse:book")
                .unwrap();

        assert_eq!(receipt.assignments.len(), 3);
        assert_eq!(
            receipt
                .assignments
                .iter()
                .filter(|assignment| matches!(
                    assignment.disposition,
                    RegionCompilationDisposition::CompiledCandidate { .. }
                ))
                .count(),
            2
        );
        assert_eq!(
            receipt
                .assignments
                .iter()
                .filter(|assignment| matches!(
                    assignment.disposition,
                    RegionCompilationDisposition::StructuralOnly
                ))
                .count(),
            1
        );
        assert!(receipt.source_coverage_complete());
    }

    #[test]
    fn unicode_offsets_are_character_not_byte_offsets() {
        let text = "αβγ. Next.";
        let document = LongDocumentSource {
            ingest: ingest(SourceFamily::Document),
            title: None,
            edition_ref: None,
            regions: vec![DocumentRegion {
                region_ref: "sentence:unicode".into(),
                source_revision_ref: "revision:fixture".into(),
                parent_region_ref: None,
                kind: DocumentRegionKind::Sentence,
                start_char: 0,
                end_char: 4,
            }],
        };
        let receipt =
            compile_long_document(&EchoProducer, &document, text, "parse:unicode").unwrap();
        assert_eq!(receipt.compiled[0].statement.literal_text, "αβγ.");
    }
}
