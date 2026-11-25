//! INGEST-1A durable long-document structural/compilation receipts.
//!
//! The canonical source text/revision remains owned by the existing source
//! substrate. This store persists the book-scale structural index and the
//! exhaustive region compilation partition so a restart can audit exactly what
//! was compiled, what failed parsing, and what remained structural.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls, GenericClient};
use sha2::{Digest, Sha256};
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

-- The original tables remain the historical legacy partition. New
-- partitions have their own receipts and assignments, with no legacy
-- receipt overwrite or interpretation as current coverage.
CREATE TABLE IF NOT EXISTS ingest.long_document_partition_revision (
    source_revision_ref TEXT NOT NULL,
    partition_ref TEXT NOT NULL,
    previous_partition_ref TEXT,
    source_digest_ref TEXT NOT NULL,
    exact_region_count BIGINT NOT NULL CHECK (exact_region_count >= 0),
    added_region_refs TEXT[] NOT NULL,
    PRIMARY KEY (source_revision_ref, partition_ref)
);
CREATE TABLE IF NOT EXISTS ingest.long_document_partition_head (
    source_revision_ref TEXT PRIMARY KEY,
    partition_ref TEXT NOT NULL,
    FOREIGN KEY (source_revision_ref, partition_ref)
      REFERENCES ingest.long_document_partition_revision(source_revision_ref, partition_ref)
);
CREATE TABLE IF NOT EXISTS ingest.partition_region_compilation_assignment (
    LIKE ingest.region_compilation_assignment INCLUDING DEFAULTS,
    partition_ref TEXT NOT NULL,
    PRIMARY KEY (source_revision_ref, partition_ref, region_ref),
    FOREIGN KEY (source_revision_ref, partition_ref)
      REFERENCES ingest.long_document_partition_revision(source_revision_ref, partition_ref),
    FOREIGN KEY (source_revision_ref, region_ref)
      REFERENCES ingest.long_document_region(source_revision_ref, region_ref)
);
CREATE TABLE IF NOT EXISTS ingest.partition_lossless_compilation_receipt (
    LIKE ingest.lossless_compilation_receipt INCLUDING DEFAULTS,
    partition_ref TEXT NOT NULL,
    PRIMARY KEY (source_revision_ref, partition_ref),
    FOREIGN KEY (source_revision_ref, partition_ref)
      REFERENCES ingest.long_document_partition_revision(source_revision_ref, partition_ref)
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
    #[error("persisted long-document structure differs from supplied canonical structure")]
    StoredStructureMismatch,
    #[error("partition revision changed or current partition is not yet compiled")]
    PartitionNotCurrent,
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLongDocumentStructureRegion {
    pub source_revision_ref: String,
    pub source_ref: String,
    pub region_ref: String,
    pub parent_region_ref: Option<String>,
    pub region_kind: String,
    pub start_char: u64,
    pub end_char: u64,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
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
    /// None denotes the original immutable legacy partition.
    pub partition_ref: Option<String>,
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


/// Stable partition identity over source revision and exact structural tuples,
/// not merely source bytes or the number of regions. Field lengths make the
/// digest unambiguous; region ordering is canonical and independent of IO.
fn partition_ref(document: &LongDocumentSource) -> String {
    let mut h = Sha256::new();
    fn field(h: &mut Sha256, s: &str) {
        h.update((s.len() as u64).to_be_bytes());
        h.update(s.as_bytes());
    }
    field(&mut h, "slr-long-document-partition:v1");
    field(&mut h, &document.ingest.source_revision_ref);
    field(&mut h, &document.ingest.content_digest_ref);
    let mut regions = document.regions.iter().collect::<Vec<_>>();
    regions.sort_by(|a,b| a.region_ref.cmp(&b.region_ref));
    for r in regions {
        field(&mut h, &r.region_ref);
        field(&mut h, if r.parent_region_ref.is_some() { "parent" } else { "root" });
        field(&mut h, r.parent_region_ref.as_deref().unwrap_or(""));
        field(&mut h, region_kind_db(r.kind));
        h.update(r.start_char.to_be_bytes());
        h.update(r.end_char.to_be_bytes());
    }
    format!("long-document-partition:sha256:{:x}", h.finalize())
}

fn lock_partition(client: &mut impl GenericClient, revision: &str)
    -> Result<(), LongDocumentIngestStoreError>
{
    client.query_one("SELECT pg_advisory_xact_lock(hashtext('slr-long-document-partition:v1'), hashtext($1))", &[&revision])?;
    Ok(())
}

fn active_partition(client: &mut impl GenericClient, revision: &str)
    -> Result<Option<String>, LongDocumentIngestStoreError>
{
    Ok(client.query_opt("SELECT partition_ref FROM ingest.long_document_partition_head WHERE source_revision_ref=$1",
        &[&revision])?.map(|row| row.get(0)))
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct HeadingPartitionPublication {
    pub source_revision_ref: String,
    pub source_digest_ref: String,
    pub previous_partition_ref: String,
    pub partition_ref: String,
    pub added_region_refs: Vec<String>,
    pub exact_region_count: usize,
    pub candidate_only: bool,
    pub typed_coverage_paid: bool,
}

/// Publish a checked additive repair into the existing structural ledger.
/// Explicitly called by a future integrated prefix runner, not by default
/// legacy ingestion. Publication and partition head are one transaction.
/// Caller must complete parser-job and consumer invalidation welds before
/// using this as a production campaign.
pub(crate) fn publish_heading_partition(
    config: &DatabaseConfig,
    legacy: &LongDocumentSource,
    canonical_text: &str,
    selected_paragraph_refs: &[String],
) -> Result<HeadingPartitionPublication, LongDocumentIngestStoreError> {
    let revision = &legacy.ingest.source_revision_ref;
    let mut db = Client::connect(config.database_url(), NoTls)?;
    db.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;
    let mut tx = db.transaction()?;
    tx.batch_execute("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE; SET LOCAL lock_timeout='5000ms'; SET LOCAL statement_timeout='90000ms'")?;
    lock_partition(&mut tx, revision)?;
    // The canonical bytes must be reopened from the original source owner,
    // not supplied as an independent manifestation by the caller.
    let row = tx.query_one("SELECT c.payload,g.content_digest_ref FROM ingest.generic_source_revision g
        JOIN corpus.document d ON d.document_ref=g.document_ref
        JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
        WHERE g.source_revision_ref=$1 AND g.source_ref=$2 AND g.candidate_only
          AND NOT g.creates_semantic_authority AND NOT g.applicability_promoted AND NOT g.claim_truth_promoted",
        &[&revision, &legacy.ingest.source_ref])?;
    let bytes: Vec<u8> = row.get(0);
    let digest: String = row.get(1);
    if bytes.as_slice() != canonical_text.as_bytes() || digest != legacy.ingest.content_digest_ref {
        return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
    }
    let stored = load_structure_with_client(&mut tx, revision)?;
    let before = reopen_heading_repaired_structure(legacy, canonical_text, &stored)?;
    let previous = partition_ref(&before);
    let head = active_partition(&mut tx, revision)?;
    if head.as_ref().is_some_and(|r| r != &previous)
        || (head.is_none() && before.regions.len() != legacy.regions.len()) {
        return Err(LongDocumentIngestStoreError::PartitionNotCurrent);
    }
    let (after, added) = crate::plain_text_document_adapter::extend_plain_text_heading_exclusions(
        &before, canonical_text, selected_paragraph_refs,
    ).map_err(|_| LongDocumentIngestStoreError::StoredStructureMismatch)?;
    let next = partition_ref(&after);
    let added_refs = added.iter().map(|r| r.region_ref.clone()).collect::<Vec<_>>();
    if !added.is_empty() {
        if head.is_none() {
            tx.execute("INSERT INTO ingest.long_document_partition_revision
                (source_revision_ref,partition_ref,previous_partition_ref,source_digest_ref,exact_region_count,added_region_refs)
                VALUES($1,$2,NULL,$3,$4,ARRAY[]::text[])",
                &[&revision,&previous,&digest,&(before.regions.len() as i64)])?;
        }
        for r in &added {
            tx.execute("INSERT INTO ingest.long_document_region
                (source_revision_ref,source_ref,region_ref,parent_region_ref,region_kind,start_char,end_char,
                 candidate_only,creates_semantic_authority,claim_truth_promoted)
                VALUES($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE)",
                &[&revision,&legacy.ingest.source_ref,&r.region_ref,&r.parent_region_ref,
                  &region_kind_db(r.kind),&(r.start_char as i64),&(r.end_char as i64)])?;
        }
        tx.execute("INSERT INTO ingest.long_document_partition_revision
            (source_revision_ref,partition_ref,previous_partition_ref,source_digest_ref,exact_region_count,added_region_refs)
            VALUES($1,$2,$3,$4,$5,$6)",
            &[&revision,&next,&previous,&digest,&(after.regions.len() as i64),&added_refs])?;
        tx.execute("INSERT INTO ingest.long_document_partition_head(source_revision_ref,partition_ref)
            VALUES($1,$2) ON CONFLICT(source_revision_ref) DO UPDATE SET partition_ref=EXCLUDED.partition_ref",
            &[&revision,&next])?;
    }
    tx.commit()?;
    Ok(HeadingPartitionPublication {
        source_revision_ref: revision.clone(), source_digest_ref:digest,
        previous_partition_ref:previous, partition_ref:next,
        added_region_refs:added_refs, exact_region_count:after.regions.len(),
        candidate_only:true, typed_coverage_paid:false,
    })
}

pub fn persist_long_document_structure(
    config: &DatabaseConfig,
    document: &LongDocumentSource,
) -> Result<usize, LongDocumentIngestStoreError> {
    document
        .validate()
        .map_err(LongDocumentIngestStoreError::InvalidDocument)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;
    lock_partition(&mut tx, &document.ingest.source_revision_ref)?;
    // Legacy preparation cannot append arbitrary rows to a repaired partition.
    if active_partition(&mut tx, &document.ingest.source_revision_ref)?.is_some() {
        return Err(LongDocumentIngestStoreError::PartitionNotCurrent);
    }

    let mut persisted = 0usize;
    for region in &document.regions {
        persisted += tx.execute(
            "INSERT INTO ingest.long_document_region (
                source_revision_ref, source_ref, region_ref, parent_region_ref,
                region_kind, start_char, end_char, candidate_only,
                creates_semantic_authority, claim_truth_promoted
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,FALSE)
             ON CONFLICT (source_revision_ref, region_ref) DO NOTHING",
            &[
                &document.ingest.source_revision_ref,
                &document.ingest.source_ref,
                &region.region_ref,
                &region.parent_region_ref,
                &region_kind_db(region.kind),
                &(region.start_char as i64),
                &(region.end_char as i64),
            ],
        )? as usize;
    }
    tx.commit()?;
    Ok(persisted)
}


pub fn load_long_document_structure(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<Vec<PersistedLongDocumentStructureRegion>, LongDocumentIngestStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;
    load_structure_with_client(&mut client, source_revision_ref)
}

fn load_structure_with_client(
    client: &mut impl GenericClient, source_revision_ref: &str,
) -> Result<Vec<PersistedLongDocumentStructureRegion>, LongDocumentIngestStoreError> {
    let rows = client.query(
        "SELECT source_ref, region_ref, parent_region_ref, region_kind,
                start_char, end_char, candidate_only,
                creates_semantic_authority, claim_truth_promoted
         FROM ingest.long_document_region
         WHERE source_revision_ref = $1
         ORDER BY region_ref",
        &[&source_revision_ref],
    )?;

    rows.into_iter()
        .map(|row| {
            let start = row.get::<_, i64>(4);
            let end = row.get::<_, i64>(5);
            if start < 0 || end < 0 {
                return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
            }
            let region = PersistedLongDocumentStructureRegion {
                source_revision_ref: source_revision_ref.to_owned(),
                source_ref: row.get(0),
                region_ref: row.get(1),
                parent_region_ref: row.get(2),
                region_kind: row.get(3),
                start_char: start as u64,
                end_char: end as u64,
                candidate_only: row.get(6),
                creates_semantic_authority: row.get(7),
                claim_truth_promoted: row.get(8),
            };
            if !region.candidate_only
                || region.creates_semantic_authority
                || region.claim_truth_promoted
            {
                return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
            }
            Ok(region)
        })
        .collect()
}

/// Reconstruct only the explicitly published v1 heading-repair extensions.
/// The original full structure remains the authority for all legacy IDs;
/// neither arbitrary extra rows nor a partially published paragraph are valid.
/// Read-only; this is not a publication or dependency invalidation API.
pub(crate) fn reopen_heading_repaired_structure(
    legacy: &LongDocumentSource,
    canonical_text: &str,
    stored: &[PersistedLongDocumentStructureRegion],
) -> Result<LongDocumentSource, LongDocumentIngestStoreError> {
    let legacy_refs = legacy.regions.iter()
        .map(|r| r.region_ref.as_str()).collect::<BTreeSet<_>>();
    let mut parents = BTreeSet::new();
    for row in stored {
        if row.source_revision_ref != legacy.ingest.source_revision_ref
            || row.source_ref != legacy.ingest.source_ref
            || !row.candidate_only || row.creates_semantic_authority || row.claim_truth_promoted
        {
            return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
        }
        if !legacy_refs.contains(row.region_ref.as_str()) {
            let prefix = format!("document-region:{}:sentence:heading-repair-v1:",
                legacy.ingest.source_revision_ref);
            if row.region_kind != "sentence" || !row.region_ref.starts_with(&prefix) {
                return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
            }
            parents.insert(row.parent_region_ref.clone()
                .ok_or(LongDocumentIngestStoreError::StoredStructureMismatch)?);
        }
    }
    let (extended, _) = crate::plain_text_document_adapter::extend_plain_text_heading_exclusions(
        legacy, canonical_text, &parents.into_iter().collect::<Vec<_>>(),
    ).map_err(|_| LongDocumentIngestStoreError::StoredStructureMismatch)?;
    let expected = extended.regions.iter().map(|r| (
        r.region_ref.as_str(), r.parent_region_ref.as_deref(),
        region_kind_db(r.kind), r.start_char, r.end_char
    )).collect::<BTreeSet<_>>();
    let actual = stored.iter().map(|r| (
        r.region_ref.as_str(), r.parent_region_ref.as_deref(),
        r.region_kind.as_str(), r.start_char, r.end_char
    )).collect::<BTreeSet<_>>();
    if expected != actual || actual.len() != stored.len() {
        return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
    }
    Ok(extended)
}

fn ensure_long_document_structure_matches(
    config: &DatabaseConfig,
    document: &LongDocumentSource,
) -> Result<(), LongDocumentIngestStoreError> {
    let stored = load_long_document_structure(config, &document.ingest.source_revision_ref)?;
    if stored.is_empty() {
        persist_long_document_structure(config, document)?;
        return Ok(());
    }

    if stored.len() != document.regions.len() {
        return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
    }
    let supplied = document
        .regions
        .iter()
        .map(|region| {
            (
                region.region_ref.as_str(),
                region.parent_region_ref.as_deref(),
                region_kind_db(region.kind),
                region.start_char,
                region.end_char,
            )
        })
        .collect::<BTreeSet<_>>();
    let persisted = stored
        .iter()
        .map(|region| {
            (
                region.region_ref.as_str(),
                region.parent_region_ref.as_deref(),
                region.region_kind.as_str(),
                region.start_char,
                region.end_char,
            )
        })
        .collect::<BTreeSet<_>>();

    if supplied != persisted
        || stored
            .iter()
            .any(|region| region.source_ref != document.ingest.source_ref)
    {
        return Err(LongDocumentIngestStoreError::StoredStructureMismatch);
    }
    Ok(())
}

pub fn persist_long_document_compilation(
    config: &DatabaseConfig,
    document: &LongDocumentSource,
    compilation: &LosslessBulkSourceCompilation,
) -> Result<PersistedLongDocumentIngestReceipt, LongDocumentIngestStoreError> {
    validate_partition(document, compilation)?;

    ensure_long_document_structure_matches(config, document)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;
    lock_partition(&mut tx, &compilation.source_revision_ref)?;
    let current = active_partition(&mut tx, &compilation.source_revision_ref)?;
    let expected = partition_ref(document);
    if current.as_ref().is_some_and(|r| r != &expected) {
        return Err(LongDocumentIngestStoreError::PartitionNotCurrent);
    }
    let stored_count: i64 = tx.query_one("SELECT count(*)::bigint FROM ingest.long_document_region WHERE source_revision_ref=$1",
        &[&compilation.source_revision_ref])?.get(0);
    if stored_count != document.regions.len() as i64 {
        return Err(LongDocumentIngestStoreError::PartitionNotCurrent);
    }

    for assignment in &compilation.assignments {
        let (disposition, statement_ref, parser_receipt_ref, error_ref) =
            disposition_parts(&assignment.disposition);
        let legacy_sql = "INSERT INTO ingest.region_compilation_assignment (
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
                claim_truth_promoted = FALSE";
        let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = vec![
            &compilation.source_revision_ref, &assignment.region_ref,
            &disposition, &statement_ref, &parser_receipt_ref, &error_ref,
        ];
        if let Some(partition) = &current {
            let sql = legacy_sql.replace("ingest.region_compilation_assignment", "ingest.partition_region_compilation_assignment")
                .replace("semantic_authority_created, claim_truth_promoted", "semantic_authority_created, claim_truth_promoted, partition_ref")
                .replace("TRUE,FALSE,FALSE)", "TRUE,FALSE,FALSE,$7)")
                .replace("ON CONFLICT (source_revision_ref, region_ref)", "ON CONFLICT (source_revision_ref, partition_ref, region_ref)");
            params.push(partition);
            tx.execute(sql.as_str(), &params)?;
        } else {
            tx.execute(legacy_sql, &params)?;
        }
    }

    let legacy_sql = "INSERT INTO ingest.lossless_compilation_receipt (
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
            parse_failure_deletes_source = FALSE";
    let exact_count = compilation.exact_region_count as i64;
    let semantic_count = compilation.semantic_candidate_region_count as i64;
    let compiled_count = compilation.compiled_statement_count as i64;
    let residual_count = compilation.residuals.len() as i64;
    let pnf_count = compilation.candidate_pnf_count as i64;
    let transport_count = compilation.transport_or_nonsemantic_region_count as i64;
    let accounted_count = compilation.source_region_accounted_count as i64;
    let loss_count = compilation.source_region_loss_count as i64;
    let mut params: Vec<&(dyn postgres::types::ToSql + Sync)> = vec![
        &compilation.source_revision_ref, &compilation.source_ref,
        &exact_count, &semantic_count, &compiled_count, &residual_count,
        &pnf_count, &transport_count, &accounted_count, &loss_count,
    ];
    if let Some(partition) = &current {
        let sql = legacy_sql.replace("ingest.lossless_compilation_receipt", "ingest.partition_lossless_compilation_receipt")
            .replace("claim_truth_promoted, parse_failure_deletes_source", "claim_truth_promoted, parse_failure_deletes_source, partition_ref")
            .replace("TRUE,FALSE,FALSE,FALSE,FALSE", "TRUE,FALSE,FALSE,FALSE,FALSE,$11")
            .replace("ON CONFLICT (source_revision_ref)", "ON CONFLICT (source_revision_ref, partition_ref)");
        params.push(partition);
        tx.execute(sql.as_str(), &params)?;
    } else {
        tx.execute(legacy_sql, &params)?;
    }
    tx.commit()?;

    load_long_document_ingest_receipt(config, &compilation.source_revision_ref)
}


pub fn load_long_document_regions(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<Vec<PersistedLongDocumentRegion>, LongDocumentIngestStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LONG_DOCUMENT_INGEST_SCHEMA_SQL)?;

    let mut tx = client.transaction()?;
    lock_partition(&mut tx, source_revision_ref)?;
    let current = active_partition(&mut tx, source_revision_ref)?;
    let legacy_sql = "SELECT
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
         ORDER BY r.start_char, r.end_char, r.region_ref";
    let rows = if let Some(partition) = &current {
        let sql = legacy_sql.replace("ingest.region_compilation_assignment", "ingest.partition_region_compilation_assignment")
            .replace("WHERE r.source_revision_ref = $1", "WHERE r.source_revision_ref = $1 AND a.partition_ref=$2");
        tx.query(sql.as_str(), &[&source_revision_ref, partition])?
    } else {
        tx.query(legacy_sql, &[&source_revision_ref])?
    };
    let expected: i64 = tx.query_one("SELECT count(*)::bigint FROM ingest.long_document_region WHERE source_revision_ref=$1",
        &[&source_revision_ref])?.get(0);
    if current.is_some() && rows.len() as i64 != expected {
        return Err(LongDocumentIngestStoreError::PartitionNotCurrent);
    }
    tx.commit()?;

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

    let mut tx = client.transaction()?;
    lock_partition(&mut tx, source_revision_ref)?;
    let current = active_partition(&mut tx, source_revision_ref)?;
    let legacy_sql = "SELECT source_ref, exact_region_count, compiled_statement_count,
                parser_residual_count, source_region_loss_count,
                candidate_only, creates_semantic_authority,
                applicability_promoted, claim_truth_promoted,
                parse_failure_deletes_source
         FROM ingest.lossless_compilation_receipt
         WHERE source_revision_ref = $1";
    let legacy_assignments = "SELECT disposition, COUNT(*)::BIGINT
         FROM ingest.region_compilation_assignment
         WHERE source_revision_ref = $1
         GROUP BY disposition";
    let (row, assignment_rows) = if let Some(partition) = &current {
        let sql = legacy_sql.replace("ingest.lossless_compilation_receipt", "ingest.partition_lossless_compilation_receipt")
            + " AND partition_ref=$2";
        let row = tx.query_opt(sql.as_str(), &[&source_revision_ref, partition])?
            .ok_or(LongDocumentIngestStoreError::PartitionNotCurrent)?;
        let sql = legacy_assignments.replace("ingest.region_compilation_assignment", "ingest.partition_region_compilation_assignment")
            .replace("GROUP BY disposition", "AND partition_ref=$2 GROUP BY disposition");
        (row, tx.query(sql.as_str(), &[&source_revision_ref, partition])?)
    } else {
        (tx.query_one(legacy_sql, &[&source_revision_ref])?,
         tx.query(legacy_assignments, &[&source_revision_ref])?)
    };
    let expected: i64 = tx.query_one("SELECT count(*)::bigint FROM ingest.long_document_region WHERE source_revision_ref=$1",
        &[&source_revision_ref])?.get(0);
    if row.get::<_,i64>(1) != expected {
        return Err(LongDocumentIngestStoreError::PartitionNotCurrent);
    }
    tx.commit()?;

    let counts = assignment_rows
        .into_iter()
        .map(|row| (row.get::<_, String>(0), row.get::<_, i64>(1) as usize))
        .collect::<BTreeMap<_, _>>();

    let receipt = PersistedLongDocumentIngestReceipt {
        partition_ref: current,
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
    use crate::{
        CandidatePnfBatch, CandidatePnfFactor, CandidatePnfRole, ExactSourceSpan,
        SourceStatementEnvelope, StatementCandidatePnf, StatementOrigin,
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
            compiled: vec![StatementCandidatePnf {
                statement: SourceStatementEnvelope {
                    statement_ref: "statement:1".into(),
                    document_ref: "book:fixture".into(),
                    source_revision_ref: "revision:fixture".into(),
                    span: ExactSourceSpan {
                        span_ref: "sentence:1".into(), start_char: 0, end_char: 10,
                    },
                    literal_text: "fixture text".into(),
                    origin: StatementOrigin::InitialIntake,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                },
                pnf: CandidatePnfBatch {
                    exact_span_ref: "sentence:1".into(),
                    candidates: vec![CandidatePnfFactor {
                        candidate_ref: "candidate:1".into(),
                        role: CandidatePnfRole::Other,
                        source_start_char: 0,
                        source_end_char: 10,
                        surface: "fixture text".into(),
                        lemma: "fixture".into(),
                        dependency_ref: "dep:root".into(),
                        candidate_only: true,
                    }],
                    proposition_support_paid: false,
                    applicability_paid: false,
                    claim_truth_paid: false,
                },
                parser_receipt_ref: "parser:1".into(),
                candidate_only: true,
                semantic_admission_paid: false,
                proposition_support_paid: false,
                applicability_paid: false,
                claim_truth_paid: false,
            }],
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

#[cfg(test)]
mod heading_repair_reopen_tests {
    use super::*;
    use crate::{build_plain_text_long_document_source, extend_plain_text_heading_exclusions};

    fn rows(doc: &LongDocumentSource) -> Vec<PersistedLongDocumentStructureRegion> {
        doc.regions.iter().map(|r| PersistedLongDocumentStructureRegion {
            source_revision_ref: r.source_revision_ref.clone(),
            source_ref: doc.ingest.source_ref.clone(),
            region_ref: r.region_ref.clone(),
            parent_region_ref: r.parent_region_ref.clone(),
            region_kind: region_kind_db(r.kind).into(),
            start_char: r.start_char,
            end_char: r.end_char,
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        }).collect()
    }

    #[test]
    fn partition_identity_distinguishes_extension_and_is_order_independent() {
        let text = "Chapter One\nFirst. Second.\n\nLater.";
        let (legacy, _) = build_plain_text_long_document_source(
            "book:fixture", "revision:fixture", "plain", "receipt:fixture",
            None, None, text,
        ).unwrap();
        let parent = legacy.regions.iter()
            .find(|r| r.kind == DocumentRegionKind::Paragraph).unwrap();
        let (extended, _) = extend_plain_text_heading_exclusions(
            &legacy, text, &[parent.region_ref.clone()],
        ).unwrap();
        assert_ne!(partition_ref(&legacy), partition_ref(&extended));
        assert_eq!(legacy.ingest.content_digest_ref, extended.ingest.content_digest_ref);
        let mut reordered = extended.clone();
        reordered.regions.reverse();
        assert_eq!(partition_ref(&extended), partition_ref(&reordered));
        reordered.regions[0].start_char += 1;
        assert_ne!(partition_ref(&extended), partition_ref(&reordered));
    }

    #[test]
    fn exact_repair_reopens_but_partial_or_forged_partition_fails() {
        let text = "Chapter One\nFirst. Second.\n\nLater.";
        let (legacy, _) = build_plain_text_long_document_source(
            "book:fixture", "revision:fixture", "plain", "receipt:fixture",
            None, None, text,
        ).unwrap();
        let parent = legacy.regions.iter()
            .find(|r| r.kind == DocumentRegionKind::Paragraph).unwrap();
        let (extended, added) = extend_plain_text_heading_exclusions(
            &legacy, text, &[parent.region_ref.clone()],
        ).unwrap();
        assert_eq!(added.len(), 3);
        let original_rows = rows(&legacy);
        assert_eq!(reopen_heading_repaired_structure(&legacy, text, &original_rows).unwrap(), legacy);
        let exact = rows(&extended);
        assert_eq!(reopen_heading_repaired_structure(&legacy, text, &exact).unwrap(), extended);

        let mut partial = exact.clone();
        partial.pop();
        assert!(reopen_heading_repaired_structure(&legacy, text, &partial).is_err());
        let mut foreign = exact.clone();
        foreign.last_mut().unwrap().parent_region_ref = Some("foreign".into());
        assert!(reopen_heading_repaired_structure(&legacy, text, &foreign).is_err());
        let mut shifted = exact.clone();
        shifted.last_mut().unwrap().start_char += 1;
        assert!(reopen_heading_repaired_structure(&legacy, text, &shifted).is_err());
        let mut duplicate = exact.clone();
        duplicate.push(exact.last().unwrap().clone());
        assert!(reopen_heading_repaired_structure(&legacy, text, &duplicate).is_err());
    }
}
