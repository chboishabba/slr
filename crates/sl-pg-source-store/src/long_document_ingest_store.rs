//! INGEST-1A durable long-document structural/compilation receipts.
//!
//! The canonical source text/revision remains owned by the existing source
//! substrate. This store persists the book-scale structural index and the
//! exhaustive region compilation partition so a restart can audit exactly what
//! was compiled, what failed parsing, and what remained structural.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use sensiblaw_core::source_ingest::{DocumentRegionKind, LongDocumentSource};
use thiserror::Error;

use crate::{
    DatabaseConfig, LosslessBulkSourceCompilation, RegionCompilationDisposition,
};

pub const LONG_DOCUMENT_INGEST_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS ingest;

CREATE TABLE IF NOT EXISTS ingest.long_document_region (
    source_revision_ref TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    region_ref TEXT NOT NULL,
    parent_region_ref TEXT NULL,
    region_kind TEXT NOT NULL,
    start_char BIGINT NOT NULL,
    end_char BIGINT NOT NULL,
    candidate_only BOOLEAN NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL,
    PRIMARY KEY (source_revision_ref, region_ref)
);

CREATE TABLE IF NOT EXISTS ingest.region_compilation_assignment (
    source_revision_ref TEXT NOT NULL,
    region_ref TEXT NOT NULL,
    disposition TEXT NOT NULL,
    statement_ref TEXT NULL,
    parser_receipt_ref TEXT NULL,
    error_ref TEXT NULL,
    source_region_preserved BOOLEAN NOT NULL,
    semantic_authority_created BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL,
    PRIMARY KEY (source_revision_ref, region_ref),
    FOREIGN KEY (source_revision_ref, region_ref)
      REFERENCES ingest.long_document_region(source_revision_ref, region_ref)
      ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS ingest.lossless_compilation_receipt (
    source_revision_ref TEXT PRIMARY KEY,
    source_ref TEXT NOT NULL,
    exact_region_count BIGINT NOT NULL,
    semantic_candidate_region_count BIGINT NOT NULL,
    compiled_statement_count BIGINT NOT NULL,
    parser_residual_count BIGINT NOT NULL,
    candidate_pnf_count BIGINT NOT NULL,
    transport_or_nonsemantic_region_count BIGINT NOT NULL,
    source_region_accounted_count BIGINT NOT NULL,
    source_region_loss_count BIGINT NOT NULL,
    candidate_only BOOLEAN NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL,
    applicability_promoted BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL,
    parse_failure_deletes_source BOOLEAN NOT NULL
);
"#;

#[derive(Debug, Error)]
pub enum LongDocumentIngestStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("long-document source failed validation: {0:?}")]
    InvalidDocument(sensiblaw_core::source_ingest::SourceIngestError),
    #[error("compilation source/revision differs from long-document source")]
    SourceRevisionMismatch,
    #[error("lossless compilation receipt is not exhaustive")]
    IncompleteCoverage,
    #[error("region assignments do not exactly match document regions")]
    RegionPartitionMismatch,
    #[error("assignment contains promoted or lost source state for {0}")]
    InvalidAssignment(String),
    #[error("stored receipt differs from supplied receipt")]
    ReceiptRoundTripMismatch,
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLongDocumentRegion {
    pub source_revision_ref: String,
    pub source_ref: String,
    pub region_ref: String,
    pub parent_region_ref: Option<String>,
    pub region_kind: String,
    pub start_char: u64,
    pub end_char: u64,
    pub disposition: RegionCompilationDisposition,
    pub source_region_preserved: bool,
    pub semantic_authority_created: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLongDocumentIngestReceipt {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub region_count: usize,
    pub assignment_count: usize,
    pub compiled_count: usize,
    pub residual_count: usize,
    pub structural_count: usize,
    pub transport_count: usize,
    pub source_region_loss_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub parse_failure_deletes_source: bool,
}

fn region_kind_db(kind: DocumentRegionKind) -> &'static str {
    match kind {
        DocumentRegionKind::Chapter => "chapter",
        DocumentRegionKind::Section => "section",
        DocumentRegionKind::Page => "page",
        DocumentRegionKind::Paragraph => "paragraph",
        DocumentRegionKind::Sentence => "sentence",
        DocumentRegionKind::Footnote => "footnote",
        DocumentRegionKind::TableCell => "table_cell",
        DocumentRegionKind::Other => "other",
    }
}

fn disposition_parts(
    disposition: &RegionCompilationDisposition,
) -> (&'static str, Option<&str>, Option<&str>, Option<&str>) {
    match disposition {
        RegionCompilationDisposition::CompiledCandidate { statement_ref } => {
            ("compiled_candidate", Some(statement_ref.as_str()), None, None)
        }
        RegionCompilationDisposition::ParserResidual {
            parser_receipt_ref,
            error_ref,
        } => (
            "parser_residual",
            None,
            Some(parser_receipt_ref.as_str()),
            Some(error_ref.as_str()),
        ),
        RegionCompilationDisposition::TransportOnly => ("transport_only", None, None, None),
        RegionCompilationDisposition::StructuralOnly => ("structural_only", None, None, None),
    }
}

fn validate_partition(
    document: &LongDocumentSource,
    compilation: &LosslessBulkSourceCompilation,
) -> Result<(), LongDocumentIngestStoreError> {
    document
        .validate()
        .map_err(LongDocumentIngestStoreError::InvalidDocument)?;

    if compilation.source_ref != document.ingest.source_ref
        || compilation.source_revision_ref != document.ingest.source_revision_ref
    {
        return Err(LongDocumentIngestStoreError::SourceRevisionMismatch);
    }
    if !compilation.source_coverage_complete() {
        return Err(LongDocumentIngestStoreError::IncompleteCoverage);
    }

    let region_refs = document
        .regions
        .iter()
        .map(|region| region.region_ref.as_str())
        .collect::<BTreeSet<_>>();
    let assignment_refs = compilation
        .assignments
        .iter()
        .map(|assignment| assignment.region_ref.as_str())
        .collect::<BTreeSet<_>>();
    if region_refs != assignment_refs {
        return Err(LongDocumentIngestStoreError::RegionPartitionMismatch);
    }

    for assignment in &compilation.assignments {
        if assignment.source_revision_ref != document.ingest.source_revision_ref
            || !assignment.source_region_preserved
            || assignment.semantic_authority_created
            || assignment.claim_truth_promoted
        {
            return Err(LongDocumentIngestStoreError::InvalidAssignment(
                assignment.region_ref.clone(),
            ));
        }
    }

    Ok(())
}

pub fn install_long_document_ingest_schema(
    config: &DatabaseConfig,
) -> Result<(), LongDocumentIngestStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_long_document_compilation(
    config: &DatabaseConfig,
    document: &LongDocumentSource,
    compilation: &LosslessBulkSourceCompilation,
) -> Result<PersistedLongDocumentIngestReceipt, LongDocumentIngestStoreError> {
    validate_partition(document, compilation)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;

    for region in &document.regions {
        tx.execute(
            "INSERT INTO ingest.long_document_region (
                source_revision_ref, source_ref, region_ref, parent_region_ref,
                region_kind, start_char, end_char, candidate_only,
                creates_semantic_authority, claim_truth_promoted
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE)
             ON CONFLICT (source_revision_ref, region_ref) DO UPDATE SET
                source_ref = EXCLUDED.source_ref,
                parent_region_ref = EXCLUDED.parent_region_ref,
                region_kind = EXCLUDED.region_kind,
                start_char = EXCLUDED.start_char,
                end_char = EXCLUDED.end_char",
            &[
                &document.ingest.source_revision_ref,
                &document.ingest.source_ref,
                &region.region_ref,
                &region.parent_region_ref,
                &region_kind_db(region.kind),
                &(region.start_char as i64),
                &(region.end_char as i64),
            ],
        )?;
    }

    for assignment in &compilation.assignments {
        let (disposition, statement_ref, parser_receipt_ref, error_ref) =
            disposition_parts(&assignment.disposition);
        tx.execute(
            "INSERT INTO ingest.region_compilation_assignment (
                source_revision_ref, region_ref, disposition, statement_ref,
                parser_receipt_ref, error_ref, source_region_preserved,
                semantic_authority_created, claim_truth_promoted
             ) VALUES ($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE)
             ON CONFLICT (source_revision_ref, region_ref) DO UPDATE SET
                disposition = EXCLUDED.disposition,
                statement_ref = EXCLUDED.statement_ref,
                parser_receipt_ref = EXCLUDED.parser_receipt_ref,
                error_ref = EXCLUDED.error_ref,
                source_region_preserved = TRUE,
                semantic_authority_created = FALSE,
                claim_truth_promoted = FALSE",
            &[
                &compilation.source_revision_ref,
                &assignment.region_ref,
                &disposition,
                &statement_ref,
                &parser_receipt_ref,
                &error_ref,
            ],
        )?;
    }

    tx.execute(
        "INSERT INTO ingest.lossless_compilation_receipt (
            source_revision_ref, source_ref, exact_region_count,
            semantic_candidate_region_count, compiled_statement_count,
            parser_residual_count, candidate_pnf_count,
            transport_or_nonsemantic_region_count, source_region_accounted_count,
            source_region_loss_count, candidate_only,
            creates_semantic_authority, applicability_promoted,
            claim_truth_promoted, parse_failure_deletes_source
         ) VALUES (
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE,FALSE,FALSE,FALSE,FALSE
         )
         ON CONFLICT (source_revision_ref) DO UPDATE SET
            source_ref = EXCLUDED.source_ref,
            exact_region_count = EXCLUDED.exact_region_count,
            semantic_candidate_region_count = EXCLUDED.semantic_candidate_region_count,
            compiled_statement_count = EXCLUDED.compiled_statement_count,
            parser_residual_count = EXCLUDED.parser_residual_count,
            candidate_pnf_count = EXCLUDED.candidate_pnf_count,
            transport_or_nonsemantic_region_count =
              EXCLUDED.transport_or_nonsemantic_region_count,
            source_region_accounted_count = EXCLUDED.source_region_accounted_count,
            source_region_loss_count = EXCLUDED.source_region_loss_count,
            candidate_only = TRUE,
            creates_semantic_authority = FALSE,
            applicability_promoted = FALSE,
            claim_truth_promoted = FALSE,
            parse_failure_deletes_source = FALSE",
        &[
            &compilation.source_revision_ref,
            &compilation.source_ref,
            &(compilation.exact_region_count as i64),
            &(compilation.semantic_candidate_region_count as i64),
            &(compilation.compiled_statement_count as i64),
            &(compilation.residuals.len() as i64),
            &(compilation.candidate_pnf_count as i64),
            &(compilation.transport_or_nonsemantic_region_count as i64),
            &(compilation.source_region_accounted_count as i64),
            &(compilation.source_region_loss_count as i64),
        ],
    )?;
    tx.commit()?;

    load_long_document_ingest_receipt(config, &compilation.source_revision_ref)
}


pub fn load_long_document_regions(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<Vec<PersistedLongDocumentRegion>, LongDocumentIngestStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;

    let rows = client.query(
        "SELECT
           r.source_ref,
           r.region_ref,
           r.parent_region_ref,
           r.region_kind,
           r.start_char,
           r.end_char,
           a.disposition,
           a.statement_ref,
           a.parser_receipt_ref,
           a.error_ref,
           a.source_region_preserved,
           a.semantic_authority_created,
           a.claim_truth_promoted
         FROM ingest.long_document_region r
         JOIN ingest.region_compilation_assignment a
           ON a.source_revision_ref = r.source_revision_ref
          AND a.region_ref = r.region_ref
         WHERE r.source_revision_ref = $1
         ORDER BY r.start_char, r.end_char, r.region_ref",
        &[&source_revision_ref],
    )?;

    rows.into_iter()
        .map(|row| {
            let disposition_ref: String = row.get(6);
            let statement_ref: Option<String> = row.get(7);
            let parser_receipt_ref: Option<String> = row.get(8);
            let error_ref: Option<String> = row.get(9);

            let disposition = match disposition_ref.as_str() {
                "compiled_candidate" => RegionCompilationDisposition::CompiledCandidate {
                    statement_ref: statement_ref.ok_or(
                        LongDocumentIngestStoreError::ReceiptRoundTripMismatch,
                    )?,
                },
                "parser_residual" => RegionCompilationDisposition::ParserResidual {
                    parser_receipt_ref: parser_receipt_ref.ok_or(
                        LongDocumentIngestStoreError::ReceiptRoundTripMismatch,
                    )?,
                    error_ref: error_ref.ok_or(
                        LongDocumentIngestStoreError::ReceiptRoundTripMismatch,
                    )?,
                },
                "transport_only" => RegionCompilationDisposition::TransportOnly,
                "structural_only" => RegionCompilationDisposition::StructuralOnly,
                _ => return Err(LongDocumentIngestStoreError::ReceiptRoundTripMismatch),
            };

            let start_char = row.get::<_, i64>(4);
            let end_char = row.get::<_, i64>(5);
            if start_char < 0 || end_char < 0 {
                return Err(LongDocumentIngestStoreError::ReceiptRoundTripMismatch);
            }

            let region = PersistedLongDocumentRegion {
                source_revision_ref: source_revision_ref.to_owned(),
                source_ref: row.get(0),
                region_ref: row.get(1),
                parent_region_ref: row.get(2),
                region_kind: row.get(3),
                start_char: start_char as u64,
                end_char: end_char as u64,
                disposition,
                source_region_preserved: row.get(10),
                semantic_authority_created: row.get(11),
                claim_truth_promoted: row.get(12),
            };

            if !region.source_region_preserved
                || region.semantic_authority_created
                || region.claim_truth_promoted
            {
                return Err(LongDocumentIngestStoreError::InvalidAssignment(
                    region.region_ref.clone(),
                ));
            }

            Ok(region)
        })
        .collect()
}

pub fn load_long_document_ingest_receipt(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<PersistedLongDocumentIngestReceipt, LongDocumentIngestStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;

    let row = client.query_one(
        "SELECT source_ref, exact_region_count, compiled_statement_count,
                parser_residual_count, source_region_loss_count,
                candidate_only, creates_semantic_authority,
                applicability_promoted, claim_truth_promoted,
                parse_failure_deletes_source
         FROM ingest.lossless_compilation_receipt
         WHERE source_revision_ref = $1",
        &[&source_revision_ref],
    )?;

    let assignment_rows = client.query(
        "SELECT disposition, COUNT(*)::BIGINT
         FROM ingest.region_compilation_assignment
         WHERE source_revision_ref = $1
         GROUP BY disposition",
        &[&source_revision_ref],
    )?;

    let counts = assignment_rows
        .into_iter()
        .map(|row| (row.get::<_, String>(0), row.get::<_, i64>(1) as usize))
        .collect::<BTreeMap<_, _>>();

    let receipt = PersistedLongDocumentIngestReceipt {
        source_ref: row.get(0),
        source_revision_ref: source_revision_ref.to_owned(),
        region_count: row.get::<_, i64>(1) as usize,
        assignment_count: counts.values().sum(),
        compiled_count: *counts.get("compiled_candidate").unwrap_or(&0),
        residual_count: *counts.get("parser_residual").unwrap_or(&0),
        structural_count: *counts.get("structural_only").unwrap_or(&0),
        transport_count: *counts.get("transport_only").unwrap_or(&0),
        source_region_loss_count: row.get::<_, i64>(4) as usize,
        candidate_only: row.get(5),
        creates_semantic_authority: row.get(6),
        applicability_promoted: row.get(7),
        claim_truth_promoted: row.get(8),
        parse_failure_deletes_source: row.get(9),
    };

    let stored_compiled_count = row.get::<_, i64>(2) as usize;
    let stored_residual_count = row.get::<_, i64>(3) as usize;
    if receipt.assignment_count != receipt.region_count
        || receipt.compiled_count != stored_compiled_count
        || receipt.residual_count != stored_residual_count
        || receipt.source_region_loss_count != 0
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
        || receipt.parse_failure_deletes_source
    {
        return Err(LongDocumentIngestStoreError::ReceiptRoundTripMismatch);
    }

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::source_ingest::{
        DocumentRegion, IngestRoleClass, SourceFamily, SourceIngestEnvelope,
    };

    fn document() -> LongDocumentSource {
        LongDocumentSource {
            ingest: SourceIngestEnvelope {
                source_ref: "book:fixture".into(),
                source_revision_ref: "revision:fixture".into(),
                provider_ref: "plain-text".into(),
                family: SourceFamily::Document,
                role_class: IngestRoleClass::ContentSource,
                content_digest_ref: "sha256:fixture".into(),
                acquisition_receipt_ref: "receipt:fixture".into(),
                media_type_ref: "text/plain".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            title: Some("Fixture".into()),
            edition_ref: None,
            regions: vec![
                DocumentRegion {
                    region_ref: "chapter:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: None,
                    kind: DocumentRegionKind::Chapter,
                    start_char: 0,
                    end_char: 20,
                },
                DocumentRegion {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    parent_region_ref: Some("chapter:1".into()),
                    kind: DocumentRegionKind::Sentence,
                    start_char: 0,
                    end_char: 10,
                },
            ],
        }
    }

    fn compilation() -> LosslessBulkSourceCompilation {
        LosslessBulkSourceCompilation {
            source_ref: "book:fixture".into(),
            source_revision_ref: "revision:fixture".into(),
            exact_region_count: 2,
            semantic_candidate_region_count: 1,
            compiled_statement_count: 1,
            candidate_pnf_count: 1,
            residuals: vec![],
            assignments: vec![
                crate::RegionCompilationAssignment {
                    region_ref: "chapter:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    disposition: RegionCompilationDisposition::StructuralOnly,
                    source_region_preserved: true,
                    semantic_authority_created: false,
                    claim_truth_promoted: false,
                },
                crate::RegionCompilationAssignment {
                    region_ref: "sentence:1".into(),
                    source_revision_ref: "revision:fixture".into(),
                    disposition: RegionCompilationDisposition::CompiledCandidate {
                        statement_ref: "statement:1".into(),
                    },
                    source_region_preserved: true,
                    semantic_authority_created: false,
                    claim_truth_promoted: false,
                },
            ],
            transport_or_nonsemantic_region_count: 1,
            source_region_accounted_count: 2,
            source_region_loss_count: 0,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            parse_failure_deletes_source: false,
        }
    }


    #[test]
    fn persisted_region_type_retains_exact_partition_disposition() {
        let region = PersistedLongDocumentRegion {
            source_revision_ref: "revision:fixture".into(),
            source_ref: "book:fixture".into(),
            region_ref: "sentence:1".into(),
            parent_region_ref: Some("chapter:1".into()),
            region_kind: "sentence".into(),
            start_char: 0,
            end_char: 10,
            disposition: RegionCompilationDisposition::CompiledCandidate {
                statement_ref: "statement:1".into(),
            },
            source_region_preserved: true,
            semantic_authority_created: false,
            claim_truth_promoted: false,
        };
        assert!(matches!(
            region.disposition,
            RegionCompilationDisposition::CompiledCandidate { .. }
        ));
        assert!(region.source_region_preserved);
        assert!(!region.semantic_authority_created);
        assert!(!region.claim_truth_promoted);
    }

    #[test]
    fn exact_partition_is_required_before_persistence() {
        assert!(validate_partition(&document(), &compilation()).is_ok());

        let mut broken = compilation();
        broken.assignments.pop();
        assert!(matches!(
            validate_partition(&document(), &broken),
            Err(LongDocumentIngestStoreError::IncompleteCoverage)
        ));
    }
}
