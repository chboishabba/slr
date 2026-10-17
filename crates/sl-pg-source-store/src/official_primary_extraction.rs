//! Durable provenance for one official-primary PDF -> canonical UTF-8 transformation.
//!
//! The official court PDF remains the source payload. The extractor is a pinned
//! transformation implementation only; using OALC creator code does not relabel
//! the resulting materialisation as an OALC corpus record.

use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    install_provider_materialization_schema, load_provider_materialization, DatabaseConfig,
    ProviderMaterializationError,
};

pub const OFFICIAL_PRIMARY_EXTRACTION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;

CREATE TABLE IF NOT EXISTS source_provenance.official_primary_extraction (
  extraction_ref TEXT PRIMARY KEY,
  materialization_ref TEXT NOT NULL
    REFERENCES source_provenance.provider_materialization(materialization_ref),
  schema_ref TEXT NOT NULL,
  citation TEXT NOT NULL,
  source_provider_ref TEXT NOT NULL,
  source_url TEXT NOT NULL,
  source_media_type TEXT NOT NULL,
  source_sha256_hex TEXT NOT NULL,
  source_size_bytes BIGINT NOT NULL CHECK (source_size_bytes >= 0),
  extractor_repo TEXT NOT NULL,
  extractor_revision TEXT NOT NULL,
  extractor_path TEXT NOT NULL,
  extractor_function TEXT NOT NULL,
  extractor_license TEXT NOT NULL,
  parameters_json TEXT NOT NULL,
  output_sha256_hex TEXT NOT NULL,
  output_size_bytes BIGINT NOT NULL CHECK (output_size_bytes >= 0),
  authority_ref TEXT NOT NULL,
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE(materialization_ref)
);
"#;

const OALC_CREATOR_REPO: &str =
    "https://github.com/isaacus-dev/open-australian-legal-corpus-creator";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialPrimaryExtractionReceipt {
    pub schema: String,
    pub citation: String,
    pub source_provider_ref: String,
    pub source_url: String,
    pub source_media_type: String,
    pub source_sha256_hex: String,
    pub source_size_bytes: u64,
    pub extractor_repo: String,
    pub extractor_revision: String,
    pub extractor_path: String,
    pub extractor_function: String,
    pub extractor_license: String,
    pub parameters: serde_json::Value,
    pub output_sha256_hex: String,
    pub output_size_bytes: u64,
    pub authority: String,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedOfficialPrimaryExtraction {
    pub extraction_ref: String,
    pub materialization_ref: String,
    pub receipt: OfficialPrimaryExtractionReceipt,
}

#[derive(Debug, Error)]
pub enum OfficialPrimaryExtractionError {
    #[error("provider materialisation error: {0}")]
    Provider(#[from] ProviderMaterializationError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("serde JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("empty extraction coordinate: {0}")]
    EmptyCoordinate(&'static str),
    #[error("invalid SHA-256 coordinate: {0}")]
    InvalidSha256(&'static str),
    #[error("extractor revision is not an immutable git commit")]
    InvalidExtractorRevision,
    #[error("official-primary extraction authority boundary violated")]
    AuthorityEscalation,
    #[error("receipt does not describe an official HCA PDF")]
    WrongSourceProvider,
    #[error("receipt does not name the pinned OALC creator implementation")]
    WrongExtractor,
    #[error("receipt disagrees with provider materialisation: {0}")]
    MaterializationMismatch(&'static str),
    #[error("persisted extraction receipt conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("official-primary extraction receipt not found: {0}")]
    NotFound(String),
    #[error("receipt byte length exceeds PostgreSQL BIGINT")]
    ByteLengthOverflow,
}

fn required(name: &'static str, value: &str) -> Result<(), OfficialPrimaryExtractionError> {
    if value.trim().is_empty() {
        Err(OfficialPrimaryExtractionError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn valid_hex(value: &str, lengths: &[usize]) -> bool {
    lengths.contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn validate_official_primary_extraction_receipt(
    receipt: &OfficialPrimaryExtractionReceipt,
) -> Result<(), OfficialPrimaryExtractionError> {
    for (name, value) in [
        ("schema", receipt.schema.as_str()),
        ("citation", receipt.citation.as_str()),
        ("source_provider_ref", receipt.source_provider_ref.as_str()),
        ("source_url", receipt.source_url.as_str()),
        ("source_media_type", receipt.source_media_type.as_str()),
        ("extractor_repo", receipt.extractor_repo.as_str()),
        ("extractor_revision", receipt.extractor_revision.as_str()),
        ("extractor_path", receipt.extractor_path.as_str()),
        ("extractor_function", receipt.extractor_function.as_str()),
        ("extractor_license", receipt.extractor_license.as_str()),
        ("authority", receipt.authority.as_str()),
    ] {
        required(name, value)?;
    }
    let official_hca_url = receipt.source_url.starts_with("https://www.hcourt.gov.au/")
        || receipt
            .source_url
            .starts_with("https://eresources.hcourt.gov.au/");
    if receipt.schema != "sensiblaw-official-primary-extraction/v1"
        || receipt.source_provider_ref != "provider:official-hca-pdf"
        || receipt.source_media_type != "application/pdf"
        || !official_hca_url
    {
        return Err(OfficialPrimaryExtractionError::WrongSourceProvider);
    }
    if receipt.extractor_repo != OALC_CREATOR_REPO
        || receipt.extractor_license != "MIT"
        || receipt.extractor_function != "pdf2txt"
    {
        return Err(OfficialPrimaryExtractionError::WrongExtractor);
    }
    if !valid_hex(&receipt.source_sha256_hex, &[64]) {
        return Err(OfficialPrimaryExtractionError::InvalidSha256("source_sha256_hex"));
    }
    if !valid_hex(&receipt.output_sha256_hex, &[64]) {
        return Err(OfficialPrimaryExtractionError::InvalidSha256("output_sha256_hex"));
    }
    if !valid_hex(&receipt.extractor_revision, &[40, 64]) {
        return Err(OfficialPrimaryExtractionError::InvalidExtractorRevision);
    }
    if receipt.authority != "source-observation-only"
        || receipt.creates_semantic_authority
        || receipt.creates_legal_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(OfficialPrimaryExtractionError::AuthorityEscalation);
    }
    Ok(())
}

pub fn canonical_official_primary_extraction_ref(
    receipt: &OfficialPrimaryExtractionReceipt,
) -> Result<String, OfficialPrimaryExtractionError> {
    validate_official_primary_extraction_receipt(receipt)?;
    let parameters_json = serde_json::to_string(&receipt.parameters)?;
    let mut hasher = Sha256::new();
    for value in [
        receipt.source_provider_ref.as_str(),
        receipt.source_url.as_str(),
        receipt.source_sha256_hex.as_str(),
        receipt.extractor_repo.as_str(),
        receipt.extractor_revision.as_str(),
        receipt.extractor_path.as_str(),
        receipt.extractor_function.as_str(),
        parameters_json.as_str(),
        receipt.output_sha256_hex.as_str(),
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    Ok(format!("official-primary-extraction:sha256:{:x}", hasher.finalize()))
}

pub fn install_official_primary_extraction_schema(
    config: &DatabaseConfig,
) -> Result<(), OfficialPrimaryExtractionError> {
    install_provider_materialization_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(OFFICIAL_PRIMARY_EXTRACTION_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_official_primary_extraction(
    config: &DatabaseConfig,
    materialization_ref: &str,
    receipt: &OfficialPrimaryExtractionReceipt,
) -> Result<PersistedOfficialPrimaryExtraction, OfficialPrimaryExtractionError> {
    validate_official_primary_extraction_receipt(receipt)?;
    install_official_primary_extraction_schema(config)?;
    let materialization = load_provider_materialization(config, materialization_ref)?;

    if materialization.identity.provider_ref != receipt.source_provider_ref {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch("provider_ref"));
    }
    if materialization.identity.citation != receipt.citation {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch("citation"));
    }
    if materialization.identity.source_url.as_deref() != Some(receipt.source_url.as_str()) {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch("source_url"));
    }
    if materialization.identity.dataset_revision_ref
        != format!("sha256:{}", receipt.source_sha256_hex)
    {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch(
            "source_payload_revision",
        ));
    }
    if materialization.identity.acquisition_receipt_ref
        != canonical_official_primary_extraction_ref(receipt)?
    {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch(
            "acquisition_receipt_ref",
        ));
    }
    if materialization.canonical_sha256_hex != receipt.output_sha256_hex {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch("output_digest"));
    }
    let output_size = i64::try_from(receipt.output_size_bytes)
        .map_err(|_| OfficialPrimaryExtractionError::ByteLengthOverflow)?;
    if materialization.canonical_byte_length != output_size {
        return Err(OfficialPrimaryExtractionError::MaterializationMismatch("output_length"));
    }

    let extraction_ref = canonical_official_primary_extraction_ref(receipt)?;
    let source_size = i64::try_from(receipt.source_size_bytes)
        .map_err(|_| OfficialPrimaryExtractionError::ByteLengthOverflow)?;
    let parameters_json = serde_json::to_string(&receipt.parameters)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO source_provenance.official_primary_extraction
           (extraction_ref,materialization_ref,schema_ref,citation,source_provider_ref,
            source_url,source_media_type,source_sha256_hex,source_size_bytes,
            extractor_repo,extractor_revision,extractor_path,extractor_function,
            extractor_license,parameters_json,output_sha256_hex,output_size_bytes,
            authority_ref,creates_semantic_authority,creates_legal_authority,
            applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,
                  FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(extraction_ref) DO NOTHING"#,
        &[
            &extraction_ref,
            &materialization_ref,
            &receipt.schema,
            &receipt.citation,
            &receipt.source_provider_ref,
            &receipt.source_url,
            &receipt.source_media_type,
            &receipt.source_sha256_hex,
            &source_size,
            &receipt.extractor_repo,
            &receipt.extractor_revision,
            &receipt.extractor_path,
            &receipt.extractor_function,
            &receipt.extractor_license,
            &parameters_json,
            &receipt.output_sha256_hex,
            &output_size,
            &receipt.authority,
        ],
    )?;

    let reopened = load_official_primary_extraction(config, &extraction_ref)?;
    let expected = PersistedOfficialPrimaryExtraction {
        extraction_ref,
        materialization_ref: materialization_ref.to_owned(),
        receipt: receipt.clone(),
    };
    if reopened != expected {
        return Err(OfficialPrimaryExtractionError::ExistingRowConflict);
    }
    Ok(reopened)
}

pub fn load_official_primary_extraction(
    config: &DatabaseConfig,
    extraction_ref: &str,
) -> Result<PersistedOfficialPrimaryExtraction, OfficialPrimaryExtractionError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT materialization_ref,schema_ref,citation,source_provider_ref,source_url,
                      source_media_type,source_sha256_hex,source_size_bytes,extractor_repo,
                      extractor_revision,extractor_path,extractor_function,extractor_license,
                      parameters_json,output_sha256_hex,output_size_bytes,authority_ref,
                      creates_semantic_authority,creates_legal_authority,applicability_promoted,
                      claim_truth_promoted
               FROM source_provenance.official_primary_extraction
               WHERE extraction_ref=$1"#,
            &[&extraction_ref],
        )?
        .ok_or_else(|| OfficialPrimaryExtractionError::NotFound(extraction_ref.to_owned()))?;

    let source_size: i64 = row.get(7);
    let output_size: i64 = row.get(15);
    let parameters_json: String = row.get(13);
    let receipt = OfficialPrimaryExtractionReceipt {
        schema: row.get(1),
        citation: row.get(2),
        source_provider_ref: row.get(3),
        source_url: row.get(4),
        source_media_type: row.get(5),
        source_sha256_hex: row.get(6),
        source_size_bytes: u64::try_from(source_size)
            .map_err(|_| OfficialPrimaryExtractionError::ByteLengthOverflow)?,
        extractor_repo: row.get(8),
        extractor_revision: row.get(9),
        extractor_path: row.get(10),
        extractor_function: row.get(11),
        extractor_license: row.get(12),
        parameters: serde_json::from_str(&parameters_json)?,
        output_sha256_hex: row.get(14),
        output_size_bytes: u64::try_from(output_size)
            .map_err(|_| OfficialPrimaryExtractionError::ByteLengthOverflow)?,
        authority: row.get(16),
        creates_semantic_authority: row.get(17),
        creates_legal_authority: row.get(18),
        applicability_promoted: row.get(19),
        claim_truth_promoted: row.get(20),
    };
    validate_official_primary_extraction_receipt(&receipt)?;
    Ok(PersistedOfficialPrimaryExtraction {
        extraction_ref: extraction_ref.to_owned(),
        materialization_ref: row.get(0),
        receipt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> OfficialPrimaryExtractionReceipt {
        OfficialPrimaryExtractionReceipt {
            schema: "sensiblaw-official-primary-extraction/v1".into(),
            citation: "Smith v Chief Executive, Queensland Corrective Services [2026] HCA 37".into(),
            source_provider_ref: "provider:official-hca-pdf".into(),
            source_url: "https://www.hcourt.gov.au/sites/default/files/eresources/2026-10-14/HCA/example.pdf".into(),
            source_media_type: "application/pdf".into(),
            source_sha256_hex: "a".repeat(64),
            source_size_bytes: 123,
            extractor_repo: OALC_CREATOR_REPO.into(),
            extractor_revision: "dbdb9e3752394a87a9c966c9a9199f9685db9af2".into(),
            extractor_path: "src/oalc_creator/ocr.py".into(),
            extractor_function: "pdf2txt".into(),
            extractor_license: "MIT".into(),
            parameters: serde_json::json!({"pdf_scale": 2}),
            output_sha256_hex: "b".repeat(64),
            output_size_bytes: 456,
            authority: "source-observation-only".into(),
            creates_semantic_authority: false,
            creates_legal_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn official_pdf_receipt_is_non_promoting_and_content_addressed() {
        let value = receipt();
        validate_official_primary_extraction_receipt(&value).unwrap();
        let reference = canonical_official_primary_extraction_ref(&value).unwrap();
        assert!(reference.starts_with("official-primary-extraction:sha256:"));
    }

    #[test]
    fn non_hca_url_cannot_be_relabeled_as_official_hca_pdf() {
        let mut value = receipt();
        value.source_url = "https://example.com/judgment.pdf".into();
        assert!(matches!(
            validate_official_primary_extraction_receipt(&value),
            Err(OfficialPrimaryExtractionError::WrongSourceProvider)
        ));
    }

    #[test]
    fn authority_promotion_fails_closed() {
        let mut value = receipt();
        value.claim_truth_promoted = true;
        assert!(matches!(
            validate_official_primary_extraction_receipt(&value),
            Err(OfficialPrimaryExtractionError::AuthorityEscalation)
        ));
    }
}
