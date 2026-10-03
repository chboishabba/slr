//! Provider-neutral exact legal-source manifestation receipts.
//!
//! This is the runtime counterpart of the exact OALC source-manifestation
//! boundary: provider/native identifiers and immutable payload digests are
//! bound to an already-persisted `legal_source_revision`.  The receipt proves
//! byte identity/source lineage only.  It does not choose an evidentiary role,
//! create legal authority, or promote a proposition to truth.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::DatabaseConfig;

pub const LEGAL_SOURCE_MANIFESTATION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;
CREATE TABLE IF NOT EXISTS source_provenance.legal_source_manifestation (
  manifestation_ref TEXT PRIMARY KEY,
  source_revision_ref TEXT NOT NULL REFERENCES legal_source_revision(source_revision_ref),
  document_ref TEXT NOT NULL,
  canonical_ref TEXT NOT NULL,
  source_family_ref TEXT NOT NULL,
  provider_ref TEXT NOT NULL,
  native_source_ref TEXT NOT NULL,
  native_revision_ref TEXT NOT NULL,
  canonical_sha256 BYTEA NOT NULL,
  acquisition_receipt_ref TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE (source_revision_ref, provider_ref, native_source_ref, native_revision_ref)
);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalSourceManifestationDraft {
    pub source_revision_ref: String,
    pub document_ref: String,
    pub source_family_ref: String,
    pub provider_ref: String,
    pub native_source_ref: String,
    pub native_revision_ref: String,
    /// Lowercase hexadecimal SHA-256 over the exact persisted canonical bytes.
    pub expected_canonical_sha256_hex: String,
    pub acquisition_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalSourceManifestation {
    pub manifestation_ref: String,
    pub source_revision_ref: String,
    pub document_ref: String,
    pub canonical_ref: String,
    pub source_family_ref: String,
    pub provider_ref: String,
    pub native_source_ref: String,
    pub native_revision_ref: String,
    pub canonical_sha256_hex: String,
    pub acquisition_receipt_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum LegalSourceManifestationError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("required source-manifestation coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("expected canonical SHA-256 is not 64 lowercase hexadecimal characters")]
    InvalidDigest,
    #[error("legal source revision/document/canonical payload does not reopen exactly")]
    SourceCoordinateMismatch,
    #[error("persisted canonical payload digest differs from the declared native source digest")]
    CanonicalDigestMismatch,
    #[error("persisted source manifestation conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("source manifestation not found: {0}")]
    NotFound(String),
}

pub fn install_legal_source_manifestation_schema(
    config: &DatabaseConfig,
) -> Result<(), LegalSourceManifestationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(LEGAL_SOURCE_MANIFESTATION_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_legal_source_manifestation(
    config: &DatabaseConfig,
    draft: &LegalSourceManifestationDraft,
) -> Result<PersistedLegalSourceManifestation, LegalSourceManifestationError> {
    validate_draft(draft)?;
    install_legal_source_manifestation_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client.query_opt(
        r#"SELECT d.canonical_ref,c.payload,l.compile_eligible
           FROM legal_source_revision l
           JOIN corpus.document d ON d.document_ref=l.document_ref
           JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
           WHERE l.source_revision_ref=$1 AND l.document_ref=$2"#,
        &[&draft.source_revision_ref, &draft.document_ref],
    )?.ok_or(LegalSourceManifestationError::SourceCoordinateMismatch)?;
    if !row.get::<_, bool>(2) {
        return Err(LegalSourceManifestationError::SourceCoordinateMismatch);
    }
    let canonical_ref:String=row.get(0);
    let payload:Vec<u8>=row.get(1);
    let canonical_digest=digest_bytes(&payload);
    let canonical_hex=hex(&canonical_digest);
    if canonical_hex != draft.expected_canonical_sha256_hex {
        return Err(LegalSourceManifestationError::CanonicalDigestMismatch);
    }
    let manifestation_ref=stable_ref(&[
        "slr-legal-source-manifestation:v1",
        &draft.source_revision_ref,
        &draft.document_ref,
        &canonical_ref,
        &draft.source_family_ref,
        &draft.provider_ref,
        &draft.native_source_ref,
        &draft.native_revision_ref,
        &canonical_hex,
        &draft.acquisition_receipt_ref,
    ]);
    client.execute(
        r#"INSERT INTO source_provenance.legal_source_manifestation
           (manifestation_ref,source_revision_ref,document_ref,canonical_ref,source_family_ref,
            provider_ref,native_source_ref,native_revision_ref,canonical_sha256,
            acquisition_receipt_ref,candidate_only,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,true,false,false,false,false)
           ON CONFLICT (manifestation_ref) DO NOTHING"#,
        &[&manifestation_ref,&draft.source_revision_ref,&draft.document_ref,&canonical_ref,
          &draft.source_family_ref,&draft.provider_ref,&draft.native_source_ref,
          &draft.native_revision_ref,&&canonical_digest[..],&draft.acquisition_receipt_ref],
    )?;
    let loaded=load_legal_source_manifestation(config,&manifestation_ref)?;
    if loaded.source_revision_ref!=draft.source_revision_ref
        ||loaded.document_ref!=draft.document_ref
        ||loaded.canonical_ref!=canonical_ref
        ||loaded.source_family_ref!=draft.source_family_ref
        ||loaded.provider_ref!=draft.provider_ref
        ||loaded.native_source_ref!=draft.native_source_ref
        ||loaded.native_revision_ref!=draft.native_revision_ref
        ||loaded.canonical_sha256_hex!=canonical_hex
        ||loaded.acquisition_receipt_ref!=draft.acquisition_receipt_ref
        ||!loaded.candidate_only||loaded.creates_semantic_authority
        ||loaded.creates_legal_authority||loaded.applicability_promoted||loaded.claim_truth_promoted
    { return Err(LegalSourceManifestationError::ExistingRowConflict); }
    Ok(loaded)
}

pub fn load_legal_source_manifestation(
    config:&DatabaseConfig,
    manifestation_ref:&str,
)->Result<PersistedLegalSourceManifestation,LegalSourceManifestationError>{
    if manifestation_ref.trim().is_empty(){return Err(LegalSourceManifestationError::EmptyCoordinate("manifestation_ref"));}
    let mut client=Client::connect(config.database_url(),NoTls)?;
    let row=client.query_opt(r#"SELECT manifestation_ref,source_revision_ref,document_ref,canonical_ref,
      source_family_ref,provider_ref,native_source_ref,native_revision_ref,canonical_sha256,
      acquisition_receipt_ref,candidate_only,creates_semantic_authority,creates_legal_authority,
      applicability_promoted,claim_truth_promoted
      FROM source_provenance.legal_source_manifestation WHERE manifestation_ref=$1"#,&[&manifestation_ref])?
      .ok_or_else(||LegalSourceManifestationError::NotFound(manifestation_ref.to_owned()))?;
    let digest:Vec<u8>=row.get(8);
    Ok(PersistedLegalSourceManifestation{
      manifestation_ref:row.get(0),source_revision_ref:row.get(1),document_ref:row.get(2),canonical_ref:row.get(3),
      source_family_ref:row.get(4),provider_ref:row.get(5),native_source_ref:row.get(6),native_revision_ref:row.get(7),
      canonical_sha256_hex:hex(&digest),acquisition_receipt_ref:row.get(9),candidate_only:row.get(10),
      creates_semantic_authority:row.get(11),creates_legal_authority:row.get(12),applicability_promoted:row.get(13),claim_truth_promoted:row.get(14)})
}

fn validate_draft(d:&LegalSourceManifestationDraft)->Result<(),LegalSourceManifestationError>{
    for (name,value) in [("source_revision_ref",d.source_revision_ref.as_str()),("document_ref",d.document_ref.as_str()),
      ("source_family_ref",d.source_family_ref.as_str()),("provider_ref",d.provider_ref.as_str()),
      ("native_source_ref",d.native_source_ref.as_str()),("native_revision_ref",d.native_revision_ref.as_str()),
      ("acquisition_receipt_ref",d.acquisition_receipt_ref.as_str())]{if value.trim().is_empty(){return Err(LegalSourceManifestationError::EmptyCoordinate(name));}}
    let digest=d.expected_canonical_sha256_hex.as_bytes();
    if digest.len()!=64||!digest.iter().all(|b|matches!(b,b'0'..=b'9'|b'a'..=b'f')){return Err(LegalSourceManifestationError::InvalidDigest);}
    Ok(())
}
fn stable_ref(parts:&[&str])->String{format!("legal-source-manifestation:sha256:{}",hex(&digest_parts(parts)))}
fn digest_parts(parts:&[&str])->[u8;32]{let mut h=Sha256::new();for p in parts{h.update((p.len() as u64).to_be_bytes());h.update(p.as_bytes());}h.finalize().into()}
fn digest_bytes(bytes:&[u8])->[u8;32]{Sha256::digest(bytes).into()}
fn hex(bytes:&[u8])->String{const D:&[u8;16]=b"0123456789abcdef";let mut o=String::with_capacity(bytes.len()*2);for &b in bytes{o.push(D[(b>>4)as usize]as char);o.push(D[(b&15)as usize]as char);}o}

#[cfg(test)]
mod tests{use super::*;#[test]fn rejects_noncanonical_digest(){let d=LegalSourceManifestationDraft{source_revision_ref:"s".into(),document_ref:"d".into(),source_family_ref:"legal".into(),provider_ref:"oalc".into(),native_source_ref:"mabo".into(),native_revision_ref:"r".into(),expected_canonical_sha256_hex:"ABC".into(),acquisition_receipt_ref:"a".into()};assert!(matches!(validate_draft(&d),Err(LegalSourceManifestationError::InvalidDigest)));}}
