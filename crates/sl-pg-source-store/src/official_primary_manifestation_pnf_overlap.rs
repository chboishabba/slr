//! Predicate-PNF overlap for two official manifestations of one HCA judgment.
//!
//! This is a thin source-provenance adapter over the existing generic
//! candidate-PNF / relational-interlingua owner.  It does not invent a second
//! semantic comparator.  PDF and DOCX batches must already exist in the normal
//! statement/PNF store and must reopen to the exact legal-source revisions
//! whose acquisition receipts are the persisted manifestation extraction refs.
//!
//! An exact candidate-shape (or explicitly licensed same-transformation
//! alignment) may close only the source-transformation residual.  It never
//! creates a second source witness, a legal controversy, semantic/legal
//! authority, applicability, or claim truth.

use std::collections::{BTreeMap, BTreeSet};

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    compare_relational_observations, install_candidate_pnf_schema,
    install_official_primary_manifestation_comparison_schema, load_candidate_pnf_batch,
    load_official_primary_manifestation_comparison, load_provider_legal_source_registration,
    observation_from_candidate_pnf, AlignmentDirection, CandidatePnfBatch, CandidatePnfFactor,
    CandidatePnfRole, CandidatePnfStoreError, ComparisonFinding, DatabaseConfig,
    LicensedAlignment, ManifestationComparisonDisposition, ObservationContext,
    OfficialPrimaryManifestationComparisonError, RelationalComparisonError,
    RelationalConsumer, RelationalSourceFamily,
};

pub const OFFICIAL_PRIMARY_MANIFESTATION_PNF_OVERLAP_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;

CREATE TABLE IF NOT EXISTS source_provenance.official_primary_manifestation_pnf_overlap (
  overlap_ref TEXT PRIMARY KEY,
  manifestation_comparison_ref TEXT NOT NULL
    REFERENCES source_provenance.official_primary_manifestation_comparison(comparison_ref),
  pdf_batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref),
  docx_batch_ref TEXT NOT NULL REFERENCES pnf.statement_candidate_batch(batch_ref),
  pdf_predicate_candidate_ref TEXT NOT NULL,
  docx_predicate_candidate_ref TEXT NOT NULL,
  relational_comparison_ref TEXT NOT NULL,
  finding_ref TEXT NOT NULL,
  residual_obligation_refs TEXT[] NOT NULL,
  alignment_witness_refs TEXT[] NOT NULL,
  disposition_ref TEXT NOT NULL CHECK (
    disposition_ref IN ('exact','partial','no_typed_meet')
  ),
  requires_human_pnf_review BOOLEAN NOT NULL,
  closes_source_transformation_residual BOOLEAN NOT NULL,
  creates_legal_controversy BOOLEAN NOT NULL CHECK (NOT creates_legal_controversy),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  independent_source_count_increment INTEGER NOT NULL DEFAULT 0
    CHECK (independent_source_count_increment = 0),
  UNIQUE(manifestation_comparison_ref,pdf_batch_ref,docx_batch_ref,
         pdf_predicate_candidate_ref,docx_predicate_candidate_ref)
);
"#;

const TRANSFORMATION_CONSUMER_REF: &str =
    "consumer:official-primary-manifestation-predicate-pnf-overlap:v1";
const TRANSFORMATION_LICENCE_REF: &str =
    "licence:same-official-judgment-transformation-pnf:v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestationPnfOverlapDisposition {
    Exact,
    Partial,
    NoTypedMeet,
}

impl ManifestationPnfOverlapDisposition {
    fn db(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Partial => "partial",
            Self::NoTypedMeet => "no_typed_meet",
        }
    }

    fn from_db(value: &str) -> Result<Self, OfficialPrimaryManifestationPnfOverlapError> {
        match value {
            "exact" => Ok(Self::Exact),
            "partial" => Ok(Self::Partial),
            "no_typed_meet" => Ok(Self::NoTypedMeet),
            _ => Err(OfficialPrimaryManifestationPnfOverlapError::ExistingRowConflict),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManifestationPnfOverlapClassification {
    pub disposition: ManifestationPnfOverlapDisposition,
    pub requires_human_pnf_review: bool,
    pub closes_source_transformation_residual: bool,
    pub creates_legal_controversy: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub independent_source_count_increment: i32,
}

#[must_use]
pub fn classify_manifestation_pnf_overlap(
    finding: ComparisonFinding,
) -> ManifestationPnfOverlapClassification {
    let disposition = match finding {
        ComparisonFinding::ExactCandidateShape | ComparisonFinding::LicensedCompatible => {
            ManifestationPnfOverlapDisposition::Exact
        }
        ComparisonFinding::CompatibleQualification
        | ComparisonFinding::PartialResidual
        | ComparisonFinding::PolarityConflictCandidate => {
            ManifestationPnfOverlapDisposition::Partial
        }
        ComparisonFinding::MissingTypedMeet | ComparisonFinding::Undetermined => {
            ManifestationPnfOverlapDisposition::NoTypedMeet
        }
    };
    let closes = disposition == ManifestationPnfOverlapDisposition::Exact;
    ManifestationPnfOverlapClassification {
        disposition,
        requires_human_pnf_review: !closes,
        closes_source_transformation_residual: closes,
        creates_legal_controversy: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        independent_source_count_increment: 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialPrimaryManifestationPnfOverlapReceipt {
    pub overlap_ref: String,
    pub manifestation_comparison_ref: String,
    pub pdf_batch_ref: String,
    pub docx_batch_ref: String,
    pub pdf_predicate_candidate_ref: String,
    pub docx_predicate_candidate_ref: String,
    pub relational_comparison_ref: String,
    pub finding: ComparisonFinding,
    pub residual_obligation_refs: Vec<String>,
    pub alignment_witness_refs: Vec<String>,
    pub classification: ManifestationPnfOverlapClassification,
}

#[derive(Debug, Error)]
pub enum OfficialPrimaryManifestationPnfOverlapError {
    #[error("manifestation comparison error: {0}")]
    Manifestation(#[from] OfficialPrimaryManifestationComparisonError),
    #[error("candidate PNF error: {0}")]
    Candidate(#[from] CandidatePnfStoreError),
    #[error("relational PNF comparison error: {0}")]
    Relational(#[from] RelationalComparisonError),
    #[error("provider legal-source registration error: {0}")]
    LegalSource(#[from] crate::ProviderLegalSourceRegistrationError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("candidate PNF batch does not exist: {0}")]
    MissingBatch(String),
    #[error("candidate PNF batch is not bound to the expected official manifestation")]
    SourceBindingMismatch,
    #[error("selected predicate candidate is absent or is not a candidate predicate")]
    InvalidPredicate,
    #[error("manifestation comparison must remain a material-difference PNF-review residual before PNF payment")]
    NoPnfResidualToPay,
    #[error("persisted manifestation PNF overlap conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("manifestation PNF overlap not found: {0}")]
    NotFound(String),
}

fn candidate_batch(value: &crate::PersistedCandidatePnfBatch) -> CandidatePnfBatch {
    CandidatePnfBatch {
        exact_span_ref: value.exact_span_ref.clone(),
        candidates: value.factors.clone(),
        proposition_support_paid: value.proposition_support_paid,
        applicability_paid: value.applicability_paid,
        claim_truth_paid: value.claim_truth_paid,
    }
}

fn role_name(role: CandidatePnfRole) -> &'static str {
    match role {
        CandidatePnfRole::Actor => "agent",
        CandidatePnfRole::Predicate => "predicate",
        CandidatePnfRole::Patient => "patient",
        CandidatePnfRole::Qualifier => "qualifier",
        CandidatePnfRole::Other => "other",
    }
}

fn semantic_signature(factor: &CandidatePnfFactor) -> (&str, &str, &'static str) {
    (
        factor.lemma.as_str(),
        factor.dependency_ref.as_str(),
        role_name(factor.role),
    )
}

fn role_occurrences(factors: &[CandidatePnfFactor]) -> BTreeMap<(&'static str, u32), &CandidatePnfFactor> {
    let mut counts = BTreeMap::<&'static str, u32>::new();
    let mut out = BTreeMap::new();
    for factor in factors {
        if factor.role == CandidatePnfRole::Predicate {
            continue;
        }
        let role = role_name(factor.role);
        let occurrence = counts.entry(role).or_insert(0);
        out.insert((role, *occurrence), factor);
        *occurrence += 1;
    }
    out
}

fn transformation_alignment(
    left_ref: &str,
    right_ref: &str,
    witness_ref: String,
) -> LicensedAlignment {
    LicensedAlignment {
        left_ref: left_ref.to_owned(),
        right_ref: right_ref.to_owned(),
        witness_ref,
        consumer_ref: TRANSFORMATION_CONSUMER_REF.into(),
        licence_ref: TRANSFORMATION_LICENCE_REF.into(),
        direction: AlignmentDirection::Symmetric,
    }
}

fn alignment_witness(kind: &str, left: &CandidatePnfFactor, right: &CandidatePnfFactor) -> String {
    let mut hasher = Sha256::new();
    for value in [
        "official-primary-transformation-pnf-alignment:v1",
        kind,
        left.candidate_ref.as_str(),
        right.candidate_ref.as_str(),
        left.lemma.as_str(),
        left.dependency_ref.as_str(),
        role_name(left.role),
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("transformation-pnf-alignment:sha256:{:x}", hasher.finalize())
}

fn transformation_consumer(
    pdf: &CandidatePnfBatch,
    docx: &CandidatePnfBatch,
    pdf_predicate_ref: &str,
    docx_predicate_ref: &str,
) -> Result<RelationalConsumer, OfficialPrimaryManifestationPnfOverlapError> {
    let pdf_predicate = pdf
        .candidates
        .iter()
        .find(|factor| factor.candidate_ref == pdf_predicate_ref && factor.role == CandidatePnfRole::Predicate && factor.candidate_only)
        .ok_or(OfficialPrimaryManifestationPnfOverlapError::InvalidPredicate)?;
    let docx_predicate = docx
        .candidates
        .iter()
        .find(|factor| factor.candidate_ref == docx_predicate_ref && factor.role == CandidatePnfRole::Predicate && factor.candidate_only)
        .ok_or(OfficialPrimaryManifestationPnfOverlapError::InvalidPredicate)?;

    let mut predicate_alignments = Vec::new();
    if semantic_signature(pdf_predicate) == semantic_signature(docx_predicate)
        && pdf_predicate.candidate_ref != docx_predicate.candidate_ref
    {
        predicate_alignments.push(transformation_alignment(
            &pdf_predicate.candidate_ref,
            &docx_predicate.candidate_ref,
            alignment_witness("predicate", pdf_predicate, docx_predicate),
        ));
    }

    let left = role_occurrences(&pdf.candidates);
    let right = role_occurrences(&docx.candidates);
    let mut filler_alignments = Vec::new();
    for (coordinate, left_factor) in left {
        let Some(right_factor) = right.get(&coordinate) else {
            continue;
        };
        if semantic_signature(left_factor) == semantic_signature(right_factor)
            && left_factor.candidate_ref != right_factor.candidate_ref
        {
            filler_alignments.push(transformation_alignment(
                &left_factor.candidate_ref,
                &right_factor.candidate_ref,
                alignment_witness("role-filler", left_factor, right_factor),
            ));
        }
    }

    Ok(RelationalConsumer {
        consumer_ref: TRANSFORMATION_CONSUMER_REF.into(),
        permitted_source_families: vec![RelationalSourceFamily::Legal],
        required_roles: vec![],
        required_role_types: vec![],
        compare_scope: false,
        compare_time: false,
        compare_modality: false,
        compare_quantifier: false,
        compare_attribution: false,
        compare_ontology: false,
        compare_language: false,
        predicate_alignments,
        role_alignments: vec![],
        filler_alignments,
        type_alignments: vec![],
    })
}

fn statement_source_revision(
    config: &DatabaseConfig,
    statement_ref: &str,
) -> Result<String, OfficialPrimaryManifestationPnfOverlapError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        "SELECT source_revision_ref FROM corpus.source_statement WHERE statement_ref=$1",
        &[&statement_ref],
    )?;
    row.map(|row| row.get(0))
        .ok_or(OfficialPrimaryManifestationPnfOverlapError::SourceBindingMismatch)
}

fn require_batch_bound_to_extraction(
    config: &DatabaseConfig,
    batch: &crate::PersistedCandidatePnfBatch,
    extraction_ref: &str,
) -> Result<(), OfficialPrimaryManifestationPnfOverlapError> {
    let source_revision_ref = statement_source_revision(config, &batch.statement_ref)?;
    let legal = load_provider_legal_source_registration(config, &source_revision_ref)?;
    if legal.provider_acquisition_receipt_ref != extraction_ref
        || !legal.compile_eligible
        || legal.creates_semantic_authority
        || legal.creates_legal_authority
        || legal.applicability_promoted
        || legal.claim_truth_promoted
    {
        return Err(OfficialPrimaryManifestationPnfOverlapError::SourceBindingMismatch);
    }
    Ok(())
}

fn finding_db(value: &ComparisonFinding) -> &'static str {
    match value {
        ComparisonFinding::ExactCandidateShape => "exact_candidate_shape",
        ComparisonFinding::LicensedCompatible => "licensed_compatible",
        ComparisonFinding::CompatibleQualification => "compatible_qualification",
        ComparisonFinding::PolarityConflictCandidate => "polarity_conflict_candidate",
        ComparisonFinding::PartialResidual => "partial_residual",
        ComparisonFinding::MissingTypedMeet => "missing_typed_meet",
        ComparisonFinding::Undetermined => "undetermined",
    }
}

fn finding_from_db(value: &str) -> Result<ComparisonFinding, OfficialPrimaryManifestationPnfOverlapError> {
    match value {
        "exact_candidate_shape" => Ok(ComparisonFinding::ExactCandidateShape),
        "licensed_compatible" => Ok(ComparisonFinding::LicensedCompatible),
        "compatible_qualification" => Ok(ComparisonFinding::CompatibleQualification),
        "polarity_conflict_candidate" => Ok(ComparisonFinding::PolarityConflictCandidate),
        "partial_residual" => Ok(ComparisonFinding::PartialResidual),
        "missing_typed_meet" => Ok(ComparisonFinding::MissingTypedMeet),
        "undetermined" => Ok(ComparisonFinding::Undetermined),
        _ => Err(OfficialPrimaryManifestationPnfOverlapError::ExistingRowConflict),
    }
}

fn overlap_ref(
    manifestation_comparison_ref: &str,
    pdf_batch_ref: &str,
    docx_batch_ref: &str,
    pdf_predicate_ref: &str,
    docx_predicate_ref: &str,
    relational_comparison_ref: &str,
) -> String {
    let mut hasher = Sha256::new();
    for value in [
        "official-primary-manifestation-pnf-overlap:v1",
        manifestation_comparison_ref,
        pdf_batch_ref,
        docx_batch_ref,
        pdf_predicate_ref,
        docx_predicate_ref,
        relational_comparison_ref,
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("official-primary-manifestation-pnf-overlap:sha256:{:x}", hasher.finalize())
}

pub fn install_official_primary_manifestation_pnf_overlap_schema(
    config: &DatabaseConfig,
) -> Result<(), OfficialPrimaryManifestationPnfOverlapError> {
    install_candidate_pnf_schema(config)?;
    install_official_primary_manifestation_comparison_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(OFFICIAL_PRIMARY_MANIFESTATION_PNF_OVERLAP_SCHEMA_SQL)?;
    Ok(())
}

pub fn compare_and_persist_official_primary_manifestation_pnf_overlap(
    config: &DatabaseConfig,
    manifestation_comparison_ref: &str,
    pdf_batch_ref: &str,
    docx_batch_ref: &str,
    pdf_predicate_candidate_ref: &str,
    docx_predicate_candidate_ref: &str,
) -> Result<OfficialPrimaryManifestationPnfOverlapReceipt, OfficialPrimaryManifestationPnfOverlapError> {
    install_official_primary_manifestation_pnf_overlap_schema(config)?;
    let manifestation = load_official_primary_manifestation_comparison(
        config,
        manifestation_comparison_ref,
    )?;
    if manifestation.disposition != ManifestationComparisonDisposition::RequiresPnfReview
        || !manifestation.requires_pnf_review
    {
        return Err(OfficialPrimaryManifestationPnfOverlapError::NoPnfResidualToPay);
    }

    let pdf_persisted = load_candidate_pnf_batch(config, pdf_batch_ref)?
        .ok_or_else(|| OfficialPrimaryManifestationPnfOverlapError::MissingBatch(pdf_batch_ref.into()))?;
    let docx_persisted = load_candidate_pnf_batch(config, docx_batch_ref)?
        .ok_or_else(|| OfficialPrimaryManifestationPnfOverlapError::MissingBatch(docx_batch_ref.into()))?;
    require_batch_bound_to_extraction(config, &pdf_persisted, &manifestation.pdf_extraction_ref)?;
    require_batch_bound_to_extraction(config, &docx_persisted, &manifestation.docx_extraction_ref)?;

    let pdf = candidate_batch(&pdf_persisted);
    let docx = candidate_batch(&docx_persisted);
    let consumer = transformation_consumer(
        &pdf,
        &docx,
        pdf_predicate_candidate_ref,
        docx_predicate_candidate_ref,
    )?;
    let pdf_observation = observation_from_candidate_pnf(
        &statement_source_revision(config, &pdf_persisted.statement_ref)?,
        RelationalSourceFamily::Legal,
        &format!("manifestation-pnf-observation:{pdf_batch_ref}"),
        &pdf_persisted.parser_receipt_ref,
        pdf_predicate_candidate_ref,
        &pdf,
        ObservationContext::default(),
        vec![manifestation_comparison_ref.into(), manifestation.pdf_extraction_ref.clone()],
    )?;
    let docx_observation = observation_from_candidate_pnf(
        &statement_source_revision(config, &docx_persisted.statement_ref)?,
        RelationalSourceFamily::Legal,
        &format!("manifestation-pnf-observation:{docx_batch_ref}"),
        &docx_persisted.parser_receipt_ref,
        docx_predicate_candidate_ref,
        &docx,
        ObservationContext::default(),
        vec![manifestation_comparison_ref.into(), manifestation.docx_extraction_ref.clone()],
    )?;
    let comparison = compare_relational_observations(&pdf_observation, &docx_observation, &consumer)?;
    let classification = classify_manifestation_pnf_overlap(comparison.finding.clone());
    let mut residual_obligation_refs = comparison
        .residuals
        .iter()
        .map(|residual| residual.obligation_ref.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    residual_obligation_refs.sort();
    let mut alignment_witness_refs = comparison.used_alignment_witness_refs.clone();
    alignment_witness_refs.sort();
    alignment_witness_refs.dedup();
    let reference = overlap_ref(
        manifestation_comparison_ref,
        pdf_batch_ref,
        docx_batch_ref,
        pdf_predicate_candidate_ref,
        docx_predicate_candidate_ref,
        &comparison.comparison_ref,
    );
    let expected = OfficialPrimaryManifestationPnfOverlapReceipt {
        overlap_ref: reference.clone(),
        manifestation_comparison_ref: manifestation_comparison_ref.into(),
        pdf_batch_ref: pdf_batch_ref.into(),
        docx_batch_ref: docx_batch_ref.into(),
        pdf_predicate_candidate_ref: pdf_predicate_candidate_ref.into(),
        docx_predicate_candidate_ref: docx_predicate_candidate_ref.into(),
        relational_comparison_ref: comparison.comparison_ref,
        finding: comparison.finding,
        residual_obligation_refs,
        alignment_witness_refs,
        classification,
    };

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO source_provenance.official_primary_manifestation_pnf_overlap
           (overlap_ref,manifestation_comparison_ref,pdf_batch_ref,docx_batch_ref,
            pdf_predicate_candidate_ref,docx_predicate_candidate_ref,
            relational_comparison_ref,finding_ref,residual_obligation_refs,
            alignment_witness_refs,disposition_ref,requires_human_pnf_review,
            closes_source_transformation_residual,creates_legal_controversy,
            creates_semantic_authority,creates_legal_authority,applicability_promoted,
            claim_truth_promoted,independent_source_count_increment)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,
                  FALSE,FALSE,FALSE,FALSE,FALSE,0)
           ON CONFLICT(overlap_ref) DO NOTHING"#,
        &[
            &expected.overlap_ref,
            &expected.manifestation_comparison_ref,
            &expected.pdf_batch_ref,
            &expected.docx_batch_ref,
            &expected.pdf_predicate_candidate_ref,
            &expected.docx_predicate_candidate_ref,
            &expected.relational_comparison_ref,
            &finding_db(&expected.finding),
            &expected.residual_obligation_refs,
            &expected.alignment_witness_refs,
            &expected.classification.disposition.db(),
            &expected.classification.requires_human_pnf_review,
            &expected.classification.closes_source_transformation_residual,
        ],
    )?;
    let reopened = load_official_primary_manifestation_pnf_overlap(config, &reference)?;
    if reopened != expected {
        return Err(OfficialPrimaryManifestationPnfOverlapError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_official_primary_manifestation_pnf_overlap(
    config: &DatabaseConfig,
    overlap_ref: &str,
) -> Result<OfficialPrimaryManifestationPnfOverlapReceipt, OfficialPrimaryManifestationPnfOverlapError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT manifestation_comparison_ref,pdf_batch_ref,docx_batch_ref,
                  pdf_predicate_candidate_ref,docx_predicate_candidate_ref,
                  relational_comparison_ref,finding_ref,residual_obligation_refs,
                  alignment_witness_refs,disposition_ref,requires_human_pnf_review,
                  closes_source_transformation_residual,creates_legal_controversy,
                  creates_semantic_authority,creates_legal_authority,applicability_promoted,
                  claim_truth_promoted,independent_source_count_increment
           FROM source_provenance.official_primary_manifestation_pnf_overlap
           WHERE overlap_ref=$1"#,
        &[&overlap_ref],
    )?.ok_or_else(|| OfficialPrimaryManifestationPnfOverlapError::NotFound(overlap_ref.into()))?;
    let finding = finding_from_db(&row.get::<_, String>(6))?;
    let classification = ManifestationPnfOverlapClassification {
        disposition: ManifestationPnfOverlapDisposition::from_db(&row.get::<_, String>(9))?,
        requires_human_pnf_review: row.get(10),
        closes_source_transformation_residual: row.get(11),
        creates_legal_controversy: row.get(12),
        creates_semantic_authority: row.get(13),
        creates_legal_authority: row.get(14),
        applicability_promoted: row.get(15),
        claim_truth_promoted: row.get(16),
        independent_source_count_increment: row.get(17),
    };
    if classification != classify_manifestation_pnf_overlap(finding.clone()) {
        return Err(OfficialPrimaryManifestationPnfOverlapError::ExistingRowConflict);
    }
    Ok(OfficialPrimaryManifestationPnfOverlapReceipt {
        overlap_ref: overlap_ref.into(),
        manifestation_comparison_ref: row.get(0),
        pdf_batch_ref: row.get(1),
        docx_batch_ref: row.get(2),
        pdf_predicate_candidate_ref: row.get(3),
        docx_predicate_candidate_ref: row.get(4),
        relational_comparison_ref: row.get(5),
        finding,
        residual_obligation_refs: row.get(7),
        alignment_witness_refs: row.get(8),
        classification,
    })
}
