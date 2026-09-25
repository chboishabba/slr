//! INGEST-1 generic content-bearing source persistence.
//!
//! Reuses the established corpus.canonical_content + corpus.document substrate.
//! This module adds only the missing provider-neutral source-revision binding
//! needed by long documents and other content source families.

use postgres::{Client, NoTls};
use sensiblaw_core::source_ingest::{IngestRoleClass, SourceFamily, SourceIngestEnvelope};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub const GENERIC_SOURCE_REVISION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS ingest;

CREATE TABLE IF NOT EXISTS ingest.generic_source_revision (
    source_revision_ref TEXT PRIMARY KEY,
    source_ref TEXT NOT NULL,
    document_ref TEXT NOT NULL,
    provider_ref TEXT NOT NULL,
    source_family_ref TEXT NOT NULL,
    ingest_role_class_ref TEXT NOT NULL,
    acquisition_receipt_ref TEXT NOT NULL,
    content_digest_ref TEXT NOT NULL,
    media_type_ref TEXT NOT NULL,
    candidate_only BOOLEAN NOT NULL,
    creates_semantic_authority BOOLEAN NOT NULL,
    applicability_promoted BOOLEAN NOT NULL,
    claim_truth_promoted BOOLEAN NOT NULL
);

CREATE INDEX IF NOT EXISTS generic_source_revision_source_idx
ON ingest.generic_source_revision(source_ref);
"#;

#[derive(Debug, Error)]
pub enum GenericSourceContentStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("source ingest envelope failed validation: {0:?}")]
    InvalidEnvelope(sensiblaw_core::source_ingest::SourceIngestError),
    #[error("canonical corpus tables are not installed")]
    CanonicalCorpusSchemaMissing,
    #[error("canonical text is empty")]
    EmptyCanonicalText,
    #[error("source envelope digest does not match canonical text")]
    ContentDigestMismatch,
    #[error("stored canonical payload is not valid UTF-8")]
    InvalidUtf8,
    #[error("stored source revision violated non-promotion boundary")]
    StoredPromotionBoundary,
    #[error("source revision ref already exists with different immutable identity")]
    SourceRevisionIdentityConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedGenericSourceContent {
    pub source_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub canonical_ref: String,
    pub provider_ref: String,
    pub source_family_ref: String,
    pub ingest_role_class_ref: String,
    pub acquisition_receipt_ref: String,
    pub content_digest_ref: String,
    pub media_type_ref: String,
    pub byte_length: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

fn sha256_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn family_db(family: SourceFamily) -> &'static str {
    match family {
        SourceFamily::Document => "document",
        SourceFamily::Mail => "mail",
        SourceFamily::Chat => "chat",
        SourceFamily::SocialMessage => "social_message",
        SourceFamily::Transcript => "transcript",
        SourceFamily::Audio => "audio",
        SourceFamily::ImageOcr => "image_ocr",
        SourceFamily::Web => "web",
        SourceFamily::Wiki => "wiki",
        SourceFamily::LegalAuthority => "legal_authority",
        SourceFamily::NoteResearch => "note_research",
        SourceFamily::FieldCapture => "field_capture",
        SourceFamily::Calendar => "calendar",
        SourceFamily::FinancialRecord => "financial_record",
        SourceFamily::StructuredDataset => "structured_dataset",
        SourceFamily::MachineArtifact => "machine_artifact",
    }
}

fn role_db(role: IngestRoleClass) -> &'static str {
    match role {
        IngestRoleClass::ContentSource => "content_source",
        IngestRoleClass::ObserverSource => "observer_source",
        IngestRoleClass::OperationalSource => "operational_source",
        IngestRoleClass::ExternalAuthoritySource => "external_authority_source",
        IngestRoleClass::ContextOverlay => "context_overlay",
    }
}

fn ensure_canonical_schema(client: &mut Client) -> Result<(), GenericSourceContentStoreError> {
    let row = client.query_one(
        "SELECT
           to_regclass('corpus.canonical_content')::text,
           to_regclass('corpus.document')::text",
        &[],
    )?;
    let canonical: Option<String> = row.get(0);
    let document: Option<String> = row.get(1);
    if canonical.is_none() || document.is_none() {
        return Err(GenericSourceContentStoreError::CanonicalCorpusSchemaMissing);
    }
    Ok(())
}

pub fn install_generic_source_revision_schema(
    config: &DatabaseConfig,
) -> Result<(), GenericSourceContentStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    ensure_canonical_schema(&mut client)?;
    client.batch_execute(GENERIC_SOURCE_REVISION_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_generic_text_source(
    config: &DatabaseConfig,
    envelope: &SourceIngestEnvelope,
    canonical_text: &str,
) -> Result<PersistedGenericSourceContent, GenericSourceContentStoreError> {
    envelope
        .validate()
        .map_err(GenericSourceContentStoreError::InvalidEnvelope)?;
    if canonical_text.is_empty() {
        return Err(GenericSourceContentStoreError::EmptyCanonicalText);
    }

    let digest = sha256_bytes(canonical_text.as_bytes());
    let digest_hex = hex(&digest);
    let digest_ref = format!("sha256:{digest_hex}");
    if envelope.content_digest_ref != digest_ref {
        return Err(GenericSourceContentStoreError::ContentDigestMismatch);
    }

    let canonical_ref = format!("canonical:sha256:{digest_hex}");
    let document_ref = format!("document:sha256:{digest_hex}");

    let mut client = Client::connect(config.database_url(), NoTls)?;
    ensure_canonical_schema(&mut client)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(GENERIC_SOURCE_REVISION_SCHEMA_SQL)?;

    tx.execute(
        "INSERT INTO corpus.canonical_content
         (canonical_ref, content_sha256, encoding_ref, normalization_ref,
          compression_ref, payload, uncompressed_byte_length)
         VALUES ($1,$2,'utf-8','identity',NULL,$3,$4)
         ON CONFLICT (content_sha256, encoding_ref, normalization_ref) DO NOTHING",
        &[
            &canonical_ref,
            &&digest[..],
            &canonical_text.as_bytes(),
            &(canonical_text.len() as i64),
        ],
    )?;

    let canonical_ref: String = tx
        .query_one(
            "SELECT canonical_ref
             FROM corpus.canonical_content
             WHERE content_sha256=$1
               AND encoding_ref='utf-8'
               AND normalization_ref='identity'",
            &[&&digest[..]],
        )?
        .get(0);

    tx.execute(
        "INSERT INTO corpus.document
         (document_ref, source_content_ref, canonical_ref, media_type,
          adapter_ref, adapter_version, compiler_context_ref, document_sha256)
         VALUES ($1,NULL,$2,$3,'generic-source-ingest','1','INGEST-1',$4)
         ON CONFLICT (document_sha256) DO NOTHING",
        &[
            &document_ref,
            &canonical_ref,
            &envelope.media_type_ref,
            &&digest[..],
        ],
    )?;

    let document_ref: String = tx
        .query_one(
            "SELECT document_ref FROM corpus.document WHERE document_sha256=$1",
            &[&&digest[..]],
        )?
        .get(0);

    tx.execute(
        "INSERT INTO ingest.generic_source_revision
         (source_revision_ref, source_ref, document_ref, provider_ref,
          source_family_ref, ingest_role_class_ref, acquisition_receipt_ref,
          content_digest_ref, media_type_ref, candidate_only,
          creates_semantic_authority, applicability_promoted, claim_truth_promoted)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,TRUE,FALSE,FALSE,FALSE)
         ON CONFLICT (source_revision_ref) DO NOTHING",
        &[
            &envelope.source_revision_ref,
            &envelope.source_ref,
            &document_ref,
            &envelope.provider_ref,
            &family_db(envelope.family),
            &role_db(envelope.role_class),
            &envelope.acquisition_receipt_ref,
            &envelope.content_digest_ref,
            &envelope.media_type_ref,
        ],
    )?;

    let identity = tx.query_one(
        "SELECT source_ref, document_ref, provider_ref, source_family_ref,
                ingest_role_class_ref, acquisition_receipt_ref,
                content_digest_ref, media_type_ref, candidate_only,
                creates_semantic_authority, applicability_promoted,
                claim_truth_promoted
         FROM ingest.generic_source_revision
         WHERE source_revision_ref = $1",
        &[&envelope.source_revision_ref],
    )?;
    let identity_matches =
        identity.get::<_, String>(0) == envelope.source_ref
        && identity.get::<_, String>(1) == document_ref
        && identity.get::<_, String>(2) == envelope.provider_ref
        && identity.get::<_, String>(3) == family_db(envelope.family)
        && identity.get::<_, String>(4) == role_db(envelope.role_class)
        && identity.get::<_, String>(5) == envelope.acquisition_receipt_ref
        && identity.get::<_, String>(6) == envelope.content_digest_ref
        && identity.get::<_, String>(7) == envelope.media_type_ref
        && identity.get::<_, bool>(8)
        && !identity.get::<_, bool>(9)
        && !identity.get::<_, bool>(10)
        && !identity.get::<_, bool>(11);
    if !identity_matches {
        return Err(GenericSourceContentStoreError::SourceRevisionIdentityConflict);
    }

    tx.commit()?;

    Ok(PersistedGenericSourceContent {
        source_ref: envelope.source_ref.clone(),
        source_revision_ref: envelope.source_revision_ref.clone(),
        document_ref,
        canonical_ref,
        provider_ref: envelope.provider_ref.clone(),
        source_family_ref: family_db(envelope.family).into(),
        ingest_role_class_ref: role_db(envelope.role_class).into(),
        acquisition_receipt_ref: envelope.acquisition_receipt_ref.clone(),
        content_digest_ref: digest_ref,
        media_type_ref: envelope.media_type_ref.clone(),
        byte_length: canonical_text.len(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn load_generic_text_source(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<(PersistedGenericSourceContent, String), GenericSourceContentStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    ensure_canonical_schema(&mut client)?;
    client.batch_execute(GENERIC_SOURCE_REVISION_SCHEMA_SQL)?;

    let row = client.query_one(
        "SELECT
           r.source_ref,
           r.document_ref,
           d.canonical_ref,
           r.provider_ref,
           r.source_family_ref,
           r.ingest_role_class_ref,
           r.acquisition_receipt_ref,
           r.content_digest_ref,
           r.media_type_ref,
           c.payload,
           r.candidate_only,
           r.creates_semantic_authority,
           r.applicability_promoted,
           r.claim_truth_promoted
         FROM ingest.generic_source_revision r
         JOIN corpus.document d ON d.document_ref = r.document_ref
         JOIN corpus.canonical_content c ON c.canonical_ref = d.canonical_ref
         WHERE r.source_revision_ref = $1",
        &[&source_revision_ref],
    )?;

    let payload: Vec<u8> = row.get(9);
    let text = String::from_utf8(payload)
        .map_err(|_| GenericSourceContentStoreError::InvalidUtf8)?;
    let receipt = PersistedGenericSourceContent {
        source_ref: row.get(0),
        source_revision_ref: source_revision_ref.to_owned(),
        document_ref: row.get(1),
        canonical_ref: row.get(2),
        provider_ref: row.get(3),
        source_family_ref: row.get(4),
        ingest_role_class_ref: row.get(5),
        acquisition_receipt_ref: row.get(6),
        content_digest_ref: row.get(7),
        media_type_ref: row.get(8),
        byte_length: text.len(),
        candidate_only: row.get(10),
        creates_semantic_authority: row.get(11),
        applicability_promoted: row.get(12),
        claim_truth_promoted: row.get(13),
    };

    if !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(GenericSourceContentStoreError::StoredPromotionBoundary);
    }

    let digest_ref = format!("sha256:{}", hex(&sha256_bytes(text.as_bytes())));
    if digest_ref != receipt.content_digest_ref {
        return Err(GenericSourceContentStoreError::ContentDigestMismatch);
    }

    Ok((receipt, text))
}


fn family_from_db(value: &str) -> Option<SourceFamily> {
    Some(match value {
        "document" => SourceFamily::Document,
        "mail" => SourceFamily::Mail,
        "chat" => SourceFamily::Chat,
        "social_message" => SourceFamily::SocialMessage,
        "transcript" => SourceFamily::Transcript,
        "audio" => SourceFamily::Audio,
        "image_ocr" => SourceFamily::ImageOcr,
        "web" => SourceFamily::Web,
        "wiki" => SourceFamily::Wiki,
        "legal_authority" => SourceFamily::LegalAuthority,
        "note_research" => SourceFamily::NoteResearch,
        "field_capture" => SourceFamily::FieldCapture,
        "calendar" => SourceFamily::Calendar,
        "financial_record" => SourceFamily::FinancialRecord,
        "structured_dataset" => SourceFamily::StructuredDataset,
        "machine_artifact" => SourceFamily::MachineArtifact,
        _ => return None,
    })
}

fn role_from_db(value: &str) -> Option<IngestRoleClass> {
    Some(match value {
        "content_source" => IngestRoleClass::ContentSource,
        "observer_source" => IngestRoleClass::ObserverSource,
        "operational_source" => IngestRoleClass::OperationalSource,
        "external_authority_source" => IngestRoleClass::ExternalAuthoritySource,
        "context_overlay" => IngestRoleClass::ContextOverlay,
        _ => return None,
    })
}

pub fn load_generic_source_envelope(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<SourceIngestEnvelope, GenericSourceContentStoreError> {
    let (receipt, _) = load_generic_text_source(config, source_revision_ref)?;
    let family = family_from_db(&receipt.source_family_ref)
        .ok_or(GenericSourceContentStoreError::StoredPromotionBoundary)?;
    let role_class = role_from_db(&receipt.ingest_role_class_ref)
        .ok_or(GenericSourceContentStoreError::StoredPromotionBoundary)?;
    let envelope = SourceIngestEnvelope {
        source_ref: receipt.source_ref,
        source_revision_ref: receipt.source_revision_ref,
        provider_ref: receipt.provider_ref,
        family,
        role_class,
        content_digest_ref: receipt.content_digest_ref,
        acquisition_receipt_ref: receipt.acquisition_receipt_ref,
        media_type_ref: receipt.media_type_ref,
        candidate_only: receipt.candidate_only,
        creates_semantic_authority: receipt.creates_semantic_authority,
        applicability_promoted: receipt.applicability_promoted,
        claim_truth_promoted: receipt.claim_truth_promoted,
    };
    envelope
        .validate()
        .map_err(GenericSourceContentStoreError::InvalidEnvelope)?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_and_role_are_orthogonal_persistence_coordinates() {
        assert_eq!(family_db(SourceFamily::Document), "document");
        assert_eq!(family_db(SourceFamily::Calendar), "calendar");
        assert_eq!(role_db(IngestRoleClass::ContentSource), "content_source");
        assert_eq!(
            role_db(IngestRoleClass::OperationalSource),
            "operational_source"
        );
    }

    #[test]
    fn sha256_reference_shape_matches_ingest_envelope_contract() {
        let text = "A book sentence.";
        let digest_ref = format!("sha256:{}", hex(&sha256_bytes(text.as_bytes())));
        assert!(digest_ref.starts_with("sha256:"));
        assert_eq!(digest_ref.len(), "sha256:".len() + 64);
    }
}
