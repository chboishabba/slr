//! SCALE-1 durable candidate Statement/PNF persistence.
//!
//! Parser output and M12 candidate factors are persistent compilation products,
//! not transient counters. Review/admission remains a separate payment.

use postgres::{Client, GenericClient, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{CandidatePnfFactor, CandidatePnfRole, DatabaseConfig, StatementCandidatePnf};

pub const CANDIDATE_PNF_COMPILER_REF: &str =
    "scale1:m12-candidate-pnf:v2-productized";

pub const CANDIDATE_PNF_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS pnf;

CREATE TABLE IF NOT EXISTS pnf.candidate_semantic_product (
    product_ref TEXT PRIMARY KEY,
    parser_product_key TEXT NOT NULL,
    compiler_ref TEXT NOT NULL,
    factor_count BIGINT NOT NULL,
    exact_reopen_validated BOOLEAN NOT NULL CHECK (exact_reopen_validated),
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    UNIQUE (parser_product_key, compiler_ref)
);

ALTER TABLE pnf.candidate_semantic_product
  ADD COLUMN IF NOT EXISTS exact_reopen_validated BOOLEAN NOT NULL DEFAULT TRUE
  CHECK (exact_reopen_validated);

CREATE TABLE IF NOT EXISTS pnf.candidate_semantic_product_factor (
    product_ref TEXT NOT NULL
      REFERENCES pnf.candidate_semantic_product(product_ref) ON DELETE CASCADE,
    factor_ordinal INTEGER NOT NULL,
    token_ordinal INTEGER NOT NULL,
    start_offset BIGINT NOT NULL,
    end_offset BIGINT NOT NULL,
    role_ref TEXT NOT NULL,
    surface TEXT NOT NULL,
    lemma TEXT NOT NULL,
    dependency_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    PRIMARY KEY (product_ref, factor_ordinal)
);

CREATE TABLE IF NOT EXISTS pnf.statement_candidate_batch (
    batch_ref TEXT PRIMARY KEY,
    statement_ref TEXT NOT NULL REFERENCES corpus.source_statement(statement_ref) ON DELETE CASCADE,
    exact_span_ref TEXT NOT NULL,
    parser_receipt_ref TEXT NOT NULL,
    factor_count BIGINT NOT NULL,
    candidate_product_ref TEXT NULL,
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    semantic_admission_paid BOOLEAN NOT NULL CHECK (NOT semantic_admission_paid),
    proposition_support_paid BOOLEAN NOT NULL CHECK (NOT proposition_support_paid),
    applicability_paid BOOLEAN NOT NULL CHECK (NOT applicability_paid),
    claim_truth_paid BOOLEAN NOT NULL CHECK (NOT claim_truth_paid),
    UNIQUE (statement_ref, parser_receipt_ref)
);

ALTER TABLE pnf.statement_candidate_batch
  ADD COLUMN IF NOT EXISTS candidate_product_ref TEXT NULL;

CREATE INDEX IF NOT EXISTS statement_candidate_batch_product_idx
ON pnf.statement_candidate_batch(candidate_product_ref);

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

CREATE TABLE IF NOT EXISTS pnf.candidate_persistence_stage_receipt (
    source_revision_ref TEXT NOT NULL,
    parser_run_ref TEXT NOT NULL,
    compiler_ref TEXT NOT NULL,
    statement_count BIGINT NOT NULL,
    batch_count BIGINT NOT NULL,
    factor_count BIGINT NOT NULL,
    exact_reopen_validated BOOLEAN NOT NULL CHECK (exact_reopen_validated),
    candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
    creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
    applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
    claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
    PRIMARY KEY (source_revision_ref, parser_run_ref, compiler_ref)
);

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
pub struct CandidatePersistenceStageReceipt {
    pub source_revision_ref: String,
    pub parser_run_ref: String,
    pub compiler_ref: String,
    pub statement_count: usize,
    pub batch_count: usize,
    pub factor_count: usize,
    pub exact_reopen_validated: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CandidateProductPersistenceOutcome {
    pub product_backed: bool,
    pub reused_product: bool,
    pub product_factor_rows_inserted: usize,
    pub batch_row_inserted: bool,
}

#[derive(Debug, Clone)]
struct CandidateProductContext {
    product_ref: String,
    parser_product_key: String,
    parser_run_ref: String,
    compilation_key: String,
    region_start_char: i64,
}

fn candidate_product_context<C: GenericClient>(
    client: &mut C,
    candidate: &StatementCandidatePnf,
) -> Result<Option<CandidateProductContext>, CandidatePnfStoreError> {
    let row = client.query_opt(
        r#"
        SELECT j.parser_product_key, j.parser_run_ref, j.compilation_key,
               j.region_start_char, j.candidate_only,
               j.creates_semantic_authority, j.applicability_promoted,
               j.claim_truth_promoted
        FROM ingest.parser_job j
        WHERE j.source_revision_ref=$1
          AND j.region_ref=$2
          AND ('db-parser:' || j.parser_run_ref || ':' || j.region_ref)=$3
          AND j.status='succeeded'
        "#,
        &[
            &candidate.statement.source_revision_ref,
            &candidate.statement.span.span_ref,
            &candidate.parser_receipt_ref,
        ],
    )?;
    let Some(row) = row else {
        return Ok(None);
    };
    let parser_product_key: Option<String> = row.get(0);
    let Some(parser_product_key) = parser_product_key else {
        return Ok(None);
    };
    if !row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, bool>(7)
    {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }
    let digest = digest_parts(&[
        "candidate-semantic-product:v1",
        &parser_product_key,
        CANDIDATE_PNF_COMPILER_REF,
    ]);
    Ok(Some(CandidateProductContext {
        product_ref: format!("candidate-semantic-product:sha256:{}", hex(&digest)),
        parser_product_key,
        parser_run_ref: row.get(1),
        compilation_key: row.get(2),
        region_start_char: row.get(3),
    }))
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

pub(crate) fn load_candidate_persistence_stage_receipt_with_client(
    client: &mut Client,
    source_revision_ref: &str,
    parser_run_ref: &str,
) -> Result<Option<CandidatePersistenceStageReceipt>, CandidatePnfStoreError> {
    let Some(row) = client.query_opt(
        r#"
        SELECT compiler_ref, statement_count, batch_count, factor_count,
               exact_reopen_validated, candidate_only,
               creates_semantic_authority, applicability_promoted,
               claim_truth_promoted
        FROM pnf.candidate_persistence_stage_receipt
        WHERE source_revision_ref=$1 AND parser_run_ref=$2 AND compiler_ref=$3
        "#,
        &[&source_revision_ref, &parser_run_ref, &CANDIDATE_PNF_COMPILER_REF],
    )? else {
        return Ok(None);
    };
    let receipt = CandidatePersistenceStageReceipt {
        source_revision_ref: source_revision_ref.to_owned(),
        parser_run_ref: parser_run_ref.to_owned(),
        compiler_ref: row.get(0),
        statement_count: row.get::<_, i64>(1).max(0) as usize,
        batch_count: row.get::<_, i64>(2).max(0) as usize,
        factor_count: row.get::<_, i64>(3).max(0) as usize,
        exact_reopen_validated: row.get(4),
        candidate_only: row.get(5),
        creates_semantic_authority: row.get(6),
        applicability_promoted: row.get(7),
        claim_truth_promoted: row.get(8),
    };
    if receipt.compiler_ref != CANDIDATE_PNF_COMPILER_REF
        || !receipt.exact_reopen_validated
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }
    Ok(Some(receipt))
}

pub(crate) fn persist_candidate_persistence_stage_receipt_with_client(
    client: &mut Client,
    source_revision_ref: &str,
    parser_run_ref: &str,
    statement_count: usize,
    batch_count: usize,
    factor_count: usize,
) -> Result<CandidatePersistenceStageReceipt, CandidatePnfStoreError> {
    client.execute(
        r#"
        INSERT INTO pnf.candidate_persistence_stage_receipt
        (source_revision_ref, parser_run_ref, compiler_ref,
         statement_count, batch_count, factor_count,
         exact_reopen_validated, candidate_only,
         creates_semantic_authority, applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,TRUE,TRUE,FALSE,FALSE,FALSE)
        ON CONFLICT (source_revision_ref, parser_run_ref, compiler_ref) DO NOTHING
        "#,
        &[
            &source_revision_ref,
            &parser_run_ref,
            &CANDIDATE_PNF_COMPILER_REF,
            &(statement_count as i64),
            &(batch_count as i64),
            &(factor_count as i64),
        ],
    )?;
    load_candidate_persistence_stage_receipt_with_client(
        client,
        source_revision_ref,
        parser_run_ref,
    )?
    .ok_or(CandidatePnfStoreError::ExistingBatchConflict)
}

pub fn install_candidate_pnf_schema(config: &DatabaseConfig) -> Result<(), CandidatePnfStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(CANDIDATE_PNF_SCHEMA_SQL)?;
    Ok(())
}

fn persist_candidate_semantic_product_in_scope<C: GenericClient>(
    client: &mut C,
    candidate: &StatementCandidatePnf,
    context: &CandidateProductContext,
) -> Result<CandidateProductPersistenceOutcome, CandidatePnfStoreError> {
    if let Some(stored) = client.query_opt(
        r#"
        SELECT parser_product_key, compiler_ref, factor_count,
               exact_reopen_validated, candidate_only,
               creates_semantic_authority, applicability_promoted,
               claim_truth_promoted
        FROM pnf.candidate_semantic_product
        WHERE product_ref=$1
        "#,
        &[&context.product_ref],
    )? {
        if stored.get::<_, String>(0) != context.parser_product_key
            || stored.get::<_, String>(1) != CANDIDATE_PNF_COMPILER_REF
            || stored.get::<_, i64>(2) != candidate.pnf.candidates.len() as i64
            || !stored.get::<_, bool>(3)
            || !stored.get::<_, bool>(4)
            || stored.get::<_, bool>(5)
            || stored.get::<_, bool>(6)
            || stored.get::<_, bool>(7)
        {
            return Err(CandidatePnfStoreError::ExistingBatchConflict);
        }
        return Ok(CandidateProductPersistenceOutcome {
            product_backed: true,
            reused_product: true,
            product_factor_rows_inserted: 0,
            batch_row_inserted: false,
        });
    }

    let token_rows = client.query(
        r#"
        SELECT token_ordinal, start_char, end_char
        FROM ingest.parser_token
        WHERE compilation_key=$1
        ORDER BY token_ordinal
        "#,
        &[&context.compilation_key],
    )?;
    if token_rows.len() != candidate.pnf.candidates.len() {
        return Err(CandidatePnfStoreError::ExistingFactorConflict);
    }

    let mut token_ordinals = Vec::with_capacity(token_rows.len());
    let mut starts = Vec::with_capacity(token_rows.len());
    let mut ends = Vec::with_capacity(token_rows.len());
    for (row, factor) in token_rows.iter().zip(candidate.pnf.candidates.iter()) {
        let token_ordinal: i32 = row.get(0);
        let token_start: i64 = row.get(1);
        let token_end: i64 = row.get(2);
        let start_offset = token_start - context.region_start_char;
        let end_offset = token_end - context.region_start_char;
        if token_ordinal < 0
            || token_start != factor.source_start_char as i64
            || token_end != factor.source_end_char as i64
            || start_offset < 0
            || end_offset <= start_offset
        {
            return Err(CandidatePnfStoreError::ExistingFactorConflict);
        }
        token_ordinals.push(token_ordinal);
        starts.push(start_offset);
        ends.push(end_offset);
    }

    let ordinals = (0..candidate.pnf.candidates.len())
        .map(|value| value as i32)
        .collect::<Vec<_>>();
    let roles = candidate
        .pnf
        .candidates
        .iter()
        .map(|factor| role_db(factor.role).to_owned())
        .collect::<Vec<_>>();
    let surfaces = candidate
        .pnf
        .candidates
        .iter()
        .map(|factor| factor.surface.clone())
        .collect::<Vec<_>>();
    let lemmas = candidate
        .pnf
        .candidates
        .iter()
        .map(|factor| factor.lemma.clone())
        .collect::<Vec<_>>();
    let dependencies = candidate
        .pnf
        .candidates
        .iter()
        .map(|factor| factor.dependency_ref.clone())
        .collect::<Vec<_>>();

    let product_inserted = client.execute(
        r#"
        INSERT INTO pnf.candidate_semantic_product
          (product_ref, parser_product_key, compiler_ref, factor_count,
           exact_reopen_validated, candidate_only, creates_semantic_authority,
           applicability_promoted, claim_truth_promoted)
        VALUES ($1,$2,$3,$4,TRUE,TRUE,FALSE,FALSE,FALSE)
        ON CONFLICT (product_ref) DO NOTHING
        "#,
        &[
            &context.product_ref,
            &context.parser_product_key,
            &CANDIDATE_PNF_COMPILER_REF,
            &(candidate.pnf.candidates.len() as i64),
        ],
    )?;

    let mut product_factor_rows_inserted = 0usize;
    if !candidate.pnf.candidates.is_empty() {
        let product_refs = vec![context.product_ref.clone(); ordinals.len()];
        product_factor_rows_inserted = client.execute(
            r#"
            INSERT INTO pnf.candidate_semantic_product_factor
              (product_ref, factor_ordinal, token_ordinal,
               start_offset, end_offset, role_ref,
               surface, lemma, dependency_ref, candidate_only)
            SELECT product_ref, factor_ordinal, token_ordinal,
                   start_offset, end_offset, role_ref,
                   surface, lemma, dependency_ref, TRUE
            FROM UNNEST(
              $1::TEXT[], $2::INTEGER[], $3::INTEGER[],
              $4::BIGINT[], $5::BIGINT[], $6::TEXT[],
              $7::TEXT[], $8::TEXT[], $9::TEXT[]
            ) AS u(
              product_ref, factor_ordinal, token_ordinal,
              start_offset, end_offset, role_ref,
              surface, lemma, dependency_ref
            )
            ON CONFLICT (product_ref, factor_ordinal) DO NOTHING
            "#,
            &[
                &product_refs,
                &ordinals,
                &token_ordinals,
                &starts,
                &ends,
                &roles,
                &surfaces,
                &lemmas,
                &dependencies,
            ],
        )? as usize;
    }

    let stored = client.query_one(
        r#"
        SELECT parser_product_key, compiler_ref, factor_count,
               exact_reopen_validated, candidate_only,
               creates_semantic_authority, applicability_promoted,
               claim_truth_promoted
        FROM pnf.candidate_semantic_product
        WHERE product_ref=$1
        "#,
        &[&context.product_ref],
    )?;
    if stored.get::<_, String>(0) != context.parser_product_key
        || stored.get::<_, String>(1) != CANDIDATE_PNF_COMPILER_REF
        || stored.get::<_, i64>(2) != candidate.pnf.candidates.len() as i64
        || !stored.get::<_, bool>(3)
        || !stored.get::<_, bool>(4)
        || stored.get::<_, bool>(5)
        || stored.get::<_, bool>(6)
        || stored.get::<_, bool>(7)
    {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }

    let factor_rows = client.query(
        r#"
        SELECT factor_ordinal, token_ordinal, start_offset, end_offset,
               role_ref, surface, lemma, dependency_ref, candidate_only
        FROM pnf.candidate_semantic_product_factor
        WHERE product_ref=$1
        ORDER BY factor_ordinal
        "#,
        &[&context.product_ref],
    )?;
    if factor_rows.len() != candidate.pnf.candidates.len() {
        return Err(CandidatePnfStoreError::ExistingFactorConflict);
    }
    for (index, (row, factor)) in factor_rows
        .iter()
        .zip(candidate.pnf.candidates.iter())
        .enumerate()
    {
        if row.get::<_, i32>(0) != index as i32
            || row.get::<_, i32>(1) != token_ordinals[index]
            || row.get::<_, i64>(2) != starts[index]
            || row.get::<_, i64>(3) != ends[index]
            || row.get::<_, String>(4) != role_db(factor.role)
            || row.get::<_, String>(5) != factor.surface
            || row.get::<_, String>(6) != factor.lemma
            || row.get::<_, String>(7) != factor.dependency_ref
            || !row.get::<_, bool>(8)
        {
            return Err(CandidatePnfStoreError::ExistingFactorConflict);
        }
    }

    Ok(CandidateProductPersistenceOutcome {
        product_backed: true,
        reused_product: product_inserted == 0,
        product_factor_rows_inserted,
        batch_row_inserted: false,
    })
}

pub(crate) fn persist_statement_candidate_pnf_in_scope<C: GenericClient>(
    client: &mut C,
    candidate: &StatementCandidatePnf,
) -> Result<(String, CandidateProductPersistenceOutcome), CandidatePnfStoreError> {
    candidate
        .validate()
        .map_err(|_| CandidatePnfStoreError::InvalidCandidate)?;

    let batch_ref = canonical_candidate_pnf_batch_ref(candidate);
    let product_context = candidate_product_context(client, candidate)?;
    let product_outcome = if let Some(context) = product_context.as_ref() {
        persist_candidate_semantic_product_in_scope(client, candidate, context)?
    } else {
        CandidateProductPersistenceOutcome {
            product_backed: false,
            reused_product: false,
            product_factor_rows_inserted: 0,
            batch_row_inserted: false,
        }
    };
    let candidate_product_ref = product_context
        .as_ref()
        .map(|context| context.product_ref.clone());

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

    let batch_inserted = client.execute(
        r#"INSERT INTO pnf.statement_candidate_batch
           (batch_ref, statement_ref, exact_span_ref, parser_receipt_ref,
            factor_count, candidate_product_ref, candidate_only,
            semantic_admission_paid, proposition_support_paid,
            applicability_paid, claim_truth_paid)
           VALUES ($1,$2,$3,$4,$5,$6,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT (batch_ref) DO NOTHING"#,
        &[
            &batch_ref,
            &candidate.statement.statement_ref,
            &candidate.statement.span.span_ref,
            &candidate.parser_receipt_ref,
            &(candidate.pnf.candidates.len() as i64),
            &candidate_product_ref,
        ],
    )?;
    if let Some(product_ref) = candidate_product_ref.as_ref() {
        client.execute(
            "UPDATE pnf.statement_candidate_batch
             SET candidate_product_ref=$2
             WHERE batch_ref=$1 AND candidate_product_ref IS NULL",
            &[&batch_ref, product_ref],
        )?;
    }

    let batch = client.query_one(
        "SELECT statement_ref, exact_span_ref, parser_receipt_ref, factor_count,
                candidate_product_ref, candidate_only, semantic_admission_paid,
                proposition_support_paid, applicability_paid, claim_truth_paid
         FROM pnf.statement_candidate_batch WHERE batch_ref=$1",
        &[&batch_ref],
    )?;
    if batch.get::<_, String>(0) != candidate.statement.statement_ref
        || batch.get::<_, String>(1) != candidate.statement.span.span_ref
        || batch.get::<_, String>(2) != candidate.parser_receipt_ref
        || batch.get::<_, i64>(3) != candidate.pnf.candidates.len() as i64
        || batch.get::<_, Option<String>>(4) != candidate_product_ref
        || !batch.get::<_, bool>(5)
        || batch.get::<_, bool>(6)
        || batch.get::<_, bool>(7)
        || batch.get::<_, bool>(8)
        || batch.get::<_, bool>(9)
    {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }

    if candidate_product_ref.is_none() && !candidate.pnf.candidates.is_empty() {
        let candidate_refs = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| factor.candidate_ref.clone())
            .collect::<Vec<_>>();
        let batch_refs = vec![batch_ref.clone(); candidate.pnf.candidates.len()];
        let statement_refs = vec![
            candidate.statement.statement_ref.clone();
            candidate.pnf.candidates.len()
        ];
        let role_refs = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| role_db(factor.role).to_owned())
            .collect::<Vec<_>>();
        let starts = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| factor.source_start_char as i64)
            .collect::<Vec<_>>();
        let ends = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| factor.source_end_char as i64)
            .collect::<Vec<_>>();
        let surfaces = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| factor.surface.clone())
            .collect::<Vec<_>>();
        let lemmas = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| factor.lemma.clone())
            .collect::<Vec<_>>();
        let dependencies = candidate
            .pnf
            .candidates
            .iter()
            .map(|factor| factor.dependency_ref.clone())
            .collect::<Vec<_>>();

        client.execute(
            r#"
            INSERT INTO pnf.statement_candidate_factor
              (candidate_ref, batch_ref, statement_ref, role_ref,
               source_start_char, source_end_char, surface, lemma,
               dependency_ref, candidate_only)
            SELECT candidate_ref, batch_ref, statement_ref, role_ref,
                   source_start_char, source_end_char, surface, lemma,
                   dependency_ref, TRUE
            FROM UNNEST(
              $1::TEXT[], $2::TEXT[], $3::TEXT[], $4::TEXT[],
              $5::BIGINT[], $6::BIGINT[], $7::TEXT[], $8::TEXT[], $9::TEXT[]
            ) AS u(
              candidate_ref, batch_ref, statement_ref, role_ref,
              source_start_char, source_end_char, surface, lemma, dependency_ref
            )
            ON CONFLICT (batch_ref, candidate_ref) DO NOTHING
            "#,
            &[
                &candidate_refs,
                &batch_refs,
                &statement_refs,
                &role_refs,
                &starts,
                &ends,
                &surfaces,
                &lemmas,
                &dependencies,
            ],
        )?;
    }

    let mut outcome = product_outcome;
    outcome.batch_row_inserted = batch_inserted == 1;
    Ok((batch_ref, outcome))
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
    persist_statement_candidate_pnf_with_client_detailed(client, candidate)
        .map(|(batch, _)| batch)
}

pub(crate) fn persist_statement_candidate_pnf_with_client_detailed(
    client: &mut Client,
    candidate: &StatementCandidatePnf,
) -> Result<
    (PersistedCandidatePnfBatch, CandidateProductPersistenceOutcome),
    CandidatePnfStoreError,
> {
    candidate
        .validate()
        .map_err(|_| CandidatePnfStoreError::InvalidCandidate)?;
    let mut tx = client.transaction()?;
    let (batch_ref, outcome) =
        persist_statement_candidate_pnf_in_scope(&mut tx, candidate)?;
    tx.commit()?;

    let persisted = load_candidate_pnf_batch_with_client(client, &batch_ref)?
        .ok_or(CandidatePnfStoreError::ExistingBatchConflict)?;
    let mut expected_factors = candidate.pnf.candidates.clone();
    expected_factors.sort_by(|left, right| {
        (
            left.source_start_char,
            left.source_end_char,
            left.candidate_ref.as_str(),
        )
            .cmp(&(
                right.source_start_char,
                right.source_end_char,
                right.candidate_ref.as_str(),
            ))
    });
    if persisted.statement_ref != candidate.statement.statement_ref
        || persisted.exact_span_ref != candidate.statement.span.span_ref
        || persisted.parser_receipt_ref != candidate.parser_receipt_ref
        || persisted.factors != expected_factors
        || !persisted.candidate_only
        || persisted.semantic_admission_paid
        || persisted.proposition_support_paid
        || persisted.applicability_paid
        || persisted.claim_truth_paid
    {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }
    Ok((persisted, outcome))
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
        r#"
        SELECT b.statement_ref, b.exact_span_ref, b.parser_receipt_ref,
               b.factor_count, b.candidate_product_ref,
               b.candidate_only, b.semantic_admission_paid,
               b.proposition_support_paid, b.applicability_paid,
               b.claim_truth_paid, s.source_revision_ref
        FROM pnf.statement_candidate_batch b
        JOIN corpus.source_statement s ON s.statement_ref=b.statement_ref
        WHERE b.batch_ref=$1
        "#,
        &[&batch_ref],
    )? else {
        return Ok(None);
    };

    let statement_ref: String = batch.get(0);
    let exact_span_ref: String = batch.get(1);
    let parser_receipt_ref: String = batch.get(2);
    let candidate_product_ref: Option<String> = batch.get(4);
    let source_revision_ref: String = batch.get(10);

    let mut factors = Vec::new();
    if let Some(product_ref) = candidate_product_ref.as_ref() {
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
            return Err(CandidatePnfStoreError::ExistingBatchConflict);
        };
        if !parser_job.get::<_, bool>(2)
            || parser_job.get::<_, bool>(3)
            || parser_job.get::<_, bool>(4)
            || parser_job.get::<_, bool>(5)
        {
            return Err(CandidatePnfStoreError::ExistingBatchConflict);
        }
        let parser_run_ref: String = parser_job.get(0);
        let region_start: i64 = parser_job.get(1);
        if region_start < 0 {
            return Err(CandidatePnfStoreError::ExistingFactorConflict);
        }

        let rows = client.query(
            r#"
            SELECT factor_ordinal, token_ordinal, start_offset, end_offset,
                   role_ref, surface, lemma, dependency_ref, candidate_only
            FROM pnf.candidate_semantic_product_factor
            WHERE product_ref=$1
            ORDER BY factor_ordinal
            "#,
            &[product_ref],
        )?;
        factors.reserve(rows.len());
        for row in rows {
            let token_ordinal: i32 = row.get(1);
            let start_offset: i64 = row.get(2);
            let end_offset: i64 = row.get(3);
            let start = region_start + start_offset;
            let end = region_start + end_offset;
            if token_ordinal < 0
                || start_offset < 0
                || end_offset <= start_offset
                || start < 0
                || end < 0
            {
                return Err(CandidatePnfStoreError::ExistingFactorConflict);
            }
            factors.push(CandidatePnfFactor {
                candidate_ref: format!(
                    "candidate-pnf-db:{}:{}:{}:{}:{}",
                    parser_run_ref,
                    exact_span_ref,
                    token_ordinal,
                    start,
                    end
                ),
                role: role_from_db(&row.get::<_, String>(4))?,
                source_start_char: start as u32,
                source_end_char: end as u32,
                surface: row.get(5),
                lemma: row.get(6),
                dependency_ref: row.get(7),
                candidate_only: row.get(8),
            });
        }
    } else {
        let rows = client.query(
            "SELECT candidate_ref, role_ref, source_start_char, source_end_char,
                    surface, lemma, dependency_ref, candidate_only
             FROM pnf.statement_candidate_factor
             WHERE batch_ref=$1
             ORDER BY source_start_char, source_end_char, candidate_ref",
            &[&batch_ref],
        )?;
        factors.reserve(rows.len());
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
    }

    let expected_factor_count = batch.get::<_, i64>(3);
    if expected_factor_count < 0 || expected_factor_count as usize != factors.len() {
        return Err(CandidatePnfStoreError::ExistingBatchConflict);
    }

    Ok(Some(PersistedCandidatePnfBatch {
        batch_ref: batch_ref.to_owned(),
        statement_ref,
        exact_span_ref,
        parser_receipt_ref,
        factors,
        candidate_only: batch.get(5),
        semantic_admission_paid: batch.get(6),
        proposition_support_paid: batch.get(7),
        applicability_paid: batch.get(8),
        claim_truth_paid: batch.get(9),
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
