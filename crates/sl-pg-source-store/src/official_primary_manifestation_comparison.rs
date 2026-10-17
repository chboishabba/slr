//! Compare two official manifestations of one HCA judgment as transformation evidence.
//!
//! Agreement never increments independent authority. Divergence is a source-
//! transformation residual requiring PNF/review, not a legal controversy.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    install_official_primary_docx_extraction_schema, install_official_primary_extraction_schema,
    load_official_primary_docx_extraction, load_official_primary_extraction, DatabaseConfig,
    OfficialPrimaryDocxExtractionError, OfficialPrimaryExtractionError,
};

pub const OFFICIAL_PRIMARY_MANIFESTATION_COMPARISON_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;

CREATE TABLE IF NOT EXISTS source_provenance.official_primary_manifestation_comparison (
  comparison_ref TEXT PRIMARY KEY,
  citation TEXT NOT NULL,
  pdf_extraction_ref TEXT NOT NULL
    REFERENCES source_provenance.official_primary_extraction(extraction_ref),
  docx_extraction_ref TEXT NOT NULL
    REFERENCES source_provenance.official_primary_docx_extraction(extraction_ref),
  pdf_output_sha256_hex TEXT NOT NULL,
  docx_output_sha256_hex TEXT NOT NULL,
  normalization_ref TEXT NOT NULL,
  pdf_normalized_sha256_hex TEXT NOT NULL,
  docx_normalized_sha256_hex TEXT NOT NULL,
  disposition TEXT NOT NULL CHECK (
    disposition IN ('exact_bytes','formatting_only','requires_pnf_review')
  ),
  requires_pnf_review BOOLEAN NOT NULL,
  creates_legal_controversy BOOLEAN NOT NULL CHECK (NOT creates_legal_controversy),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  independent_source_count_increment INTEGER NOT NULL DEFAULT 0
    CHECK (independent_source_count_increment = 0),
  UNIQUE(pdf_extraction_ref, docx_extraction_ref)
);
"#;

const NORMALIZATION_REF: &str = "unicode-whitespace-collapse:v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestationComparisonDisposition {
    ExactBytes,
    FormattingOnly,
    RequiresPnfReview,
}

impl ManifestationComparisonDisposition {
    fn db(self) -> &'static str {
        match self {
            Self::ExactBytes => "exact_bytes",
            Self::FormattingOnly => "formatting_only",
            Self::RequiresPnfReview => "requires_pnf_review",
        }
    }

    fn from_db(value: &str) -> Result<Self, OfficialPrimaryManifestationComparisonError> {
        match value {
            "exact_bytes" => Ok(Self::ExactBytes),
            "formatting_only" => Ok(Self::FormattingOnly),
            "requires_pnf_review" => Ok(Self::RequiresPnfReview),
            _ => Err(OfficialPrimaryManifestationComparisonError::ExistingRowConflict),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficialPrimaryManifestationComparisonReceipt {
    pub comparison_ref: String,
    pub citation: String,
    pub pdf_extraction_ref: String,
    pub docx_extraction_ref: String,
    pub pdf_output_sha256_hex: String,
    pub docx_output_sha256_hex: String,
    pub normalization_ref: String,
    pub pdf_normalized_sha256_hex: String,
    pub docx_normalized_sha256_hex: String,
    pub disposition: ManifestationComparisonDisposition,
    pub requires_pnf_review: bool,
    pub creates_legal_controversy: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub independent_source_count_increment: i32,
}

#[derive(Debug, Error)]
pub enum OfficialPrimaryManifestationComparisonError {
    #[error("PDF extraction error: {0}")]
    Pdf(#[from] OfficialPrimaryExtractionError),
    #[error("DOCX extraction error: {0}")]
    Docx(#[from] OfficialPrimaryDocxExtractionError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("PDF and DOCX manifestations do not identify the same judgment citation")]
    CitationMismatch,
    #[error("supplied extracted text does not match its persisted extraction digest: {0}")]
    TextDigestMismatch(&'static str),
    #[error("persisted manifestation comparison conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("manifestation comparison not found: {0}")]
    NotFound(String),
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn normalized_material_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn comparison_ref(
    pdf_ref: &str,
    docx_ref: &str,
    pdf_normalized_sha: &str,
    docx_normalized_sha: &str,
) -> String {
    let mut hasher = Sha256::new();
    for value in [pdf_ref, docx_ref, NORMALIZATION_REF, pdf_normalized_sha, docx_normalized_sha] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    format!("official-primary-manifestation-comparison:sha256:{:x}", hasher.finalize())
}

pub fn install_official_primary_manifestation_comparison_schema(
    config: &DatabaseConfig,
) -> Result<(), OfficialPrimaryManifestationComparisonError> {
    install_official_primary_extraction_schema(config)?;
    install_official_primary_docx_extraction_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(OFFICIAL_PRIMARY_MANIFESTATION_COMPARISON_SCHEMA_SQL)?;
    Ok(())
}

pub fn compare_and_persist_official_primary_manifestations(
    config: &DatabaseConfig,
    pdf_extraction_ref: &str,
    docx_extraction_ref: &str,
    pdf_text: &str,
    docx_text: &str,
) -> Result<OfficialPrimaryManifestationComparisonReceipt, OfficialPrimaryManifestationComparisonError> {
    install_official_primary_manifestation_comparison_schema(config)?;
    let pdf = load_official_primary_extraction(config, pdf_extraction_ref)?;
    let docx = load_official_primary_docx_extraction(config, docx_extraction_ref)?;
    if pdf.receipt.citation != docx.receipt.citation {
        return Err(OfficialPrimaryManifestationComparisonError::CitationMismatch);
    }
    if sha256_hex(pdf_text.as_bytes()) != pdf.receipt.output_sha256_hex {
        return Err(OfficialPrimaryManifestationComparisonError::TextDigestMismatch("pdf"));
    }
    if sha256_hex(docx_text.as_bytes()) != docx.receipt.output_sha256_hex {
        return Err(OfficialPrimaryManifestationComparisonError::TextDigestMismatch("docx"));
    }

    let pdf_normalized_sha = sha256_hex(normalized_material_text(pdf_text).as_bytes());
    let docx_normalized_sha = sha256_hex(normalized_material_text(docx_text).as_bytes());
    let disposition = if pdf.receipt.output_sha256_hex == docx.receipt.output_sha256_hex {
        ManifestationComparisonDisposition::ExactBytes
    } else if pdf_normalized_sha == docx_normalized_sha {
        ManifestationComparisonDisposition::FormattingOnly
    } else {
        ManifestationComparisonDisposition::RequiresPnfReview
    };
    let requires_pnf_review = disposition == ManifestationComparisonDisposition::RequiresPnfReview;
    let reference = comparison_ref(
        pdf_extraction_ref,
        docx_extraction_ref,
        &pdf_normalized_sha,
        &docx_normalized_sha,
    );

    let expected = OfficialPrimaryManifestationComparisonReceipt {
        comparison_ref: reference.clone(),
        citation: pdf.receipt.citation,
        pdf_extraction_ref: pdf_extraction_ref.to_owned(),
        docx_extraction_ref: docx_extraction_ref.to_owned(),
        pdf_output_sha256_hex: pdf.receipt.output_sha256_hex,
        docx_output_sha256_hex: docx.receipt.output_sha256_hex,
        normalization_ref: NORMALIZATION_REF.into(),
        pdf_normalized_sha256_hex: pdf_normalized_sha,
        docx_normalized_sha256_hex: docx_normalized_sha,
        disposition,
        requires_pnf_review,
        creates_legal_controversy: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        independent_source_count_increment: 0,
    };

    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO source_provenance.official_primary_manifestation_comparison
           (comparison_ref,citation,pdf_extraction_ref,docx_extraction_ref,
            pdf_output_sha256_hex,docx_output_sha256_hex,normalization_ref,
            pdf_normalized_sha256_hex,docx_normalized_sha256_hex,disposition,
            requires_pnf_review,creates_legal_controversy,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted,
            independent_source_count_increment)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,FALSE,FALSE,FALSE,FALSE,FALSE,0)
           ON CONFLICT(comparison_ref) DO NOTHING"#,
        &[
            &expected.comparison_ref,
            &expected.citation,
            &expected.pdf_extraction_ref,
            &expected.docx_extraction_ref,
            &expected.pdf_output_sha256_hex,
            &expected.docx_output_sha256_hex,
            &expected.normalization_ref,
            &expected.pdf_normalized_sha256_hex,
            &expected.docx_normalized_sha256_hex,
            &expected.disposition.db(),
            &expected.requires_pnf_review,
        ],
    )?;

    let reopened = load_official_primary_manifestation_comparison(config, &reference)?;
    if reopened != expected {
        return Err(OfficialPrimaryManifestationComparisonError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_official_primary_manifestation_comparison(
    config: &DatabaseConfig,
    comparison_ref: &str,
) -> Result<OfficialPrimaryManifestationComparisonReceipt, OfficialPrimaryManifestationComparisonError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT citation,pdf_extraction_ref,docx_extraction_ref,pdf_output_sha256_hex,
                      docx_output_sha256_hex,normalization_ref,pdf_normalized_sha256_hex,
                      docx_normalized_sha256_hex,disposition,requires_pnf_review,
                      creates_legal_controversy,creates_semantic_authority,creates_legal_authority,
                      applicability_promoted,claim_truth_promoted,independent_source_count_increment
               FROM source_provenance.official_primary_manifestation_comparison
               WHERE comparison_ref=$1"#,
            &[&comparison_ref],
        )?
        .ok_or_else(|| OfficialPrimaryManifestationComparisonError::NotFound(comparison_ref.to_owned()))?;

    let value = OfficialPrimaryManifestationComparisonReceipt {
        comparison_ref: comparison_ref.to_owned(),
        citation: row.get(0),
        pdf_extraction_ref: row.get(1),
        docx_extraction_ref: row.get(2),
        pdf_output_sha256_hex: row.get(3),
        docx_output_sha256_hex: row.get(4),
        normalization_ref: row.get(5),
        pdf_normalized_sha256_hex: row.get(6),
        docx_normalized_sha256_hex: row.get(7),
        disposition: ManifestationComparisonDisposition::from_db(&row.get::<_, String>(8))?,
        requires_pnf_review: row.get(9),
        creates_legal_controversy: row.get(10),
        creates_semantic_authority: row.get(11),
        creates_legal_authority: row.get(12),
        applicability_promoted: row.get(13),
        claim_truth_promoted: row.get(14),
        independent_source_count_increment: row.get(15),
    };
    if value.normalization_ref != NORMALIZATION_REF
        || value.requires_pnf_review != (value.disposition == ManifestationComparisonDisposition::RequiresPnfReview)
        || value.creates_legal_controversy
        || value.creates_semantic_authority
        || value.creates_legal_authority
        || value.applicability_promoted
        || value.claim_truth_promoted
        || value.independent_source_count_increment != 0
    {
        return Err(OfficialPrimaryManifestationComparisonError::ExistingRowConflict);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatting_normalization_does_not_assert_semantic_equivalence() {
        let a = normalized_material_text("A\n\nB   C");
        let b = normalized_material_text("A B C");
        assert_eq!(a, b);
    }

    #[test]
    fn material_difference_survives_whitespace_normalization() {
        assert_ne!(
            normalized_material_text("technology assists conception"),
            normalized_material_text("technology is connected with reproduction")
        );
    }
}
