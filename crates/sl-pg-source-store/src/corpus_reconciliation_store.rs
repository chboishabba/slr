//! SCALE-1 L2 corpus reconciliation over durable candidate Statement/PNF rows.
//!
//! This layer runs automatically over every successfully compiled statement.
//! It creates *candidate* entity/proposition/event fingerprints and occurrence
//! clusters. Fingerprint equality is a scheduling/reconciliation signal only:
//! it does not create entity identity, proposition admission, event identity,
//! applicability, or truth.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub const CORPUS_RECONCILIATION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS semantic;

CREATE TABLE IF NOT EXISTS semantic.entity_mention_candidate (
    mention_ref TEXT PRIMARY KEY,
    entity_fingerprint_ref TEXT NOT NULL,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    candidate_ref TEXT NOT NULL,
    role_ref TEXT NOT NULL,
    surface TEXT NOT NULL,
    lemma TEXT NOT NULL,
    source_start_char BIGINT NOT NULL,
    source_end_char BIGINT NOT NULL,
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_entity_identity BOOLEAN NOT NULL CHECK (NOT creates_entity_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (statement_ref, candidate_ref, detector_ref)
);

CREATE INDEX IF NOT EXISTS entity_mention_fingerprint_idx
ON semantic.entity_mention_candidate(entity_fingerprint_ref, statement_ref);

CREATE TABLE IF NOT EXISTS semantic.named_entity_candidate (
    mention_ref TEXT PRIMARY KEY,
    entity_fingerprint_ref TEXT NOT NULL,
    parser_run_ref TEXT NOT NULL,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    exact_span_ref TEXT NOT NULL,
    start_char BIGINT NOT NULL,
    end_char BIGINT NOT NULL,
    surface TEXT NOT NULL,
    label_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_entity_identity BOOLEAN NOT NULL CHECK (NOT creates_entity_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (parser_run_ref, statement_ref, start_char, end_char, label_ref)
);

CREATE INDEX IF NOT EXISTS named_entity_fingerprint_idx
ON semantic.named_entity_candidate(entity_fingerprint_ref, statement_ref);

CREATE TABLE IF NOT EXISTS semantic.temporal_mention_candidate (
    temporal_mention_ref TEXT PRIMARY KEY,
    parser_run_ref TEXT NOT NULL,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    exact_span_ref TEXT NOT NULL,
    start_char BIGINT NOT NULL,
    end_char BIGINT NOT NULL,
    surface TEXT NOT NULL,
    label_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_temporal_assertion BOOLEAN NOT NULL CHECK (NOT creates_temporal_assertion),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (parser_run_ref, statement_ref, start_char, end_char, label_ref)
);

CREATE TABLE IF NOT EXISTS semantic.proposition_fingerprint_candidate (
    proposition_fingerprint_ref TEXT PRIMARY KEY,
    base_signature_ref TEXT NOT NULL,
    polarity_ref TEXT NOT NULL CHECK (polarity_ref IN ('positive','negative')),
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_proposition_identity BOOLEAN NOT NULL CHECK (NOT creates_proposition_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS semantic.proposition_candidate_occurrence (
    proposition_fingerprint_ref TEXT NOT NULL
      REFERENCES semantic.proposition_fingerprint_candidate(proposition_fingerprint_ref)
      ON DELETE CASCADE,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref) ON DELETE CASCADE,
    source_revision_ref TEXT NOT NULL,
    exact_span_ref TEXT NOT NULL,
    literal_text TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (proposition_fingerprint_ref, statement_ref, batch_ref)
);

CREATE INDEX IF NOT EXISTS proposition_candidate_source_idx
ON semantic.proposition_candidate_occurrence(source_revision_ref, exact_span_ref);

CREATE TABLE IF NOT EXISTS semantic.event_fingerprint_candidate (
    event_fingerprint_ref TEXT PRIMARY KEY,
    base_signature_ref TEXT NOT NULL,
    polarity_ref TEXT NOT NULL CHECK (polarity_ref IN ('positive','negative')),
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS semantic.event_candidate_occurrence (
    event_fingerprint_ref TEXT NOT NULL
      REFERENCES semantic.event_fingerprint_candidate(event_fingerprint_ref)
      ON DELETE CASCADE,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref) ON DELETE CASCADE,
    source_revision_ref TEXT NOT NULL,
    exact_span_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (event_fingerprint_ref, statement_ref, batch_ref)
);

CREATE TABLE IF NOT EXISTS semantic.contestation_candidate (
    relation_ref TEXT PRIMARY KEY,
    base_signature_ref TEXT NOT NULL,
    positive_proposition_fingerprint_ref TEXT NOT NULL,
    negative_proposition_fingerprint_ref TEXT NOT NULL,
    kind_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    requires_review BOOLEAN NOT NULL CHECK (requires_review),
    creates_contestation_identity BOOLEAN NOT NULL CHECK (NOT creates_contestation_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS semantic.reconciliation_pressure_candidate (
    pressure_ref TEXT PRIMARY KEY,
    semantic_kind_ref TEXT NOT NULL,
    semantic_fingerprint_ref TEXT NOT NULL,
    occurrence_count BIGINT NOT NULL,
    source_revision_count BIGINT NOT NULL,
    reason_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    requires_review BOOLEAN NOT NULL CHECK (requires_review),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS semantic.l2_candidate_product_summary (
    product_ref TEXT NOT NULL
      REFERENCES pnf.candidate_semantic_product(product_ref) ON DELETE CASCADE,
    detector_ref TEXT NOT NULL,
    factor_count BIGINT NOT NULL,
    entity_factor_count BIGINT NOT NULL,
    exact_reopen_validated BOOLEAN NOT NULL CHECK (exact_reopen_validated),
    base_signature_ref TEXT NULL,
    polarity_ref TEXT NULL CHECK (
      polarity_ref IS NULL OR polarity_ref IN ('positive','negative')
    ),
    creates_event_candidate BOOLEAN NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_entity_identity BOOLEAN NOT NULL CHECK (NOT creates_entity_identity),
    creates_proposition_identity BOOLEAN NOT NULL CHECK (NOT creates_proposition_identity),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (product_ref, detector_ref)
);

ALTER TABLE semantic.l2_candidate_product_summary
  ADD COLUMN IF NOT EXISTS entity_factor_count BIGINT NOT NULL DEFAULT 0;
ALTER TABLE semantic.l2_candidate_product_summary
  ADD COLUMN IF NOT EXISTS exact_reopen_validated BOOLEAN NOT NULL DEFAULT TRUE
  CHECK (exact_reopen_validated);

CREATE TABLE IF NOT EXISTS semantic.l2_candidate_product_entity_factor (
    product_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    factor_ordinal INTEGER NOT NULL,
    token_ordinal INTEGER NOT NULL,
    start_offset BIGINT NOT NULL,
    end_offset BIGINT NOT NULL,
    entity_fingerprint_ref TEXT NOT NULL,
    role_ref TEXT NOT NULL,
    surface TEXT NOT NULL,
    lemma TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_entity_identity BOOLEAN NOT NULL CHECK (NOT creates_entity_identity),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (product_ref, detector_ref, factor_ordinal),
    FOREIGN KEY (product_ref, detector_ref)
      REFERENCES semantic.l2_candidate_product_summary(product_ref, detector_ref)
      ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS l2_candidate_product_entity_fingerprint_idx
ON semantic.l2_candidate_product_entity_factor(entity_fingerprint_ref, product_ref);

CREATE TABLE IF NOT EXISTS semantic.corpus_reconciliation_stage_receipt (
    source_revision_ref TEXT NOT NULL,
    parser_run_ref TEXT NOT NULL,
    detector_ref TEXT NOT NULL,
    statement_count BIGINT NOT NULL,
    entity_mention_count BIGINT NOT NULL,
    entity_fingerprint_count BIGINT NOT NULL,
    named_entity_mention_count BIGINT NOT NULL,
    named_entity_fingerprint_count BIGINT NOT NULL,
    temporal_mention_count BIGINT NOT NULL,
    proposition_occurrence_count BIGINT NOT NULL,
    proposition_fingerprint_count BIGINT NOT NULL,
    event_occurrence_count BIGINT NOT NULL,
    event_fingerprint_count BIGINT NOT NULL,
    polarity_conflict_candidate_count BIGINT NOT NULL,
    review_pressure_candidate_count BIGINT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    creates_entity_identity BOOLEAN NOT NULL CHECK (NOT creates_entity_identity),
    creates_proposition_identity BOOLEAN NOT NULL CHECK (NOT creates_proposition_identity),
    creates_event_identity BOOLEAN NOT NULL CHECK (NOT creates_event_identity),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (source_revision_ref, parser_run_ref, detector_ref)
);

"#;

pub(crate) const CORPUS_RECONCILIATION_DETECTOR_REF: &str =
    "scale1:persistent-pnf-fingerprint:v1";
const DETECTOR_REF: &str = CORPUS_RECONCILIATION_DETECTOR_REF;

#[derive(Debug, Error)]
pub enum CorpusReconciliationError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("candidate PNF source rows violated expected non-promotion boundary")]
    PromotionBoundary,
    #[error("persisted reconciliation row conflicted with deterministic identity")]
    ExistingRowConflict,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CorpusReconciliationWork {
    pub factor_rows_scanned: usize,
    pub product_summary_reuse_hits: usize,
    pub product_summaries_created: usize,
    pub entity_mention_rows_inserted: usize,
    pub named_entity_rows_inserted: usize,
    pub temporal_rows_inserted: usize,
    pub proposition_fingerprint_rows_inserted: usize,
    pub proposition_occurrence_rows_inserted: usize,
    pub event_fingerprint_rows_inserted: usize,
    pub event_occurrence_rows_inserted: usize,
    pub contestation_rows_inserted: usize,
    pub pressure_rows_upserted: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusReconciliationReceipt {
    pub source_revision_ref: String,
    pub stage_reused: bool,
    pub work: CorpusReconciliationWork,
    pub statement_count: usize,
    pub entity_mention_count: usize,
    pub entity_fingerprint_count: usize,
    pub named_entity_mention_count: usize,
    pub named_entity_fingerprint_count: usize,
    pub temporal_mention_count: usize,
    pub proposition_occurrence_count: usize,
    pub proposition_fingerprint_count: usize,
    pub event_occurrence_count: usize,
    pub event_fingerprint_count: usize,
    pub polarity_conflict_candidate_count: usize,
    pub review_pressure_candidate_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_entity_identity: bool,
    pub creates_proposition_identity: bool,
    pub creates_event_identity: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone)]
struct FactorRow {
    candidate_ref: String,
    role_ref: String,
    start_char: i64,
    end_char: i64,
    surface: String,
    lemma: String,
    dependency_ref: String,
    candidate_only: bool,
}

#[derive(Debug, Clone)]
struct StatementBatch {
    batch_ref: String,
    statement_ref: String,
    source_revision_ref: String,
    exact_span_ref: String,
    literal_text: String,
    factors: Vec<FactorRow>,
}

fn digest_ref(domain: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in std::iter::once(domain).chain(parts.iter().copied()) {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn role_lemmas(batch: &StatementBatch, role: &str) -> Vec<String> {
    let mut values = batch
        .factors
        .iter()
        .filter(|factor| factor.role_ref == role)
        .map(|factor| normalize(&factor.lemma))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    values
}

fn negative(batch: &StatementBatch) -> bool {
    batch.factors.iter().any(|factor| {
        factor.dependency_ref.eq_ignore_ascii_case("neg")
            || matches!(normalize(&factor.lemma).as_str(), "not" | "never" | "n't")
    })
}

fn base_signature(batch: &StatementBatch) -> Option<String> {
    let actors = role_lemmas(batch, "actor");
    let predicates = role_lemmas(batch, "predicate");
    let patients = role_lemmas(batch, "patient");
    if predicates.is_empty() {
        return None;
    }
    Some(format!(
        "a=[{}]|p=[{}]|o=[{}]",
        actors.join(","),
        predicates.join(","),
        patients.join(",")
    ))
}

#[derive(Debug, Clone)]
struct ProductFactorRow {
    factor_ordinal: i32,
    token_ordinal: i32,
    start_offset: i64,
    end_offset: i64,
    role_ref: String,
    surface: String,
    lemma: String,
    dependency_ref: String,
}

fn ensure_l2_product_summaries(
    client: &mut Client,
    source_revision_ref: &str,
    work: &mut CorpusReconciliationWork,
) -> Result<(), CorpusReconciliationError> {
    let products = client.query(
        r#"
        SELECT DISTINCT b.candidate_product_ref
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        WHERE s.source_revision_ref=$1
          AND b.candidate_product_ref IS NOT NULL
        ORDER BY b.candidate_product_ref
        "#,
        &[&source_revision_ref],
    )?;

    for row in products {
        let product_ref: String = row.get::<_, Option<String>>(0)
            .ok_or(CorpusReconciliationError::ExistingRowConflict)?;
        let existing = client.query_opt(
            r#"
            SELECT s.factor_count, s.entity_factor_count, s.exact_reopen_validated,
                   s.candidate_only, s.creates_entity_identity,
                   s.creates_proposition_identity, s.creates_event_identity,
                   s.creates_semantic_authority, s.applicability_promoted,
                   s.claim_truth_promoted,
                   p.factor_count, p.exact_reopen_validated, p.candidate_only,
                   p.creates_semantic_authority, p.applicability_promoted,
                   p.claim_truth_promoted
            FROM semantic.l2_candidate_product_summary s
            JOIN pnf.candidate_semantic_product p
              ON p.product_ref=s.product_ref
            WHERE s.product_ref=$1 AND s.detector_ref=$2
            "#,
            &[&product_ref, &DETECTOR_REF],
        )?;
        if let Some(existing) = existing {
            if !existing.get::<_, bool>(2)
                || !existing.get::<_, bool>(3)
                || existing.get::<_, bool>(4)
                || existing.get::<_, bool>(5)
                || existing.get::<_, bool>(6)
                || existing.get::<_, bool>(7)
                || existing.get::<_, bool>(8)
                || existing.get::<_, bool>(9)
                || !existing.get::<_, bool>(11)
                || !existing.get::<_, bool>(12)
                || existing.get::<_, bool>(13)
                || existing.get::<_, bool>(14)
                || existing.get::<_, bool>(15)
            {
                return Err(CorpusReconciliationError::PromotionBoundary);
            }
            if existing.get::<_, i64>(0) != existing.get::<_, i64>(10) {
                return Err(CorpusReconciliationError::ExistingRowConflict);
            }
            let entity_factor_count = existing.get::<_, i64>(1);
            if entity_factor_count < 0 {
                return Err(CorpusReconciliationError::ExistingRowConflict);
            }
            let cached_entities: i64 = client
                .query_one(
                    "SELECT COUNT(*)::BIGINT
                     FROM semantic.l2_candidate_product_entity_factor
                     WHERE product_ref=$1 AND detector_ref=$2",
                    &[&product_ref, &DETECTOR_REF],
                )?
                .get(0);
            if cached_entities != entity_factor_count {
                return Err(CorpusReconciliationError::ExistingRowConflict);
            }
            work.product_summary_reuse_hits += 1;
            continue;
        }

        let product = client.query_one(
            r#"
            SELECT factor_count, exact_reopen_validated, candidate_only,
                   creates_semantic_authority, applicability_promoted,
                   claim_truth_promoted
            FROM pnf.candidate_semantic_product
            WHERE product_ref=$1
            "#,
            &[&product_ref],
        )?;
        if !product.get::<_, bool>(1)
            || !product.get::<_, bool>(2)
            || product.get::<_, bool>(3)
            || product.get::<_, bool>(4)
            || product.get::<_, bool>(5)
        {
            return Err(CorpusReconciliationError::PromotionBoundary);
        }

        let factor_rows = client.query(
            r#"
            SELECT factor_ordinal, token_ordinal, start_offset, end_offset,
                   role_ref, surface, lemma, dependency_ref, candidate_only
            FROM pnf.candidate_semantic_product_factor
            WHERE product_ref=$1
            ORDER BY factor_ordinal
            "#,
            &[&product_ref],
        )?;
        let mut factors = Vec::with_capacity(factor_rows.len());
        for row in factor_rows {
            if !row.get::<_, bool>(8) {
                return Err(CorpusReconciliationError::PromotionBoundary);
            }
            let factor = ProductFactorRow {
                factor_ordinal: row.get(0),
                token_ordinal: row.get(1),
                start_offset: row.get(2),
                end_offset: row.get(3),
                role_ref: row.get(4),
                surface: row.get(5),
                lemma: row.get(6),
                dependency_ref: row.get(7),
            };
            if factor.factor_ordinal < 0
                || factor.token_ordinal < 0
                || factor.start_offset < 0
                || factor.end_offset <= factor.start_offset
            {
                return Err(CorpusReconciliationError::ExistingRowConflict);
            }
            factors.push(factor);
        }
        if product.get::<_, i64>(0) < 0
            || product.get::<_, i64>(0) as usize != factors.len()
        {
            return Err(CorpusReconciliationError::ExistingRowConflict);
        }
        work.factor_rows_scanned += factors.len();

        let mut actors = Vec::new();
        let mut predicates = Vec::new();
        let mut patients = Vec::new();
        let mut entity_factors = Vec::new();
        let mut is_negative = false;
        for factor in &factors {
            let normalized = normalize(&factor.lemma);
            match factor.role_ref.as_str() {
                "actor" => {
                    if !normalized.is_empty() {
                        actors.push(normalized.clone());
                        entity_factors.push(factor.clone());
                    }
                }
                "predicate" => {
                    if !normalized.is_empty() {
                        predicates.push(normalized.clone());
                    }
                }
                "patient" => {
                    if !normalized.is_empty() {
                        patients.push(normalized.clone());
                        entity_factors.push(factor.clone());
                    }
                }
                _ => {}
            }
            is_negative |= factor.dependency_ref.eq_ignore_ascii_case("neg")
                || matches!(normalized.as_str(), "not" | "never" | "n't");
        }
        for values in [&mut actors, &mut predicates, &mut patients] {
            values.sort_unstable();
            values.dedup();
        }
        let base_signature = (!predicates.is_empty()).then(|| {
            format!(
                "a=[{}]|p=[{}]|o=[{}]",
                actors.join(","),
                predicates.join(","),
                patients.join(",")
            )
        });
        let polarity = base_signature
            .as_ref()
            .map(|_| if is_negative { "negative" } else { "positive" }.to_owned());
        let creates_event_candidate =
            base_signature.is_some() && (!actors.is_empty() || !patients.is_empty());

        let entity_factor_count = entity_factors.len();
        let mut tx = client.transaction()?;
        tx.execute(
            r#"
            INSERT INTO semantic.l2_candidate_product_summary
              (product_ref, detector_ref, factor_count, entity_factor_count,
               exact_reopen_validated, base_signature_ref,
               polarity_ref, creates_event_candidate, candidate_only,
               creates_entity_identity, creates_proposition_identity,
               creates_event_identity, creates_semantic_authority,
               applicability_promoted, claim_truth_promoted)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,TRUE,FALSE,FALSE,FALSE,FALSE,FALSE,FALSE)
            ON CONFLICT (product_ref, detector_ref) DO NOTHING
            "#,
            &[
                &product_ref,
                &DETECTOR_REF,
                &(factors.len() as i64),
                &(entity_factor_count as i64),
                &true,
                &base_signature,
                &polarity,
                &creates_event_candidate,
            ],
        )?;

        for factor in &entity_factors {
            let normalized = normalize(&factor.lemma);
            let entity_fingerprint_ref =
                format!("entity-fingerprint:{}", digest_ref("entity:v1", &[&normalized]));
            tx.execute(
                r#"
                INSERT INTO semantic.l2_candidate_product_entity_factor
                  (product_ref, detector_ref, factor_ordinal, token_ordinal,
                   start_offset, end_offset, entity_fingerprint_ref, role_ref,
                   surface, lemma, candidate_only, creates_entity_identity,
                   creates_semantic_authority, claim_truth_promoted)
                VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE,FALSE,FALSE,FALSE)
                ON CONFLICT (product_ref, detector_ref, factor_ordinal) DO NOTHING
                "#,
                &[
                    &product_ref,
                    &DETECTOR_REF,
                    &factor.factor_ordinal,
                    &factor.token_ordinal,
                    &factor.start_offset,
                    &factor.end_offset,
                    &entity_fingerprint_ref,
                    &factor.role_ref,
                    &factor.surface,
                    &factor.lemma,
                ],
            )?;
        }
        tx.commit()?;
        let cached_entities: i64 = client
            .query_one(
                "SELECT COUNT(*)::BIGINT
                 FROM semantic.l2_candidate_product_entity_factor
                 WHERE product_ref=$1 AND detector_ref=$2",
                &[&product_ref, &DETECTOR_REF],
            )?
            .get(0);
        if cached_entities != entity_factor_count as i64 {
            return Err(CorpusReconciliationError::ExistingRowConflict);
        }
        work.product_summaries_created += 1;
    }
    Ok(())
}

fn load_statement_batches(
    client: &mut Client,
    source_revision_ref: &str,
) -> Result<Vec<StatementBatch>, CorpusReconciliationError> {
    let rows = client.query(
        r#"
        SELECT b.batch_ref, b.statement_ref, s.source_revision_ref,
               s.exact_span_ref, s.literal_text, b.parser_receipt_ref,
               b.candidate_product_ref, b.candidate_only,
               b.semantic_admission_paid, b.proposition_support_paid,
               b.applicability_paid, b.claim_truth_paid
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        WHERE s.source_revision_ref=$1
          AND b.candidate_product_ref IS NULL
        ORDER BY s.exact_span_ref, b.batch_ref
        "#,
        &[&source_revision_ref],
    )?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if !row.get::<_, bool>(7)
            || row.get::<_, bool>(8)
            || row.get::<_, bool>(9)
            || row.get::<_, bool>(10)
            || row.get::<_, bool>(11)
        {
            return Err(CorpusReconciliationError::PromotionBoundary);
        }
        let batch_ref: String = row.get(0);
        let exact_span_ref: String = row.get(3);
        let parser_receipt_ref: String = row.get(5);
        let candidate_product_ref: Option<String> = row.get(6);

        let factors = if let Some(product_ref) = candidate_product_ref {
            let parser_job = client.query_opt(
                r#"
                SELECT parser_run_ref, region_start_char,
                       candidate_only, creates_semantic_authority,
                       applicability_promoted, claim_truth_promoted
                FROM ingest.parser_job
                WHERE source_revision_ref=$1
                  AND region_ref=$2
                  AND ('db-parser:' || parser_run_ref || ':' || region_ref)=$3
                  AND status='succeeded'
                "#,
                &[&source_revision_ref, &exact_span_ref, &parser_receipt_ref],
            )?;
            let Some(parser_job) = parser_job else {
                return Err(CorpusReconciliationError::ExistingRowConflict);
            };
            if !parser_job.get::<_, bool>(2)
                || parser_job.get::<_, bool>(3)
                || parser_job.get::<_, bool>(4)
                || parser_job.get::<_, bool>(5)
            {
                return Err(CorpusReconciliationError::PromotionBoundary);
            }
            let parser_run_ref: String = parser_job.get(0);
            let region_start: i64 = parser_job.get(1);
            let factor_rows = client.query(
                r#"
                SELECT token_ordinal, start_offset, end_offset,
                       role_ref, surface, lemma, dependency_ref, candidate_only
                FROM pnf.candidate_semantic_product_factor
                WHERE product_ref=$1
                ORDER BY factor_ordinal
                "#,
                &[&product_ref],
            )?;
            factor_rows
                .into_iter()
                .map(|factor| {
                    let token_ordinal: i32 = factor.get(0);
                    let start = region_start + factor.get::<_, i64>(1);
                    let end = region_start + factor.get::<_, i64>(2);
                    FactorRow {
                        candidate_ref: format!(
                            "candidate-pnf-db:{}:{}:{}:{}:{}",
                            parser_run_ref, exact_span_ref, token_ordinal, start, end
                        ),
                        role_ref: factor.get(3),
                        start_char: start,
                        end_char: end,
                        surface: factor.get(4),
                        lemma: factor.get(5),
                        dependency_ref: factor.get(6),
                        candidate_only: factor.get(7),
                    }
                })
                .collect::<Vec<_>>()
        } else {
            let factor_rows = client.query(
                r#"
                SELECT candidate_ref, role_ref, source_start_char, source_end_char,
                       surface, lemma, dependency_ref, candidate_only
                FROM pnf.statement_candidate_factor
                WHERE batch_ref=$1
                ORDER BY source_start_char, source_end_char, candidate_ref
                "#,
                &[&batch_ref],
            )?;
            factor_rows
                .into_iter()
                .map(|factor| FactorRow {
                    candidate_ref: factor.get(0),
                    role_ref: factor.get(1),
                    start_char: factor.get(2),
                    end_char: factor.get(3),
                    surface: factor.get(4),
                    lemma: factor.get(5),
                    dependency_ref: factor.get(6),
                    candidate_only: factor.get(7),
                })
                .collect::<Vec<_>>()
        };

        if factors.iter().any(|factor| {
            !factor.candidate_only || factor.start_char < 0 || factor.end_char <= factor.start_char
        }) {
            return Err(CorpusReconciliationError::PromotionBoundary);
        }
        out.push(StatementBatch {
            batch_ref,
            statement_ref: row.get(1),
            source_revision_ref: row.get(2),
            exact_span_ref,
            literal_text: row.get(4),
            factors,
        });
    }
    Ok(out)
}

#[derive(Debug, Default)]
struct ProductBindingResult {
    statement_count: usize,
    entity_mention_count: usize,
    proposition_occurrence_count: usize,
    event_occurrence_count: usize,
    entity_fingerprints: BTreeSet<String>,
    proposition_fingerprints: BTreeSet<String>,
    event_fingerprints: BTreeSet<String>,
    touched_base_signatures: BTreeSet<String>,
}

fn bind_l2_product_occurrences(
    tx: &mut postgres::Transaction<'_>,
    source_revision_ref: &str,
    work: &mut CorpusReconciliationWork,
) -> Result<ProductBindingResult, CorpusReconciliationError> {
    let mut out = ProductBindingResult::default();

    let entity_rows = tx.query(
        r#"
        SELECT b.batch_ref, b.statement_ref, s.exact_span_ref,
               j.parser_run_ref, j.region_start_char,
               f.token_ordinal, f.start_offset, f.end_offset,
               f.entity_fingerprint_ref, f.role_ref, f.surface, f.lemma
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        JOIN ingest.parser_job j
          ON j.source_revision_ref=s.source_revision_ref
         AND j.region_ref=s.exact_span_ref
         AND ('db-parser:' || j.parser_run_ref || ':' || j.region_ref)=b.parser_receipt_ref
         AND j.status='succeeded'
        JOIN semantic.l2_candidate_product_entity_factor f
          ON f.product_ref=b.candidate_product_ref
         AND f.detector_ref=$2
        WHERE s.source_revision_ref=$1
          AND b.candidate_product_ref IS NOT NULL
        ORDER BY b.batch_ref, f.factor_ordinal
        "#,
        &[&source_revision_ref, &DETECTOR_REF],
    )?;
    for row in entity_rows {
        let batch_ref: String = row.get(0);
        let statement_ref: String = row.get(1);
        let exact_span_ref: String = row.get(2);
        let parser_run_ref: String = row.get(3);
        let region_start: i64 = row.get(4);
        let token_ordinal: i32 = row.get(5);
        let start = region_start + row.get::<_, i64>(6);
        let end = region_start + row.get::<_, i64>(7);
        if region_start < 0 || token_ordinal < 0 || start < 0 || end <= start {
            return Err(CorpusReconciliationError::ExistingRowConflict);
        }
        let entity_fingerprint_ref: String = row.get(8);
        let candidate_ref = format!(
            "candidate-pnf-db:{}:{}:{}:{}:{}",
            parser_run_ref, exact_span_ref, token_ordinal, start, end
        );
        let mention_ref = format!(
            "entity-mention:{}",
            digest_ref(
                "entity-mention:v1",
                &[&statement_ref, &batch_ref, &candidate_ref],
            )
        );
        work.entity_mention_rows_inserted += tx.execute(
            r#"
            INSERT INTO semantic.entity_mention_candidate
              (mention_ref, entity_fingerprint_ref, statement_ref, candidate_ref,
               role_ref, surface, lemma, source_start_char, source_end_char,
               detector_ref, candidate_only, creates_entity_identity,
               creates_semantic_authority, claim_truth_promoted)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE,FALSE,FALSE,FALSE)
            ON CONFLICT (mention_ref) DO NOTHING
            "#,
            &[
                &mention_ref,
                &entity_fingerprint_ref,
                &statement_ref,
                &candidate_ref,
                &row.get::<_, String>(9),
                &row.get::<_, String>(10),
                &row.get::<_, String>(11),
                &start,
                &end,
                &DETECTOR_REF,
            ],
        )? as usize;
        out.entity_mention_count += 1;
        out.entity_fingerprints.insert(entity_fingerprint_ref);
    }

    let batch_rows = tx.query(
        r#"
        SELECT b.batch_ref, b.statement_ref, s.exact_span_ref, s.literal_text,
               p.base_signature_ref, p.polarity_ref, p.creates_event_candidate
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        JOIN semantic.l2_candidate_product_summary p
          ON p.product_ref=b.candidate_product_ref
         AND p.detector_ref=$2
        WHERE s.source_revision_ref=$1
          AND b.candidate_product_ref IS NOT NULL
        ORDER BY s.exact_span_ref, b.batch_ref
        "#,
        &[&source_revision_ref, &DETECTOR_REF],
    )?;
    out.statement_count = batch_rows.len();
    for row in batch_rows {
        let batch_ref: String = row.get(0);
        let statement_ref: String = row.get(1);
        let exact_span_ref: String = row.get(2);
        let literal_text: String = row.get(3);
        let base: Option<String> = row.get(4);
        let polarity: Option<String> = row.get(5);
        let creates_event_candidate: bool = row.get(6);
        let (Some(base), Some(polarity)) = (base, polarity) else {
            continue;
        };
        let proposition_fingerprint_ref = format!(
            "proposition-fingerprint:{}",
            digest_ref("proposition-fingerprint:v1", &[&base, &polarity])
        );
        work.proposition_fingerprint_rows_inserted += tx.execute(
            r#"
            INSERT INTO semantic.proposition_fingerprint_candidate
              (proposition_fingerprint_ref, base_signature_ref, polarity_ref,
               detector_ref, candidate_only, creates_proposition_identity,
               creates_semantic_authority, applicability_promoted, claim_truth_promoted)
            VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE,FALSE)
            ON CONFLICT (proposition_fingerprint_ref) DO NOTHING
            "#,
            &[&proposition_fingerprint_ref, &base, &polarity, &DETECTOR_REF],
        )? as usize;
        work.proposition_occurrence_rows_inserted += tx.execute(
            r#"
            INSERT INTO semantic.proposition_candidate_occurrence
              (proposition_fingerprint_ref, statement_ref, batch_ref,
               source_revision_ref, exact_span_ref, literal_text,
               candidate_only, creates_semantic_authority, claim_truth_promoted)
            VALUES ($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE)
            ON CONFLICT DO NOTHING
            "#,
            &[
                &proposition_fingerprint_ref,
                &statement_ref,
                &batch_ref,
                &source_revision_ref,
                &exact_span_ref,
                &literal_text,
            ],
        )? as usize;
        out.proposition_occurrence_count += 1;
        out.proposition_fingerprints
            .insert(proposition_fingerprint_ref.clone());
        out.touched_base_signatures.insert(base.clone());

        if creates_event_candidate {
            let event_fingerprint_ref = format!(
                "event-fingerprint:{}",
                digest_ref("event-fingerprint:v1", &[&base, &polarity])
            );
            work.event_fingerprint_rows_inserted += tx.execute(
                r#"
                INSERT INTO semantic.event_fingerprint_candidate
                  (event_fingerprint_ref, base_signature_ref, polarity_ref,
                   detector_ref, candidate_only, creates_event_identity,
                   creates_semantic_authority, claim_truth_promoted)
                VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE)
                ON CONFLICT (event_fingerprint_ref) DO NOTHING
                "#,
                &[&event_fingerprint_ref, &base, &polarity, &DETECTOR_REF],
            )? as usize;
            work.event_occurrence_rows_inserted += tx.execute(
                r#"
                INSERT INTO semantic.event_candidate_occurrence
                  (event_fingerprint_ref, statement_ref, batch_ref,
                   source_revision_ref, exact_span_ref, candidate_only,
                   creates_event_identity, creates_semantic_authority,
                   claim_truth_promoted)
                VALUES ($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE)
                ON CONFLICT DO NOTHING
                "#,
                &[
                    &event_fingerprint_ref,
                    &statement_ref,
                    &batch_ref,
                    &source_revision_ref,
                    &exact_span_ref,
                ],
            )? as usize;
            out.event_occurrence_count += 1;
            out.event_fingerprints.insert(event_fingerprint_ref);
        }
    }

    Ok(out)
}

pub fn install_corpus_reconciliation_schema(
    config: &DatabaseConfig,
) -> Result<(), CorpusReconciliationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CORPUS_RECONCILIATION_SCHEMA_SQL)?;
    Ok(())
}

fn load_reconciliation_stage_receipt(
    client: &mut Client,
    source_revision_ref: &str,
    parser_run_ref: &str,
) -> Result<Option<CorpusReconciliationReceipt>, CorpusReconciliationError> {
    let Some(row) = client.query_opt(
        r#"
        SELECT statement_count, entity_mention_count, entity_fingerprint_count,
               named_entity_mention_count, named_entity_fingerprint_count,
               temporal_mention_count, proposition_occurrence_count,
               proposition_fingerprint_count, event_occurrence_count,
               event_fingerprint_count, polarity_conflict_candidate_count,
               review_pressure_candidate_count, candidate_only,
               creates_semantic_authority, creates_entity_identity,
               creates_proposition_identity, creates_event_identity,
               claim_truth_promoted
        FROM semantic.corpus_reconciliation_stage_receipt
        WHERE source_revision_ref=$1 AND parser_run_ref=$2 AND detector_ref=$3
        "#,
        &[&source_revision_ref, &parser_run_ref, &DETECTOR_REF],
    )? else {
        return Ok(None);
    };
    let count = |idx: usize| row.get::<_, i64>(idx).max(0) as usize;
    let receipt = CorpusReconciliationReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: true,
        work: CorpusReconciliationWork::default(),
        statement_count: count(0),
        entity_mention_count: count(1),
        entity_fingerprint_count: count(2),
        named_entity_mention_count: count(3),
        named_entity_fingerprint_count: count(4),
        temporal_mention_count: count(5),
        proposition_occurrence_count: count(6),
        proposition_fingerprint_count: count(7),
        event_occurrence_count: count(8),
        event_fingerprint_count: count(9),
        polarity_conflict_candidate_count: count(10),
        review_pressure_candidate_count: count(11),
        candidate_only: row.get(12),
        creates_semantic_authority: row.get(13),
        creates_entity_identity: row.get(14),
        creates_proposition_identity: row.get(15),
        creates_event_identity: row.get(16),
        claim_truth_promoted: row.get(17),
    };
    if !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.creates_entity_identity
        || receipt.creates_proposition_identity
        || receipt.creates_event_identity
        || receipt.claim_truth_promoted
    {
        return Err(CorpusReconciliationError::PromotionBoundary);
    }
    Ok(Some(receipt))
}

pub fn reconcile_source_candidate_semantics(
    config: &DatabaseConfig,
    source_revision_ref: &str,
    parser_run_ref: &str,
) -> Result<CorpusReconciliationReceipt, CorpusReconciliationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CORPUS_RECONCILIATION_SCHEMA_SQL)?;
    if let Some(receipt) =
        load_reconciliation_stage_receipt(&mut client, source_revision_ref, parser_run_ref)?
    {
        return Ok(receipt);
    }
    let mut work = CorpusReconciliationWork::default();
    ensure_l2_product_summaries(&mut client, source_revision_ref, &mut work)?;
    let batches = load_statement_batches(&mut client, source_revision_ref)?;
    work.factor_rows_scanned += batches.iter().map(|batch| batch.factors.len()).sum::<usize>();

    let mut tx = client.transaction()?;
    let product_binding =
        bind_l2_product_occurrences(&mut tx, source_revision_ref, &mut work)?;
    let product_statement_count = product_binding.statement_count;
    let product_entity_mention_count = product_binding.entity_mention_count;
    let product_proposition_occurrence_count = product_binding.proposition_occurrence_count;
    let product_event_occurrence_count = product_binding.event_occurrence_count;
    let mut entity_fingerprints = product_binding.entity_fingerprints;
    let mut proposition_fingerprints = product_binding.proposition_fingerprints;
    let mut event_fingerprints = product_binding.event_fingerprints;
    let mut touched_base_signatures = product_binding.touched_base_signatures;

    let mut entity_mention_count = product_entity_mention_count;
    let mut named_entity_mention_count = 0usize;
    let mut temporal_mention_count = 0usize;
    let mut named_entity_fingerprints = BTreeSet::new();
    let mut proposition_occurrence_count = product_proposition_occurrence_count;
    let mut event_occurrence_count = product_event_occurrence_count;

    for batch in &batches {
        for factor in batch
            .factors
            .iter()
            .filter(|factor| matches!(factor.role_ref.as_str(), "actor" | "patient"))
        {
            let normalized = normalize(&factor.lemma);
            if normalized.is_empty() {
                continue;
            }
            let entity_fingerprint_ref =
                format!("entity-fingerprint:{}", digest_ref("entity:v1", &[&normalized]));
            let mention_ref = format!(
                "entity-mention:{}",
                digest_ref(
                    "entity-mention:v1",
                    &[&batch.statement_ref, &batch.batch_ref, &factor.candidate_ref],
                )
            );
            work.entity_mention_rows_inserted += tx.execute(
                r#"INSERT INTO semantic.entity_mention_candidate
                   (mention_ref, entity_fingerprint_ref, statement_ref, candidate_ref,
                    role_ref, surface, lemma, source_start_char, source_end_char,
                    detector_ref, candidate_only, creates_entity_identity,
                    creates_semantic_authority, claim_truth_promoted)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,TRUE,FALSE,FALSE,FALSE)
                   ON CONFLICT (mention_ref) DO NOTHING"#,
                &[
                    &mention_ref,
                    &entity_fingerprint_ref,
                    &batch.statement_ref,
                    &factor.candidate_ref,
                    &factor.role_ref,
                    &factor.surface,
                    &factor.lemma,
                    &factor.start_char,
                    &factor.end_char,
                    &DETECTOR_REF,
                ],
            )? as usize;
            entity_mention_count += 1;
            entity_fingerprints.insert(entity_fingerprint_ref);
        }

        let Some(base) = base_signature(batch) else {
            continue;
        };
        let polarity = if negative(batch) { "negative" } else { "positive" };
        let proposition_fingerprint_ref = format!(
            "proposition-fingerprint:{}",
            digest_ref("proposition-fingerprint:v1", &[&base, polarity])
        );
        work.proposition_fingerprint_rows_inserted += tx.execute(
            r#"INSERT INTO semantic.proposition_fingerprint_candidate
               (proposition_fingerprint_ref, base_signature_ref, polarity_ref,
                detector_ref, candidate_only, creates_proposition_identity,
                creates_semantic_authority, applicability_promoted, claim_truth_promoted)
               VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE,FALSE)
               ON CONFLICT (proposition_fingerprint_ref) DO NOTHING"#,
            &[&proposition_fingerprint_ref, &base, &polarity, &DETECTOR_REF],
        )? as usize;
        work.proposition_occurrence_rows_inserted += tx.execute(
            r#"INSERT INTO semantic.proposition_candidate_occurrence
               (proposition_fingerprint_ref, statement_ref, batch_ref,
                source_revision_ref, exact_span_ref, literal_text,
                candidate_only, creates_semantic_authority, claim_truth_promoted)
               VALUES ($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE)
               ON CONFLICT DO NOTHING"#,
            &[
                &proposition_fingerprint_ref,
                &batch.statement_ref,
                &batch.batch_ref,
                &batch.source_revision_ref,
                &batch.exact_span_ref,
                &batch.literal_text,
            ],
        )? as usize;
        proposition_occurrence_count += 1;
        proposition_fingerprints.insert(proposition_fingerprint_ref.clone());
        touched_base_signatures.insert(base.clone());

        let actors = role_lemmas(batch, "actor");
        let patients = role_lemmas(batch, "patient");
        if !actors.is_empty() || !patients.is_empty() {
            let event_fingerprint_ref = format!(
                "event-fingerprint:{}",
                digest_ref("event-fingerprint:v1", &[&base, polarity])
            );
            work.event_fingerprint_rows_inserted += tx.execute(
                r#"INSERT INTO semantic.event_fingerprint_candidate
                   (event_fingerprint_ref, base_signature_ref, polarity_ref,
                    detector_ref, candidate_only, creates_event_identity,
                    creates_semantic_authority, claim_truth_promoted)
                   VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE)
                   ON CONFLICT (event_fingerprint_ref) DO NOTHING"#,
                &[&event_fingerprint_ref, &base, &polarity, &DETECTOR_REF],
            )? as usize;
            work.event_occurrence_rows_inserted += tx.execute(
                r#"INSERT INTO semantic.event_candidate_occurrence
                   (event_fingerprint_ref, statement_ref, batch_ref,
                    source_revision_ref, exact_span_ref, candidate_only,
                    creates_event_identity, creates_semantic_authority,
                    claim_truth_promoted)
                   VALUES ($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE)
                   ON CONFLICT DO NOTHING"#,
                &[
                    &event_fingerprint_ref,
                    &batch.statement_ref,
                    &batch.batch_ref,
                    &batch.source_revision_ref,
                    &batch.exact_span_ref,
                ],
            )? as usize;
            event_occurrence_count += 1;
            event_fingerprints.insert(event_fingerprint_ref);
        }
    }


    let entity_rows = tx.query(
        r#"
        SELECT e.entity_ordinal, e.start_char, e.end_char, e.surface, e.label_ref,
               j.region_ref, s.statement_ref
        FROM ingest.parser_entity e
        JOIN ingest.parser_job j ON j.compilation_key=e.compilation_key
        JOIN corpus.source_statement s
          ON s.source_revision_ref=j.source_revision_ref
         AND s.exact_span_ref=j.region_ref
        WHERE j.parser_run_ref=$1
          AND j.source_revision_ref=$2
          AND j.status='succeeded'
        ORDER BY j.region_ref, e.entity_ordinal
        "#,
        &[&parser_run_ref, &source_revision_ref],
    )?;
    for row in entity_rows {
        let start_char = row.get::<_, i64>(1);
        let end_char = row.get::<_, i64>(2);
        if start_char < 0 || end_char <= start_char {
            return Err(CorpusReconciliationError::PromotionBoundary);
        }
        let surface: String = row.get(3);
        let label_ref: String = row.get(4);
        let exact_span_ref: String = row.get(5);
        let statement_ref: String = row.get(6);
        let normalized = normalize(&surface);
        if normalized.is_empty() || label_ref.trim().is_empty() {
            continue;
        }
        let entity_fingerprint_ref = format!(
            "named-entity-fingerprint:{}",
            digest_ref("named-entity:v1", &[&label_ref, &normalized])
        );
        let mention_ref = format!(
            "named-entity-mention:{}",
            digest_ref(
                "named-entity-mention:v1",
                &[
                    parser_run_ref,
                    &statement_ref,
                    &start_char.to_string(),
                    &end_char.to_string(),
                    &label_ref,
                ],
            )
        );
        work.named_entity_rows_inserted += tx.execute(
            r#"INSERT INTO semantic.named_entity_candidate
               (mention_ref, entity_fingerprint_ref, parser_run_ref,
                statement_ref, exact_span_ref, start_char, end_char,
                surface, label_ref, detector_ref, candidate_only,
                creates_entity_identity, creates_semantic_authority,
                claim_truth_promoted)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,
                       'scale1:parser-ner:v1',TRUE,FALSE,FALSE,FALSE)
               ON CONFLICT (mention_ref) DO NOTHING"#,
            &[
                &mention_ref,
                &entity_fingerprint_ref,
                &parser_run_ref,
                &statement_ref,
                &exact_span_ref,
                &start_char,
                &end_char,
                &surface,
                &label_ref,
            ],
        )? as usize;
        named_entity_mention_count += 1;
        named_entity_fingerprints.insert(entity_fingerprint_ref);

        if matches!(label_ref.as_str(), "DATE" | "TIME") {
            let temporal_mention_ref = format!(
                "temporal-mention:{}",
                digest_ref(
                    "temporal-mention:v1",
                    &[
                        parser_run_ref,
                        &statement_ref,
                        &start_char.to_string(),
                        &end_char.to_string(),
                        &label_ref,
                    ],
                )
            );
            work.temporal_rows_inserted += tx.execute(
                r#"INSERT INTO semantic.temporal_mention_candidate
                   (temporal_mention_ref, parser_run_ref, statement_ref,
                    exact_span_ref, start_char, end_char, surface, label_ref,
                    detector_ref, candidate_only, creates_temporal_assertion,
                    creates_semantic_authority, claim_truth_promoted)
                   VALUES ($1,$2,$3,$4,$5,$6,$7,$8,
                           'scale1:parser-ner:v1',TRUE,FALSE,FALSE,FALSE)
                   ON CONFLICT (temporal_mention_ref) DO NOTHING"#,
                &[
                    &temporal_mention_ref,
                    &parser_run_ref,
                    &statement_ref,
                    &exact_span_ref,
                    &start_char,
                    &end_char,
                    &surface,
                    &label_ref,
                ],
            )? as usize;
            temporal_mention_count += 1;
        }
    }

    let mut polarity_conflict_candidate_count = 0usize;
    for base in &touched_base_signatures {
        let rows = tx.query(
            "SELECT polarity_ref, proposition_fingerprint_ref
             FROM semantic.proposition_fingerprint_candidate
             WHERE base_signature_ref=$1
             ORDER BY polarity_ref, proposition_fingerprint_ref",
            &[base],
        )?;
        let mut by_polarity = BTreeMap::new();
        for row in rows {
            by_polarity.insert(row.get::<_, String>(0), row.get::<_, String>(1));
        }
        if let (Some(positive), Some(negative)) =
            (by_polarity.get("positive"), by_polarity.get("negative"))
        {
            let relation_ref = format!(
                "contestation-candidate:{}",
                digest_ref("polarity-conflict:v1", &[base, positive, negative])
            );
            work.contestation_rows_inserted += tx.execute(
                r#"INSERT INTO semantic.contestation_candidate
                   (relation_ref, base_signature_ref,
                    positive_proposition_fingerprint_ref,
                    negative_proposition_fingerprint_ref, kind_ref, detector_ref,
                    candidate_only, requires_review, creates_contestation_identity,
                    creates_semantic_authority, claim_truth_promoted)
                   VALUES ($1,$2,$3,$4,'potential_polarity_conflict',$5,
                           TRUE,TRUE,FALSE,FALSE,FALSE)
                   ON CONFLICT (relation_ref) DO NOTHING"#,
                &[&relation_ref, base, positive, negative, &DETECTOR_REF],
            )? as usize;
            polarity_conflict_candidate_count += 1;
        }
    }

    // Review pressure is scheduling metadata. Multi-occurrence clusters are
    // surfaced because reconciliation can amortize review across repeated
    // candidate semantics; this is not a truth/importance score.
    let pressure_rows = tx.query(
        r#"
        WITH touched_proposition AS (
          SELECT DISTINCT proposition_fingerprint_ref
          FROM semantic.proposition_candidate_occurrence
          WHERE source_revision_ref=$1
        ),
        touched_event AS (
          SELECT DISTINCT event_fingerprint_ref
          FROM semantic.event_candidate_occurrence
          WHERE source_revision_ref=$1
        )
        SELECT 'proposition'::TEXT, o.proposition_fingerprint_ref,
               COUNT(*)::BIGINT, COUNT(DISTINCT o.source_revision_ref)::BIGINT
        FROM semantic.proposition_candidate_occurrence o
        JOIN touched_proposition t
          ON t.proposition_fingerprint_ref=o.proposition_fingerprint_ref
        GROUP BY o.proposition_fingerprint_ref
        HAVING COUNT(*) > 1
        UNION ALL
        SELECT 'event'::TEXT, o.event_fingerprint_ref,
               COUNT(*)::BIGINT, COUNT(DISTINCT o.source_revision_ref)::BIGINT
        FROM semantic.event_candidate_occurrence o
        JOIN touched_event t ON t.event_fingerprint_ref=o.event_fingerprint_ref
        GROUP BY o.event_fingerprint_ref
        HAVING COUNT(*) > 1
        "#,
        &[&source_revision_ref],
    )?;
    let mut review_pressure_candidate_count = 0usize;
    for row in pressure_rows {
        let kind: String = row.get(0);
        let fingerprint: String = row.get(1);
        let occurrence_count: i64 = row.get(2);
        let source_revision_count: i64 = row.get(3);
        let pressure_ref = format!(
            "reconciliation-pressure:{}",
            digest_ref("reconciliation-pressure:v1", &[&kind, &fingerprint])
        );
        work.pressure_rows_upserted += tx.execute(
            r#"INSERT INTO semantic.reconciliation_pressure_candidate
               (pressure_ref, semantic_kind_ref, semantic_fingerprint_ref,
                occurrence_count, source_revision_count, reason_ref,
                candidate_only, requires_review, creates_semantic_authority,
                claim_truth_promoted)
               VALUES ($1,$2,$3,$4,$5,'multi_occurrence_cluster',
                       TRUE,TRUE,FALSE,FALSE)
               ON CONFLICT (pressure_ref) DO UPDATE SET
                 occurrence_count=EXCLUDED.occurrence_count,
                 source_revision_count=EXCLUDED.source_revision_count"#,
            &[
                &pressure_ref,
                &kind,
                &fingerprint,
                &occurrence_count,
                &source_revision_count,
            ],
        )? as usize;
        review_pressure_candidate_count += 1;
    }

    let receipt = CorpusReconciliationReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        stage_reused: false,
        work,
        statement_count: product_statement_count + batches.len(),
        entity_mention_count,
        entity_fingerprint_count: entity_fingerprints.len(),
        named_entity_mention_count,
        named_entity_fingerprint_count: named_entity_fingerprints.len(),
        temporal_mention_count,
        proposition_occurrence_count,
        proposition_fingerprint_count: proposition_fingerprints.len(),
        event_occurrence_count,
        event_fingerprint_count: event_fingerprints.len(),
        polarity_conflict_candidate_count,
        review_pressure_candidate_count,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_entity_identity: false,
        creates_proposition_identity: false,
        creates_event_identity: false,
        claim_truth_promoted: false,
    };

    tx.execute(
        r#"
        INSERT INTO semantic.corpus_reconciliation_stage_receipt
        (source_revision_ref, parser_run_ref, detector_ref, statement_count,
         entity_mention_count, entity_fingerprint_count,
         named_entity_mention_count, named_entity_fingerprint_count,
         temporal_mention_count, proposition_occurrence_count,
         proposition_fingerprint_count, event_occurrence_count,
         event_fingerprint_count, polarity_conflict_candidate_count,
         review_pressure_candidate_count, candidate_only,
         creates_semantic_authority, creates_entity_identity,
         creates_proposition_identity, creates_event_identity,
         claim_truth_promoted)
        VALUES
        ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,
         TRUE,FALSE,FALSE,FALSE,FALSE,FALSE)
        ON CONFLICT (source_revision_ref, parser_run_ref, detector_ref) DO NOTHING
        "#,
        &[
            &source_revision_ref,
            &parser_run_ref,
            &DETECTOR_REF,
            &(receipt.statement_count as i64),
            &(receipt.entity_mention_count as i64),
            &(receipt.entity_fingerprint_count as i64),
            &(receipt.named_entity_mention_count as i64),
            &(receipt.named_entity_fingerprint_count as i64),
            &(receipt.temporal_mention_count as i64),
            &(receipt.proposition_occurrence_count as i64),
            &(receipt.proposition_fingerprint_count as i64),
            &(receipt.event_occurrence_count as i64),
            &(receipt.event_fingerprint_count as i64),
            &(receipt.polarity_conflict_candidate_count as i64),
            &(receipt.review_pressure_candidate_count as i64),
        ],
    )?;
    tx.commit()?;

    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_deterministic_and_not_identity_payment() {
        assert_eq!(normalize("  Alice   SMITH "), "alice smith");
        assert_ne!(
            digest_ref("entity:v1", &["alice"]),
            digest_ref("proposition-fingerprint:v1", &["alice"])
        );
    }
}
