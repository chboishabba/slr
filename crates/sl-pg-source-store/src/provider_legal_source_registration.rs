//! Provider-materialisation -> curated legal-source registration.
//!
//! Port of the historical SensibLaw `persist_source_admission_receipts` +
//! `persist_legal_source_revision` transaction boundary, adapted to the current
//! SLR provider materialisation spine.  Registration is source/compile
//! eligibility only: it does not assign an evidence role, normative order,
//! applicability, legal authority, or claim truth.

use postgres::{Client, NoTls};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    load_provider_materialization, DatabaseConfig, PersistedProviderMaterialization,
    ProviderMaterializationError,
};

pub const PROVIDER_LEGAL_SOURCE_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS source_admission_receipt (
    receipt_ref TEXT PRIMARY KEY,
    corpus_ref TEXT,
    source_revision_ref TEXT NOT NULL,
    source_role TEXT NOT NULL,
    semantic_scope TEXT NOT NULL,
    admission_state TEXT NOT NULL CHECK (
        admission_state IN ('compile', 'evidence_only', 'exclude')
    ),
    exclusion_reason TEXT,
    profile_ref TEXT NOT NULL,
    contract_ref TEXT NOT NULL,
    semantic_state_promoted BOOLEAN NOT NULL DEFAULT FALSE
        CHECK (semantic_state_promoted = FALSE),
    legal_truth_closed BOOLEAN NOT NULL DEFAULT FALSE
        CHECK (legal_truth_closed = FALSE),
    receipt_sha256 BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (source_revision_ref, profile_ref)
);

CREATE TABLE IF NOT EXISTS legal_source_revision (
    source_revision_ref TEXT PRIMARY KEY,
    document_ref TEXT NOT NULL,
    admission_receipt_ref TEXT NOT NULL
        REFERENCES source_admission_receipt(receipt_ref),
    acquisition_receipt_ref TEXT,
    jurisdiction_ref TEXT NOT NULL,
    source_role TEXT NOT NULL,
    authority_level TEXT NOT NULL,
    temporal_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    provider_profile_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    media_type TEXT NOT NULL,
    canonical_text_sha256 TEXT NOT NULL,
    compile_eligible BOOLEAN NOT NULL DEFAULT TRUE
        CHECK (compile_eligible = TRUE),
    identity_promoted BOOLEAN NOT NULL DEFAULT FALSE
        CHECK (identity_promoted = FALSE),
    applicability_closed BOOLEAN NOT NULL DEFAULT FALSE
        CHECK (applicability_closed = FALSE),
    legal_truth_closed BOOLEAN NOT NULL DEFAULT FALSE
        CHECK (legal_truth_closed = FALSE),
    revision_sha256 BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS legal_source_revision_selection_idx
    ON legal_source_revision
    (jurisdiction_ref, source_role, authority_level, compile_eligible);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderLegalSourceRegistrationDraft {
    pub materialization_ref: String,
    pub corpus_ref: String,
    pub admission_profile_ref: String,
    pub source_role: String,
    pub authority_level: String,
    pub semantic_scope: String,
    pub temporal_refs: Vec<String>,
    pub provider_profile_refs: Vec<String>,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedProviderLegalSourceRegistration {
    pub source_revision_ref: String,
    pub document_ref: String,
    pub materialization_ref: String,
    pub admission_receipt_ref: String,
    pub provider_acquisition_receipt_ref: String,
    pub jurisdiction_ref: String,
    pub source_role: String,
    pub authority_level: String,
    pub semantic_scope: String,
    pub temporal_refs: Vec<String>,
    pub provider_profile_refs: Vec<String>,
    pub media_type: String,
    pub canonical_text_sha256: String,
    pub compile_eligible: bool,
    pub bytes_residency_required_for_identity: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum ProviderLegalSourceRegistrationError {
    #[error(transparent)]
    Provider(#[from] ProviderMaterializationError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("legal-source registration coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("legal-source registration requires at least one provider profile ref")]
    MissingProviderProfile,
    #[error("legal-source registration provider profile does not identify the persisted provider")]
    WrongProviderProfile,
    #[error("persisted legal-source registration conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("legal-source registration not found: {0}")]
    NotFound(String),
}

pub fn install_provider_legal_source_schema(
    config: &DatabaseConfig,
) -> Result<(), ProviderLegalSourceRegistrationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(PROVIDER_LEGAL_SOURCE_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_provider_legal_source_registration(
    config: &DatabaseConfig,
    draft: &ProviderLegalSourceRegistrationDraft,
) -> Result<PersistedProviderLegalSourceRegistration, ProviderLegalSourceRegistrationError> {
    validate_draft(draft)?;
    let materialization = load_provider_materialization(config, &draft.materialization_ref)?;
    if !draft
        .provider_profile_refs
        .iter()
        .any(|value| value == &materialization.identity.provider_ref)
    {
        return Err(ProviderLegalSourceRegistrationError::WrongProviderProfile);
    }
    install_provider_legal_source_schema(config)?;

    // Use one source-revision identity across the provider/source-resolution and
    // curated legal-source layers. This is an identity weld, not a truth weld.
    let source_revision_ref = materialization.external_source_revision_ref.clone();
    let admission_receipt_ref = admission_ref(draft, &materialization, &source_revision_ref);
    let admission_digest = digest_parts(&[
        "slr-source-admission:v1",
        &source_revision_ref,
        &draft.corpus_ref,
        &draft.admission_profile_ref,
        &draft.source_role,
        &draft.semantic_scope,
        "compile",
    ]);
    let temporal_json = serde_json::to_string(&sorted_unique(&draft.temporal_refs))
        .expect("serializing string refs cannot fail");
    let provider_json = serde_json::to_string(&sorted_unique(&draft.provider_profile_refs))
        .expect("serializing string refs cannot fail");
    let revision_digest = digest_parts(&[
        "slr-legal-source-revision:v1",
        &source_revision_ref,
        &materialization.document_ref,
        &admission_receipt_ref,
        &materialization.identity.acquisition_receipt_ref,
        &materialization.identity.jurisdiction_ref,
        &draft.source_role,
        &draft.authority_level,
        &temporal_json,
        &provider_json,
        &draft.media_type,
        &materialization.canonical_sha256_hex,
    ]);

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        r#"INSERT INTO source_admission_receipt
           (receipt_ref,corpus_ref,source_revision_ref,source_role,semantic_scope,
            admission_state,exclusion_reason,profile_ref,contract_ref,
            semantic_state_promoted,legal_truth_closed,receipt_sha256)
           VALUES ($1,$2,$3,$4,$5,'compile',NULL,$6,'source-admission:v0_2',FALSE,FALSE,$7)
           ON CONFLICT (receipt_ref) DO NOTHING"#,
        &[
            &admission_receipt_ref,
            &draft.corpus_ref,
            &source_revision_ref,
            &draft.source_role,
            &draft.semantic_scope,
            &draft.admission_profile_ref,
            &&admission_digest[..],
        ],
    )?;
    tx.execute(
        r#"INSERT INTO legal_source_revision
           (source_revision_ref,document_ref,admission_receipt_ref,acquisition_receipt_ref,
            jurisdiction_ref,source_role,authority_level,temporal_refs,provider_profile_refs,
            media_type,canonical_text_sha256,compile_eligible,identity_promoted,
            applicability_closed,legal_truth_closed,revision_sha256)
           VALUES ($1,$2,$3,NULL,$4,$5,$6,$7::jsonb,$8::jsonb,$9,$10,
                   TRUE,FALSE,FALSE,FALSE,$11)
           ON CONFLICT (source_revision_ref) DO NOTHING"#,
        &[
            &source_revision_ref,
            &materialization.document_ref,
            &admission_receipt_ref,
            &materialization.identity.jurisdiction_ref,
            &draft.source_role,
            &draft.authority_level,
            &temporal_json,
            &provider_json,
            &draft.media_type,
            &materialization.canonical_sha256_hex,
            &&revision_digest[..],
        ],
    )?;
    tx.commit()?;

    let loaded = load_provider_legal_source_registration(config, &source_revision_ref)?;
    if loaded.materialization_ref != draft.materialization_ref
        || loaded.document_ref != materialization.document_ref
        || loaded.admission_receipt_ref != admission_receipt_ref
        || loaded.provider_acquisition_receipt_ref
            != materialization.identity.acquisition_receipt_ref
        || loaded.jurisdiction_ref != materialization.identity.jurisdiction_ref
        || loaded.source_role != draft.source_role
        || loaded.authority_level != draft.authority_level
        || loaded.semantic_scope != draft.semantic_scope
        || loaded.temporal_refs != sorted_unique(&draft.temporal_refs)
        || loaded.provider_profile_refs != sorted_unique(&draft.provider_profile_refs)
        || loaded.media_type != draft.media_type
        || loaded.canonical_text_sha256 != materialization.canonical_sha256_hex
        || !loaded.compile_eligible
        || loaded.bytes_residency_required_for_identity
        || loaded.creates_semantic_authority
        || loaded.creates_legal_authority
        || loaded.applicability_promoted
        || loaded.claim_truth_promoted
    {
        return Err(ProviderLegalSourceRegistrationError::ExistingRowConflict);
    }
    Ok(loaded)
}

pub fn load_provider_legal_source_registration(
    config: &DatabaseConfig,
    source_revision_ref: &str,
) -> Result<PersistedProviderLegalSourceRegistration, ProviderLegalSourceRegistrationError> {
    required("source_revision_ref", source_revision_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT l.document_ref,l.admission_receipt_ref,l.jurisdiction_ref,l.source_role,
                      l.authority_level,l.temporal_refs::text,l.provider_profile_refs::text,
                      l.media_type,l.canonical_text_sha256,l.compile_eligible,
                      l.identity_promoted,l.applicability_closed,l.legal_truth_closed,
                      a.semantic_scope,a.semantic_state_promoted,a.legal_truth_closed,
                      m.materialization_ref,m.acquisition_receipt_ref,m.creates_semantic_authority,
                      m.creates_legal_authority,m.applicability_promoted,m.claim_truth_promoted
               FROM legal_source_revision l
               JOIN source_admission_receipt a ON a.receipt_ref=l.admission_receipt_ref
               JOIN source_provenance.provider_materialization m
                 ON m.external_source_revision_ref=l.source_revision_ref
                AND m.document_ref=l.document_ref
               WHERE l.source_revision_ref=$1"#,
            &[&source_revision_ref],
        )?
        .ok_or_else(|| ProviderLegalSourceRegistrationError::NotFound(source_revision_ref.to_owned()))?;

    let compile_eligible: bool = row.get(9);
    let identity_promoted: bool = row.get(10);
    let applicability_closed: bool = row.get(11);
    let legal_truth_closed: bool = row.get(12);
    let admission_semantic_promoted: bool = row.get(14);
    let admission_truth_closed: bool = row.get(15);
    let materialization_semantic_authority: bool = row.get(18);
    let materialization_legal_authority: bool = row.get(19);
    let materialization_applicability: bool = row.get(20);
    let materialization_truth: bool = row.get(21);
    if !compile_eligible
        || identity_promoted
        || applicability_closed
        || legal_truth_closed
        || admission_semantic_promoted
        || admission_truth_closed
        || materialization_semantic_authority
        || materialization_legal_authority
        || materialization_applicability
        || materialization_truth
    {
        return Err(ProviderLegalSourceRegistrationError::ExistingRowConflict);
    }

    let temporal_refs: Vec<String> = serde_json::from_str(&row.get::<_, String>(5))
        .map_err(|_| ProviderLegalSourceRegistrationError::ExistingRowConflict)?;
    let provider_profile_refs: Vec<String> = serde_json::from_str(&row.get::<_, String>(6))
        .map_err(|_| ProviderLegalSourceRegistrationError::ExistingRowConflict)?;

    Ok(PersistedProviderLegalSourceRegistration {
        source_revision_ref: source_revision_ref.to_owned(),
        document_ref: row.get(0),
        materialization_ref: row.get(16),
        admission_receipt_ref: row.get(1),
        provider_acquisition_receipt_ref: row.get(17),
        jurisdiction_ref: row.get(2),
        source_role: row.get(3),
        authority_level: row.get(4),
        semantic_scope: row.get(13),
        temporal_refs,
        provider_profile_refs,
        media_type: row.get(7),
        canonical_text_sha256: row.get(8),
        compile_eligible: true,
        bytes_residency_required_for_identity: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

fn validate_draft(
    draft: &ProviderLegalSourceRegistrationDraft,
) -> Result<(), ProviderLegalSourceRegistrationError> {
    for (name, value) in [
        ("materialization_ref", draft.materialization_ref.as_str()),
        ("corpus_ref", draft.corpus_ref.as_str()),
        ("admission_profile_ref", draft.admission_profile_ref.as_str()),
        ("source_role", draft.source_role.as_str()),
        ("authority_level", draft.authority_level.as_str()),
        ("semantic_scope", draft.semantic_scope.as_str()),
        ("media_type", draft.media_type.as_str()),
    ] {
        required(name, value)?;
    }
    if draft.provider_profile_refs.is_empty()
        || draft.provider_profile_refs.iter().any(|value| value.trim().is_empty())
    {
        return Err(ProviderLegalSourceRegistrationError::MissingProviderProfile);
    }
    Ok(())
}

fn admission_ref(
    draft: &ProviderLegalSourceRegistrationDraft,
    materialization: &PersistedProviderMaterialization,
    source_revision_ref: &str,
) -> String {
    format!(
        "source-admission:sha256:{}",
        hex(&digest_parts(&[
            "slr-source-admission-ref:v1",
            source_revision_ref,
            &materialization.document_ref,
            &draft.corpus_ref,
            &draft.admission_profile_ref,
            &draft.source_role,
            &draft.semantic_scope,
        ]))
    )
}

fn sorted_unique(values: &[String]) -> Vec<String> {
    let mut result = values.to_vec();
    result.sort();
    result.dedup();
    result
}

fn required(
    name: &'static str,
    value: &str,
) -> Result<(), ProviderLegalSourceRegistrationError> {
    if value.trim().is_empty() {
        Err(ProviderLegalSourceRegistrationError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
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
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_registration_is_explicitly_not_review_or_truth() {
        fn assert_boundary(value: &PersistedProviderLegalSourceRegistration) {
            assert!(value.compile_eligible);
            assert!(!value.bytes_residency_required_for_identity);
            assert!(!value.creates_semantic_authority);
            assert!(!value.creates_legal_authority);
            assert!(!value.applicability_promoted);
            assert!(!value.claim_truth_promoted);
        }
        let _ = assert_boundary as fn(&PersistedProviderLegalSourceRegistration);
    }
}
