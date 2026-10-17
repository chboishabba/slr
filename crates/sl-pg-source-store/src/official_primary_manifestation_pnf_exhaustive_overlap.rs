//! Exhaustive Predicate-PNF coverage for two official manifestations of one judgment.
//!
//! The earlier v2 overlap receipt proves compatibility around a selected
//! predicate fibre.  This v3 receipt is the release-grade completeness claim:
//! it compares the full multiset of every non-`Other` candidate PNF factor under
//! the declared `(role, lemma, dependency)` projection.  Parser/OCR layout debris
//! remains counted separately and cannot affect semantic equality.
//!
//! Exact exhaustive overlap may close only the source-transformation residual.
//! It never creates a second witness, semantic/legal authority, applicability,
//! or claim truth.

use std::collections::BTreeMap;

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    install_official_primary_manifestation_pnf_overlap_schema, load_candidate_pnf_batch,
    load_official_primary_manifestation_pnf_overlap, CandidatePnfFactor, CandidatePnfRole,
    CandidatePnfStoreError, DatabaseConfig, ManifestationPnfOverlapDisposition,
    OfficialPrimaryManifestationPnfOverlapError,
};

pub const EXHAUSTIVE_PNF_PROJECTION_REF: &str =
    "predicate-pnf-semantic-multiset:(role,lemma,dependency):v1";
pub const EXHAUSTIVE_PNF_COMPARATOR_REVISION_REF: &str =
    "official-primary-manifestation-predicate-pnf-exhaustive-overlap:v3";

pub const OFFICIAL_PRIMARY_MANIFESTATION_PNF_EXHAUSTIVE_OVERLAP_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;

CREATE TABLE IF NOT EXISTS source_provenance.official_primary_manifestation_pnf_exhaustive_overlap (
  exhaustive_overlap_ref TEXT PRIMARY KEY,
  overlap_ref TEXT NOT NULL
    REFERENCES source_provenance.official_primary_manifestation_pnf_overlap(overlap_ref),
  comparator_revision_ref TEXT NOT NULL,
  projection_ref TEXT NOT NULL,
  pdf_batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref),
  docx_batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref),
  pdf_parser_factor_count BIGINT NOT NULL CHECK (pdf_parser_factor_count >= 0),
  docx_parser_factor_count BIGINT NOT NULL CHECK (docx_parser_factor_count >= 0),
  pdf_semantic_factor_count BIGINT NOT NULL CHECK (pdf_semantic_factor_count >= 0),
  docx_semantic_factor_count BIGINT NOT NULL CHECK (docx_semantic_factor_count >= 0),
  matched_semantic_factor_count BIGINT NOT NULL CHECK (matched_semantic_factor_count >= 0),
  unmatched_pdf_semantic_count BIGINT NOT NULL CHECK (unmatched_pdf_semantic_count >= 0),
  unmatched_docx_semantic_count BIGINT NOT NULL CHECK (unmatched_docx_semantic_count >= 0),
  ignored_pdf_nonsemantic_count BIGINT NOT NULL CHECK (ignored_pdf_nonsemantic_count >= 0),
  ignored_docx_nonsemantic_count BIGINT NOT NULL CHECK (ignored_docx_nonsemantic_count >= 0),
  semantic_factor_multiset_exact BOOLEAN NOT NULL,
  closes_source_transformation_residual BOOLEAN NOT NULL,
  requires_human_pnf_review BOOLEAN NOT NULL,
  creates_legal_controversy BOOLEAN NOT NULL CHECK (NOT creates_legal_controversy),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  independent_source_count_increment INTEGER NOT NULL DEFAULT 0
    CHECK (independent_source_count_increment = 0),
  UNIQUE(overlap_ref, comparator_revision_ref)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticPnfCoverage {
    pub projection_ref: String,
    pub pdf_parser_factor_count: usize,
    pub docx_parser_factor_count: usize,
    pub pdf_semantic_factor_count: usize,
    pub docx_semantic_factor_count: usize,
    pub matched_semantic_factor_count: usize,
    pub unmatched_pdf_semantic_count: usize,
    pub unmatched_docx_semantic_count: usize,
    pub ignored_pdf_nonsemantic_count: usize,
    pub ignored_docx_nonsemantic_count: usize,
    pub semantic_factor_multiset_exact: bool,
    pub closes_source_transformation_residual: bool,
    pub requires_human_pnf_review: bool,
    pub creates_legal_controversy: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub independent_source_count_increment: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialPrimaryManifestationPnfExhaustiveOverlapReceipt {
    pub exhaustive_overlap_ref: String,
    pub overlap_ref: String,
    pub comparator_revision_ref: String,
    pub pdf_batch_ref: String,
    pub docx_batch_ref: String,
    pub coverage: SemanticPnfCoverage,
}

#[derive(Debug, Error)]
pub enum OfficialPrimaryManifestationPnfExhaustiveOverlapError {
    #[error("selected-fibre PNF overlap error: {0}")]
    Overlap(#[from] OfficialPrimaryManifestationPnfOverlapError),
    #[error("candidate PNF error: {0}")]
    Candidate(#[from] CandidatePnfStoreError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("candidate PNF batch does not exist: {0}")]
    MissingBatch(String),
    #[error("base overlap and candidate-PNF batch coordinates disagree")]
    CoordinateMismatch,
    #[error("candidate PNF batch crossed the non-promotion boundary")]
    Promotion,
    #[error("semantic factor counts cannot be represented by PostgreSQL BIGINT")]
    CountOverflow,
    #[error("persisted exhaustive overlap conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("exhaustive overlap not found: {0}")]
    NotFound(String),
}

fn role_tag(role: CandidatePnfRole) -> &'static str {
    match role {
        CandidatePnfRole::Actor => "actor",
        CandidatePnfRole::Predicate => "predicate",
        CandidatePnfRole::Patient => "patient",
        CandidatePnfRole::Qualifier => "qualifier",
        CandidatePnfRole::Other => "other",
    }
}

fn semantic_multiset(
    factors: &[CandidatePnfFactor],
) -> BTreeMap<(String, String, String), usize> {
    let mut counts = BTreeMap::new();
    for factor in factors.iter().filter(|factor| factor.role != CandidatePnfRole::Other) {
        *counts
            .entry((
                role_tag(factor.role).to_owned(),
                factor.lemma.clone(),
                factor.dependency_ref.clone(),
            ))
            .or_insert(0) += 1;
    }
    counts
}

/// Measure complete semantic coverage, preserving multiplicity.
#[must_use]
pub fn measure_semantic_pnf_coverage(
    pdf: &[CandidatePnfFactor],
    docx: &[CandidatePnfFactor],
) -> SemanticPnfCoverage {
    let pdf_multiset = semantic_multiset(pdf);
    let docx_multiset = semantic_multiset(docx);
    let pdf_semantic_factor_count = pdf_multiset.values().sum::<usize>();
    let docx_semantic_factor_count = docx_multiset.values().sum::<usize>();

    let mut matched_semantic_factor_count = 0usize;
    for (signature, pdf_count) in &pdf_multiset {
        let docx_count = docx_multiset.get(signature).copied().unwrap_or(0);
        matched_semantic_factor_count += (*pdf_count).min(docx_count);
    }
    let unmatched_pdf_semantic_count =
        pdf_semantic_factor_count.saturating_sub(matched_semantic_factor_count);
    let unmatched_docx_semantic_count =
        docx_semantic_factor_count.saturating_sub(matched_semantic_factor_count);
    let semantic_factor_multiset_exact = pdf_multiset == docx_multiset;

    SemanticPnfCoverage {
        projection_ref: EXHAUSTIVE_PNF_PROJECTION_REF.into(),
        pdf_parser_factor_count: pdf.len(),
        docx_parser_factor_count: docx.len(),
        pdf_semantic_factor_count,
        docx_semantic_factor_count,
        matched_semantic_factor_count,
        unmatched_pdf_semantic_count,
        unmatched_docx_semantic_count,
        ignored_pdf_nonsemantic_count: pdf.len().saturating_sub(pdf_semantic_factor_count),
        ignored_docx_nonsemantic_count: docx.len().saturating_sub(docx_semantic_factor_count),
        semantic_factor_multiset_exact,
        closes_source_transformation_residual: semantic_factor_multiset_exact,
        requires_human_pnf_review: !semantic_factor_multiset_exact,
        creates_legal_controversy: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        independent_source_count_increment: 0,
    }
}

fn count_i64(value: usize) -> Result<i64, OfficialPrimaryManifestationPnfExhaustiveOverlapError> {
    i64::try_from(value).map_err(|_| OfficialPrimaryManifestationPnfExhaustiveOverlapError::CountOverflow)
}

fn count_usize(value: i64) -> Result<usize, OfficialPrimaryManifestationPnfExhaustiveOverlapError> {
    if value < 0 {
        return Err(OfficialPrimaryManifestationPnfExhaustiveOverlapError::ExistingRowConflict);
    }
    usize::try_from(value).map_err(|_| OfficialPrimaryManifestationPnfExhaustiveOverlapError::CountOverflow)
}

fn stable_ref(overlap_ref: &str, coverage: &SemanticPnfCoverage) -> String {
    let mut hasher = Sha256::new();
    let values = [
        EXHAUSTIVE_PNF_COMPARATOR_REVISION_REF.to_owned(),
        overlap_ref.to_owned(),
        coverage.projection_ref.clone(),
        coverage.pdf_parser_factor_count.to_string(),
        coverage.docx_parser_factor_count.to_string(),
        coverage.pdf_semantic_factor_count.to_string(),
        coverage.docx_semantic_factor_count.to_string(),
        coverage.matched_semantic_factor_count.to_string(),
        coverage.unmatched_pdf_semantic_count.to_string(),
        coverage.unmatched_docx_semantic_count.to_string(),
        coverage.ignored_pdf_nonsemantic_count.to_string(),
        coverage.ignored_docx_nonsemantic_count.to_string(),
        coverage.semantic_factor_multiset_exact.to_string(),
    ];
    for value in &values {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!(
        "official-primary-manifestation-pnf-exhaustive-overlap:sha256:{:x}",
        hasher.finalize()
    )
}

pub fn install_official_primary_manifestation_pnf_exhaustive_overlap_schema(
    config: &DatabaseConfig,
) -> Result<(), OfficialPrimaryManifestationPnfExhaustiveOverlapError> {
    install_official_primary_manifestation_pnf_overlap_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(OFFICIAL_PRIMARY_MANIFESTATION_PNF_EXHAUSTIVE_OVERLAP_SQL)?;
    Ok(())
}

/// Persist the exhaustive semantic-factor coverage for an already source-bound
/// PDF↔DOCX overlap.  The base v2 receipt remains immutable historical evidence;
/// this v3 receipt is a separate, stronger completeness claim.
pub fn compare_and_persist_official_primary_manifestation_pnf_exhaustive_overlap(
    config: &DatabaseConfig,
    overlap_ref: &str,
) -> Result<OfficialPrimaryManifestationPnfExhaustiveOverlapReceipt, OfficialPrimaryManifestationPnfExhaustiveOverlapError> {
    install_official_primary_manifestation_pnf_exhaustive_overlap_schema(config)?;
    let overlap = load_official_primary_manifestation_pnf_overlap(config, overlap_ref)?;
    let pdf = load_candidate_pnf_batch(config, &overlap.pdf_batch_ref)?
        .ok_or_else(|| OfficialPrimaryManifestationPnfExhaustiveOverlapError::MissingBatch(overlap.pdf_batch_ref.clone()))?;
    let docx = load_candidate_pnf_batch(config, &overlap.docx_batch_ref)?
        .ok_or_else(|| OfficialPrimaryManifestationPnfExhaustiveOverlapError::MissingBatch(overlap.docx_batch_ref.clone()))?;

    if pdf.batch_ref != overlap.pdf_batch_ref || docx.batch_ref != overlap.docx_batch_ref {
        return Err(OfficialPrimaryManifestationPnfExhaustiveOverlapError::CoordinateMismatch);
    }
    if pdf.proposition_support_paid
        || pdf.applicability_paid
        || pdf.claim_truth_paid
        || docx.proposition_support_paid
        || docx.applicability_paid
        || docx.claim_truth_paid
        || pdf.factors.iter().any(|factor| !factor.candidate_only)
        || docx.factors.iter().any(|factor| !factor.candidate_only)
    {
        return Err(OfficialPrimaryManifestationPnfExhaustiveOverlapError::Promotion);
    }

    let coverage = measure_semantic_pnf_coverage(&pdf.factors, &docx.factors);
    let exhaustive_overlap_ref = stable_ref(overlap_ref, &coverage);
    let expected = OfficialPrimaryManifestationPnfExhaustiveOverlapReceipt {
        exhaustive_overlap_ref: exhaustive_overlap_ref.clone(),
        overlap_ref: overlap_ref.into(),
        comparator_revision_ref: EXHAUSTIVE_PNF_COMPARATOR_REVISION_REF.into(),
        pdf_batch_ref: overlap.pdf_batch_ref,
        docx_batch_ref: overlap.docx_batch_ref,
        coverage,
    };

    let c = &expected.coverage;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO source_provenance.official_primary_manifestation_pnf_exhaustive_overlap
           (exhaustive_overlap_ref,overlap_ref,comparator_revision_ref,projection_ref,
            pdf_batch_ref,docx_batch_ref,pdf_parser_factor_count,docx_parser_factor_count,
            pdf_semantic_factor_count,docx_semantic_factor_count,matched_semantic_factor_count,
            unmatched_pdf_semantic_count,unmatched_docx_semantic_count,
            ignored_pdf_nonsemantic_count,ignored_docx_nonsemantic_count,
            semantic_factor_multiset_exact,closes_source_transformation_residual,
            requires_human_pnf_review,creates_legal_controversy,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted,
            independent_source_count_increment)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,
                  FALSE,FALSE,FALSE,FALSE,FALSE,0)
           ON CONFLICT(exhaustive_overlap_ref) DO NOTHING"#,
        &[
            &expected.exhaustive_overlap_ref,
            &expected.overlap_ref,
            &expected.comparator_revision_ref,
            &c.projection_ref,
            &expected.pdf_batch_ref,
            &expected.docx_batch_ref,
            &count_i64(c.pdf_parser_factor_count)?,
            &count_i64(c.docx_parser_factor_count)?,
            &count_i64(c.pdf_semantic_factor_count)?,
            &count_i64(c.docx_semantic_factor_count)?,
            &count_i64(c.matched_semantic_factor_count)?,
            &count_i64(c.unmatched_pdf_semantic_count)?,
            &count_i64(c.unmatched_docx_semantic_count)?,
            &count_i64(c.ignored_pdf_nonsemantic_count)?,
            &count_i64(c.ignored_docx_nonsemantic_count)?,
            &c.semantic_factor_multiset_exact,
            &c.closes_source_transformation_residual,
            &c.requires_human_pnf_review,
        ],
    )?;
    let reopened = load_official_primary_manifestation_pnf_exhaustive_overlap(
        config,
        &exhaustive_overlap_ref,
    )?;
    if reopened != expected {
        return Err(OfficialPrimaryManifestationPnfExhaustiveOverlapError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_official_primary_manifestation_pnf_exhaustive_overlap(
    config: &DatabaseConfig,
    exhaustive_overlap_ref: &str,
) -> Result<OfficialPrimaryManifestationPnfExhaustiveOverlapReceipt, OfficialPrimaryManifestationPnfExhaustiveOverlapError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT overlap_ref,comparator_revision_ref,projection_ref,pdf_batch_ref,docx_batch_ref,
                  pdf_parser_factor_count,docx_parser_factor_count,pdf_semantic_factor_count,
                  docx_semantic_factor_count,matched_semantic_factor_count,
                  unmatched_pdf_semantic_count,unmatched_docx_semantic_count,
                  ignored_pdf_nonsemantic_count,ignored_docx_nonsemantic_count,
                  semantic_factor_multiset_exact,closes_source_transformation_residual,
                  requires_human_pnf_review,creates_legal_controversy,creates_semantic_authority,
                  creates_legal_authority,applicability_promoted,claim_truth_promoted,
                  independent_source_count_increment
           FROM source_provenance.official_primary_manifestation_pnf_exhaustive_overlap
           WHERE exhaustive_overlap_ref=$1"#,
        &[&exhaustive_overlap_ref],
    )?.ok_or_else(|| OfficialPrimaryManifestationPnfExhaustiveOverlapError::NotFound(exhaustive_overlap_ref.into()))?;

    let coverage = SemanticPnfCoverage {
        projection_ref: row.get(2),
        pdf_parser_factor_count: count_usize(row.get(5))?,
        docx_parser_factor_count: count_usize(row.get(6))?,
        pdf_semantic_factor_count: count_usize(row.get(7))?,
        docx_semantic_factor_count: count_usize(row.get(8))?,
        matched_semantic_factor_count: count_usize(row.get(9))?,
        unmatched_pdf_semantic_count: count_usize(row.get(10))?,
        unmatched_docx_semantic_count: count_usize(row.get(11))?,
        ignored_pdf_nonsemantic_count: count_usize(row.get(12))?,
        ignored_docx_nonsemantic_count: count_usize(row.get(13))?,
        semantic_factor_multiset_exact: row.get(14),
        closes_source_transformation_residual: row.get(15),
        requires_human_pnf_review: row.get(16),
        creates_legal_controversy: row.get(17),
        creates_semantic_authority: row.get(18),
        creates_legal_authority: row.get(19),
        applicability_promoted: row.get(20),
        claim_truth_promoted: row.get(21),
        independent_source_count_increment: row.get(22),
    };
    if coverage.projection_ref != EXHAUSTIVE_PNF_PROJECTION_REF
        || coverage.closes_source_transformation_residual != coverage.semantic_factor_multiset_exact
        || coverage.requires_human_pnf_review == coverage.semantic_factor_multiset_exact
        || coverage.creates_legal_controversy
        || coverage.creates_semantic_authority
        || coverage.creates_legal_authority
        || coverage.applicability_promoted
        || coverage.claim_truth_promoted
        || coverage.independent_source_count_increment != 0
        || coverage.matched_semantic_factor_count + coverage.unmatched_pdf_semantic_count
            != coverage.pdf_semantic_factor_count
        || coverage.matched_semantic_factor_count + coverage.unmatched_docx_semantic_count
            != coverage.docx_semantic_factor_count
        || coverage.pdf_semantic_factor_count + coverage.ignored_pdf_nonsemantic_count
            != coverage.pdf_parser_factor_count
        || coverage.docx_semantic_factor_count + coverage.ignored_docx_nonsemantic_count
            != coverage.docx_parser_factor_count
    {
        return Err(OfficialPrimaryManifestationPnfExhaustiveOverlapError::ExistingRowConflict);
    }

    Ok(OfficialPrimaryManifestationPnfExhaustiveOverlapReceipt {
        exhaustive_overlap_ref: exhaustive_overlap_ref.into(),
        overlap_ref: row.get(0),
        comparator_revision_ref: row.get(1),
        pdf_batch_ref: row.get(3),
        docx_batch_ref: row.get(4),
        coverage,
    })
}
