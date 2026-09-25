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
"#;

const DETECTOR_REF: &str = "scale1:persistent-pnf-fingerprint:v1";

#[derive(Debug, Error)]
pub enum CorpusReconciliationError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("candidate PNF source rows violated expected non-promotion boundary")]
    PromotionBoundary,
    #[error("persisted reconciliation row conflicted with deterministic identity")]
    ExistingRowConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusReconciliationReceipt {
    pub source_revision_ref: String,
    pub statement_count: usize,
    pub entity_mention_count: usize,
    pub entity_fingerprint_count: usize,
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

fn load_statement_batches(
    client: &mut Client,
    source_revision_ref: &str,
) -> Result<Vec<StatementBatch>, CorpusReconciliationError> {
    let rows = client.query(
        r#"
        SELECT b.batch_ref, b.statement_ref, s.source_revision_ref,
               s.exact_span_ref, s.literal_text,
               b.candidate_only, b.semantic_admission_paid,
               b.proposition_support_paid, b.applicability_paid, b.claim_truth_paid
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        WHERE s.source_revision_ref=$1
        ORDER BY s.exact_span_ref, b.batch_ref
        "#,
        &[&source_revision_ref],
    )?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        if !row.get::<_, bool>(5)
            || row.get::<_, bool>(6)
            || row.get::<_, bool>(7)
            || row.get::<_, bool>(8)
            || row.get::<_, bool>(9)
        {
            return Err(CorpusReconciliationError::PromotionBoundary);
        }
        let batch_ref: String = row.get(0);
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
        let factors = factor_rows
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
            .collect::<Vec<_>>();
        if factors.iter().any(|factor| {
            !factor.candidate_only || factor.start_char < 0 || factor.end_char <= factor.start_char
        }) {
            return Err(CorpusReconciliationError::PromotionBoundary);
        }
        out.push(StatementBatch {
            batch_ref,
            statement_ref: row.get(1),
            source_revision_ref: row.get(2),
            exact_span_ref: row.get(3),
            literal_text: row.get(4),
            factors,
        });
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

pub fn reconcile_source_candidate_semantics(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<CorpusReconciliationReceipt, CorpusReconciliationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CORPUS_RECONCILIATION_SCHEMA_SQL)?;
    let batches = load_statement_batches(&mut client, source_revision_ref)?;

    let mut tx = client.transaction()?;
    let mut entity_fingerprints = BTreeSet::new();
    let mut proposition_fingerprints = BTreeSet::new();
    let mut event_fingerprints = BTreeSet::new();
    let mut proposition_by_base: BTreeMap<String, BTreeMap<&'static str, String>> =
        BTreeMap::new();

    let mut entity_mention_count = 0usize;
    let mut proposition_occurrence_count = 0usize;
    let mut event_occurrence_count = 0usize;

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
            tx.execute(
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
            )?;
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
        tx.execute(
            r#"INSERT INTO semantic.proposition_fingerprint_candidate
               (proposition_fingerprint_ref, base_signature_ref, polarity_ref,
                detector_ref, candidate_only, creates_proposition_identity,
                creates_semantic_authority, applicability_promoted, claim_truth_promoted)
               VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE,FALSE)
               ON CONFLICT (proposition_fingerprint_ref) DO NOTHING"#,
            &[&proposition_fingerprint_ref, &base, &polarity, &DETECTOR_REF],
        )?;
        tx.execute(
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
        )?;
        proposition_occurrence_count += 1;
        proposition_fingerprints.insert(proposition_fingerprint_ref.clone());
        proposition_by_base
            .entry(base.clone())
            .or_default()
            .insert(polarity, proposition_fingerprint_ref);

        let actors = role_lemmas(batch, "actor");
        let patients = role_lemmas(batch, "patient");
        if !actors.is_empty() || !patients.is_empty() {
            let event_fingerprint_ref = format!(
                "event-fingerprint:{}",
                digest_ref("event-fingerprint:v1", &[&base, polarity])
            );
            tx.execute(
                r#"INSERT INTO semantic.event_fingerprint_candidate
                   (event_fingerprint_ref, base_signature_ref, polarity_ref,
                    detector_ref, candidate_only, creates_event_identity,
                    creates_semantic_authority, claim_truth_promoted)
                   VALUES ($1,$2,$3,$4,TRUE,FALSE,FALSE,FALSE)
                   ON CONFLICT (event_fingerprint_ref) DO NOTHING"#,
                &[&event_fingerprint_ref, &base, &polarity, &DETECTOR_REF],
            )?;
            tx.execute(
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
            )?;
            event_occurrence_count += 1;
            event_fingerprints.insert(event_fingerprint_ref);
        }
    }

    let mut polarity_conflict_candidate_count = 0usize;
    for (base, by_polarity) in &proposition_by_base {
        if let (Some(positive), Some(negative)) =
            (by_polarity.get("positive"), by_polarity.get("negative"))
        {
            let relation_ref = format!(
                "contestation-candidate:{}",
                digest_ref("polarity-conflict:v1", &[base, positive, negative])
            );
            tx.execute(
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
            )?;
            polarity_conflict_candidate_count += 1;
        }
    }

    // Review pressure is scheduling metadata. Multi-occurrence clusters are
    // surfaced because reconciliation can amortize review across repeated
    // candidate semantics; this is not a truth/importance score.
    let pressure_rows = tx.query(
        r#"
        SELECT 'proposition'::TEXT, proposition_fingerprint_ref,
               COUNT(*)::BIGINT, COUNT(DISTINCT source_revision_ref)::BIGINT
        FROM semantic.proposition_candidate_occurrence
        GROUP BY proposition_fingerprint_ref
        HAVING COUNT(*) > 1
        UNION ALL
        SELECT 'event'::TEXT, event_fingerprint_ref,
               COUNT(*)::BIGINT, COUNT(DISTINCT source_revision_ref)::BIGINT
        FROM semantic.event_candidate_occurrence
        GROUP BY event_fingerprint_ref
        HAVING COUNT(*) > 1
        "#,
        &[],
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
        tx.execute(
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
        )?;
        review_pressure_candidate_count += 1;
    }

    tx.commit()?;

    Ok(CorpusReconciliationReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        statement_count: batches.len(),
        entity_mention_count,
        entity_fingerprint_count: entity_fingerprints.len(),
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
    })
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
