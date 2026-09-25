//! SCALE-1 durable candidate Statement/PNF persistence.
//!
//! Parser output and M12 candidate factors are persistent compilation products,
//! not transient counters. Review/admission remains a separate payment.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{CandidatePnfFactor, CandidatePnfRole, DatabaseConfig, StatementCandidatePnf};

pub const CANDIDATE_PNF_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS pnf;

CREATE TABLE IF NOT EXISTS pnf.statement_candidate_batch (
    batch_ref TEXT PRIMARY KEY,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    exact_span_ref TEXT NOT NULL,
    parser_receipt_ref TEXT NOT NULL,
    factor_count BIGINT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    semantic_admission_paid BOOLEAN NOT NULL CHECK (NOT semantic_admission_paid),
    proposition_support_paid BOOLEAN NOT NULL CHECK (NOT proposition_support_paid),
    applicability_paid BOOLEAN NOT NULL CHECK (NOT applicability_paid),
    claim_truth_paid BOOLEAN NOT NULL CHECK (NOT claim_truth_paid),
    UNIQUE (statement_ref, parser_receipt_ref)
);

CREATE TABLE IF NOT EXISTS pnf.statement_candidate_factor (
    candidate_ref TEXT NOT NULL,
    batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref) ON DELETE CASCADE,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    role_ref TEXT NOT NULL,
    source_start_char BIGINT NOT NULL,
    source_end_char BIGINT NOT NULL,
    surface TEXT NOT NULL,
    lemma TEXT NOT NULL,
    dependency_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    PRIMARY KEY (batch_ref, candidate_ref)
);

CREATE INDEX IF NOT EXISTS statement_candidate_factor_batch_idx
ON pnf.statement_candidate_factor(batch_ref, source_start_char, candidate_ref);
"#;

#[derive(Debug, Error)]
pub enum CandidatePnfStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("candidate Statement/PNF failed validation")]
    InvalidCandidate,
    #[error("candidate Statement row must already be persisted")]
    MissingStatement,
    #[error("existing candidate PNF batch conflicts with immutable candidate")]
    ExistingBatchConflict,
    #[error("existing candidate PNF factor conflicts with immutable candidate")]
    ExistingFactorConflict,
    #[error("unknown candidate PNF role {0}")]
    UnknownRole(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCandidatePnfBatch {
    pub batch_ref: String,
    pub statement_ref: String,
    pub exact_span_ref: String,
    pub parser_receipt_ref: String,
    pub factors: Vec<CandidatePnfFactor>,
    pub candidate_only: bool,
    pub semantic_admission_paid: bool,
    pub proposition_support_paid: bool,
    pub applicability_paid: bool,
    pub claim_truth_paid: bool,
}

fn digest_parts(parts: &[&str]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hasher.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn canonical_candidate_pnf_batch_ref(candidate: &StatementCandidatePnf) -> String {
    let digest = digest_parts(&[
        "statement-candidate-pnf:v1",
        &candidate.statement.statement_ref,
        &candidate.statement.span.span_ref,
        &candidate.parser_receipt_ref,
    ]);
    format!("candidate-pnf-batch:sha256:{}", hex(&digest))
}

fn role_db(role: CandidatePnfRole) -> &'static str {
    match role {
        CandidatePnfRole::Actor => "actor",
        CandidatePnfRole::Predicate => "predicate",
        CandidatePnfRole::Patient => "patient",
        CandidatePnfRole::Qualifier => "qualifier",
        CandidatePnfRole::Other => "other",
    }
}

fn role_from_db(value: &str) -> Result<CandidatePnfRole, CandidatePnfStoreError> {
    match value {
        "actor" => Ok(CandidatePnfRole::Actor),
        "predicate" => Ok(CandidatePnfRole::Predicate),
        "patient" => Ok(CandidatePnfRole::Patient),
        "qualifier" => Ok(CandidatePnfRole::Qualifier),
        "other" => Ok(CandidatePnfRole::Other),
        _ => Err(CandidatePnfStoreError::UnknownRole(value.to_owned())),
    }
}

pub fn install_candidate_pnf_schema(config: &DatabaseConfig) -> Result<(), CandidatePnfStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CANDIDATE_PNF_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_statement_candidate_pnf(
    config: &DatabaseConfig,
    candidate: &StatementCandidatePnf,
) -> Result<PersistedCandidatePnfBatch, CandidatePnfStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CANDIDATE_PNF_SCHEMA_SQL)?;
    persist_statement_candidate_pnf_with_client(&mut client, candidate)
}

/// Persist one immutable candidate batch using an existing client.
///
/// This is intentionally crate-private: the public API keeps the ordinary
/// single-item connection lifecycle, while the book-scale compiler can reuse
/// one client without relaxing any row-level integrity or reopen checks.
pub(crate) fn persist_statement_candidate_pnf_with_client(
    client: &mut Client,
    candidate: &StatementCandidatePnf,
) -> Result<PersistedCandidatePnfBatch, CandidatePnfStoreError> {
    candidate
        .validate()
        .map_err(|_| CandidatePnfStoreError::InvalidCandidate)?;

    let batch_ref = canonical_candidate_pnf_batch_ref(candidate);

    let statement_exists: bool = client
        .query_one(
            "SELECT EXISTS (
               SELECT 1 FROM corpus.source_statement WHERE statement_ref=$1
             )",
            &[&candidate.statement.statement_ref],
        )?
        .get(0);
    if !statement_exists {
        return Err(CandidatePnfStoreError::MissingStatement);
    }

    let mut tx = client.transaction()?;
    tx.execute(
        r#"INSERT INTO pnf.statement_candidate_batch
           (batch_ref, statement_ref, exact_span_ref, parser_receipt_ref,
            factor_count, candidate_only, semantic_admission_paid,
            proposition_support_paid, applicability_paid, claim_truth_paid)
           VALUES ($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT (batch_ref) DO NOTHING"#,
        &[
            &batch_ref,
            &candidate.statement.statement_ref,
            &candidate.statement.span.span_ref,
            &candidate.parser_receipt_ref,
            &(candidate.pnf.candidates.len() as i64),
        ],
    )?;

    let batch = tx.query_one(
        "SELECT statement_ref, exact_span_ref, parser_receipt_ref, factor_count,
                candidate_only, semantic_admission_paid, proposition_support_paid,
                applicability_paid, claim_truth_paid
         FROM pnf.statement_candidate_batch WHERE batch_ref=$1",
        &[&batch_ref],
    )?;
    if batch.get::<_, String>(0) != candidate.statement.statement_ref
        || batch.get::<_, String>(1) != candidate.statement.span.span_ref
        || batch.get::<_, String>(2) != candidate.parser_receipt_ref
        || batch.get::<_, i64>(3) != candidate.pnf.candidates.len() as i64
        || !batch.get::<_, bool>(4)
        || batch.get::<_, bool>(5)
        || batch.get::<_, bool>(6)
        || batch.get::<_, bool>(7)
        || batch.get::<_, bool>(8)
    {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }

    for factor in &candidate.pnf.candidates {
        tx.execute(
            r#"INSERT INTO pnf.statement_candidate_factor
               (candidate_ref, batch_ref, statement_ref, role_ref,
                source_start_char, source_end_char, surface, lemma,
                dependency_ref, candidate_only)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE)
               ON CONFLICT (batch_ref, candidate_ref) DO NOTHING"#,
            &[
                &factor.candidate_ref,
                &batch_ref,
                &candidate.statement.statement_ref,
                &role_db(factor.role),
                &(factor.source_start_char as i64),
                &(factor.source_end_char as i64),
                &factor.surface,
                &factor.lemma,
                &factor.dependency_ref,
            ],
        )?;

        let stored = tx.query_one(
            "SELECT batch_ref, statement_ref, role_ref, source_start_char,
                    source_end_char, surface, lemma, dependency_ref, candidate_only
             FROM pnf.statement_candidate_factor
             WHERE batch_ref=$1 AND candidate_ref=$2",
            &[&batch_ref, &factor.candidate_ref],
        )?;
        if stored.get::<_, String>(0) != batch_ref
            || stored.get::<_, String>(1) != candidate.statement.statement_ref
            || stored.get::<_, String>(2) != role_db(factor.role)
            || stored.get::<_, i64>(3) != factor.source_start_char as i64
            || stored.get::<_, i64>(4) != factor.source_end_char as i64
            || stored.get::<_, String>(5) != factor.surface
            || stored.get::<_, String>(6) != factor.lemma
            || stored.get::<_, String>(7) != factor.dependency_ref
            || !stored.get::<_, bool>(8)
        {
            return Err(CandidatePnfStoreError::ExistingFactorConflict);
        }
    }

    tx.commit()?;
    load_candidate_pnf_batch_with_client(client, &batch_ref)?
        .ok_or(CandidatePnfStoreError::ExistingBatchConflict)
}

pub fn load_candidate_pnf_batch(
    config: &DatabaseConfig,
    batch_ref: &str,
) -> Result<Option<PersistedCandidatePnfBatch>, CandidatePnfStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CANDIDATE_PNF_SCHEMA_SQL)?;
    load_candidate_pnf_batch_with_client(&mut client, batch_ref)
}

pub(crate) fn load_candidate_pnf_batch_with_client(
    client: &mut Client,
    batch_ref: &str,
) -> Result<Option<PersistedCandidatePnfBatch>, CandidatePnfStoreError> {
    let Some(batch) = client.query_opt(
        "SELECT statement_ref, exact_span_ref, parser_receipt_ref, factor_count,
                candidate_only, semantic_admission_paid, proposition_support_paid,
                applicability_paid, claim_truth_paid
         FROM pnf.statement_candidate_batch
         WHERE batch_ref=$1",
        &[&batch_ref],
    )?
    else {
        return Ok(None);
    };

    let rows = client.query(
        "SELECT candidate_ref, role_ref, source_start_char, source_end_char,
                surface, lemma, dependency_ref, candidate_only
         FROM pnf.statement_candidate_factor
         WHERE batch_ref=$1
         ORDER BY source_start_char, source_end_char, candidate_ref",
        &[&batch_ref],
    )?;
    let mut factors = Vec::with_capacity(rows.len());
    for row in rows {
        let start = row.get::<_, i64>(2);
        let end = row.get::<_, i64>(3);
        if start < 0 || end < 0 {
            return Err(CandidatePnfStoreError::ExistingFactorConflict);
        }
        factors.push(CandidatePnfFactor {
            candidate_ref: row.get(0),
            role: role_from_db(&row.get::<_, String>(1))?,
            source_start_char: start as u32,
            source_end_char: end as u32,
            surface: row.get(4),
            lemma: row.get(5),
            dependency_ref: row.get(6),
            candidate_only: row.get(7),
        });
    }

    let expected_factor_count = batch.get::<_, i64>(3);
    if expected_factor_count < 0 || expected_factor_count as usize != factors.len() {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }

    Ok(Some(PersistedCandidatePnfBatch {
        batch_ref: batch_ref.to_owned(),
        statement_ref: batch.get(0),
        exact_span_ref: batch.get(1),
        parser_receipt_ref: batch.get(2),
        factors,
        candidate_only: batch.get(4),
        semantic_admission_paid: batch.get(5),
        proposition_support_paid: batch.get(6),
        applicability_paid: batch.get(7),
        claim_truth_paid: batch.get(8),
    }))
}

pub fn load_candidate_pnf_batches_for_statement(
    config: &DatabaseConfig,
    statement_ref: &str,
) -> Result<Vec<PersistedCandidatePnfBatch>, CandidatePnfStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CANDIDATE_PNF_SCHEMA_SQL)?;
    let refs = client
        .query(
            "SELECT batch_ref FROM pnf.statement_candidate_batch
             WHERE statement_ref=$1 ORDER BY batch_ref",
            &[&statement_ref],
        )?
        .into_iter()
        .map(|row| row.get::<_, String>(0))
        .collect::<Vec<_>>();
    refs.into_iter()
        .map(|batch_ref| {
            load_candidate_pnf_batch(config, &batch_ref)?
                .ok_or(CandidatePnfStoreError::ExistingBatchConflict)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CandidatePnfBatch, ExactSourceSpan, SourceStatementEnvelope, StatementOrigin};

    #[test]
    fn batch_identity_depends_on_statement_span_and_parser_receipt() {
        let candidate = StatementCandidatePnf {
            statement: SourceStatementEnvelope {
                statement_ref: "statement:x".into(),
                document_ref: "document:x".into(),
                source_revision_ref: "revision:x".into(),
                span: ExactSourceSpan {
                    span_ref: "span:x".into(),
                    start_char: 0,
                    end_char: 3,
                },
                literal_text: "abc".into(),
                origin: StatementOrigin::InitialIntake,
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
            pnf: CandidatePnfBatch {
                exact_span_ref: "span:x".into(),
                candidates: vec![],
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
        };
        assert_eq!(
            canonical_candidate_pnf_batch_ref(&candidate),
            canonical_candidate_pnf_batch_ref(&candidate)
        );
    }
}
