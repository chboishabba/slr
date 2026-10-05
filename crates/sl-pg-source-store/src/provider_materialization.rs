//! Provider-pinned, eviction-safe source materialisation.
//!
//! Provider discovery may navigate without resident full text. Exact quotation,
//! parser and review actions require an exact materialisation whose bytes
//! reproduce the durable provider pin and canonical digest. Full source bytes
//! are cache state, not semantic/legal authority and not a required archive.

use postgres::{Client, NoTls, Transaction};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    DatabaseConfig, ExactResolutionReceipt, PostgresSourceStore, ResolvedExternalDocument,
    SourceSlice, SourceStoreError,
};

pub const PROVIDER_MATERIALIZATION_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS source_provenance;

-- Provider-backed documents can be re-materialised from an immutable upstream
-- pin. Identity/digest/length therefore remain when resident cache bytes are
-- evicted. Existing retained/local sources continue to keep payloads normally.
ALTER TABLE corpus.canonical_content ALTER COLUMN payload DROP NOT NULL;

CREATE TABLE IF NOT EXISTS source_provenance.provider_materialization (
  materialization_ref TEXT PRIMARY KEY,
  external_source_revision_ref TEXT NOT NULL
    REFERENCES corpus.external_source_revision(external_source_revision_ref),
  document_ref TEXT NOT NULL REFERENCES corpus.document(document_ref),
  canonical_ref TEXT NOT NULL REFERENCES corpus.canonical_content(canonical_ref),
  provider_ref TEXT NOT NULL,
  dataset_ref TEXT NOT NULL,
  dataset_revision_ref TEXT NOT NULL,
  split_ref TEXT NOT NULL,
  external_version_ref TEXT NOT NULL,
  citation TEXT NOT NULL,
  source_ref TEXT NOT NULL,
  jurisdiction_ref TEXT NOT NULL,
  source_url TEXT NULL,
  acquisition_receipt_ref TEXT NOT NULL,
  canonical_sha256_hex TEXT NOT NULL,
  canonical_byte_length BIGINT NOT NULL CHECK (canonical_byte_length >= 0),
  allow_byte_eviction BOOLEAN NOT NULL CHECK (allow_byte_eviction),
  retain_full_text_by_default BOOLEAN NOT NULL CHECK (NOT retain_full_text_by_default),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (provider_ref,dataset_ref,dataset_revision_ref,split_ref,external_version_ref,citation,jurisdiction_ref)
);
CREATE INDEX IF NOT EXISTS provider_materialization_revision_idx
  ON source_provenance.provider_materialization(external_source_revision_ref);
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderMaterializationIdentity {
    pub provider_ref: String,
    pub dataset_ref: String,
    pub dataset_revision_ref: String,
    pub split_ref: String,
    pub external_version_ref: String,
    pub citation: String,
    pub source_ref: String,
    pub jurisdiction_ref: String,
    pub source_url: Option<String>,
    pub acquisition_receipt_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderMaterializationPolicy {
    pub allow_byte_eviction: bool,
    pub retain_full_text_by_default: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub promotes_applicability: bool,
    pub promotes_claim_truth: bool,
}

impl Default for ProviderMaterializationPolicy {
    fn default() -> Self {
        Self {
            allow_byte_eviction: true,
            retain_full_text_by_default: false,
            creates_semantic_authority: false,
            creates_legal_authority: false,
            promotes_applicability: false,
            promotes_claim_truth: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedProviderMaterialization {
    pub materialization_ref: String,
    pub external_source_revision_ref: String,
    pub document_ref: String,
    pub canonical_ref: String,
    pub identity: ProviderMaterializationIdentity,
    pub canonical_sha256_hex: String,
    pub canonical_byte_length: i64,
    pub bytes_resident: bool,
    pub policy: ProviderMaterializationPolicy,
}

#[derive(Debug, Error)]
pub enum ProviderMaterializationError {
    #[error(transparent)]
    Store(#[from] SourceStoreError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("provider materialisation coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("provider revision is not immutable: {0}")]
    MutableProviderRevision(String),
    #[error("OALC/HF provider revision is not a commit-like immutable pin: {0}")]
    UnverifiableOalcRevision(String),
    #[error("provider materialisation requires an exact source-resolution receipt")]
    NonExactResolution,
    #[error("provider materialisation identity disagrees with the acquired document: {0}")]
    IdentityMismatch(&'static str),
    #[error("pinned provider record re-materialised with a different canonical digest")]
    RematerializationDigestMismatch,
    #[error("persisted provider materialisation changed under the same immutable identity")]
    ExistingRowConflict,
    #[error("persisted source revision does not match the provider pin/document/digest/resolution")]
    RevisionBindingConflict,
    #[error("provider materialisation not found: {0}")]
    NotFound(String),
    #[error("full source bytes are not resident; strict action requires exact re-materialisation")]
    BytesNotResident,
    #[error("canonical payload is shared with a non-provider/local document and cannot be evicted")]
    SharedCanonicalPayload,
    #[error("source slice is not owned by this provider materialisation: {0}")]
    WrongSliceOwner(String),
    #[error("persisted source slice digest no longer matches resident bytes")]
    SliceDigestMismatch,
    #[error("resident provider bytes are not valid UTF-8")]
    InvalidUtf8,
}

fn required(name: &'static str, value: &str) -> Result<(), ProviderMaterializationError> {
    if value.trim().is_empty() {
        Err(ProviderMaterializationError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn is_hex_commit_pin(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Validate only source immutability/identity syntax. This grants no authority.
/// OALC/HF revisions are Git/Hugging-Face revisions, so their production pin is
/// required to be a 40- or 64-hex commit-like identifier. Other providers may
/// use provider-native immutable identifiers after the generic mutable aliases
/// are rejected.
pub fn validate_provider_pin(
    identity: &ProviderMaterializationIdentity,
) -> Result<(), ProviderMaterializationError> {
    for (name, value) in [
        ("provider_ref", identity.provider_ref.as_str()),
        ("dataset_ref", identity.dataset_ref.as_str()),
        ("dataset_revision_ref", identity.dataset_revision_ref.as_str()),
        ("split_ref", identity.split_ref.as_str()),
        ("external_version_ref", identity.external_version_ref.as_str()),
        ("citation", identity.citation.as_str()),
        ("source_ref", identity.source_ref.as_str()),
        ("jurisdiction_ref", identity.jurisdiction_ref.as_str()),
        ("acquisition_receipt_ref", identity.acquisition_receipt_ref.as_str()),
    ] {
        required(name, value)?;
    }

    let revision = identity.dataset_revision_ref.trim();
    let normalized = revision.to_ascii_lowercase();
    let mutable = matches!(normalized.as_str(), "main" | "master" | "latest" | "head")
        || normalized.starts_with("refs/heads/")
        || normalized.ends_with(":latest");
    if mutable {
        return Err(ProviderMaterializationError::MutableProviderRevision(
            identity.dataset_revision_ref.clone(),
        ));
    }

    let provider = identity.provider_ref.to_ascii_lowercase();
    let dataset = identity.dataset_ref.to_ascii_lowercase();
    let is_oalc_hf = provider.contains("oalc")
        || provider.contains("huggingface")
        || provider.contains("hf")
        || dataset == "isaacus/open-australian-legal-corpus";
    if is_oalc_hf && !is_hex_commit_pin(revision) {
        return Err(ProviderMaterializationError::UnverifiableOalcRevision(
            identity.dataset_revision_ref.clone(),
        ));
    }
    Ok(())
}

/// SHA-256 of the exact resident canonical UTF-8 bytes.
pub fn canonical_sha256_hex(text: &str) -> String {
    hex(&sha256(text.as_bytes()))
}

/// Stable materialisation identity over immutable provider coordinates + bytes.
pub fn canonical_provider_materialization_ref(
    identity: &ProviderMaterializationIdentity,
    canonical_sha256_hex: &str,
) -> Result<String, ProviderMaterializationError> {
    validate_provider_pin(identity)?;
    required("canonical_sha256_hex", canonical_sha256_hex)?;
    let value = framed_digest(&[
        &identity.provider_ref,
        &identity.dataset_ref,
        &identity.dataset_revision_ref,
        &identity.split_ref,
        &identity.external_version_ref,
        &identity.citation,
        &identity.source_ref,
        &identity.jurisdiction_ref,
        identity.source_url.as_deref().unwrap_or(""),
        &identity.acquisition_receipt_ref,
        canonical_sha256_hex,
    ]);
    Ok(format!("provider-materialization:sha256:{}", hex(&value)))
}

/// Strict re-materialisation check; changed bytes under one pin fail closed.
pub fn verify_provider_rematerialization(
    expected_sha256_hex: &str,
    canonical_text: &str,
) -> Result<(), ProviderMaterializationError> {
    if canonical_sha256_hex(canonical_text) == expected_sha256_hex {
        Ok(())
    } else {
        Err(ProviderMaterializationError::RematerializationDigestMismatch)
    }
}

/// Install the provider cache metadata extension. The canonical payload becomes
/// nullable because provider bytes are cache state; legacy readers must treat a
/// NULL payload as non-resident rather than as a missing durable source.
pub fn install_provider_materialization_schema(
    config: &DatabaseConfig,
) -> Result<(), ProviderMaterializationError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(PROVIDER_MATERIALIZATION_SCHEMA_SQL)?;
    Ok(())
}

/// Persist a newly acquired exact source under an immutable provider pin.
///
/// The session advisory lock uses the legacy external-source uniqueness key,
/// deliberately excluding `split_ref`, because the legacy table also excludes
/// split. This serialises competing first writers before their source revision
/// can be reused. The returned revision/document/digest/resolution are then
/// revalidated before the provider materialisation is accepted.
pub fn persist_provider_materialization(
    config: &DatabaseConfig,
    identity: &ProviderMaterializationIdentity,
    document: &ResolvedExternalDocument<'_>,
    resolution: &ExactResolutionReceipt<'_>,
    slices: &[SourceSlice<'_>],
) -> Result<PersistedProviderMaterialization, ProviderMaterializationError> {
    validate_provider_pin(identity)?;
    validate_identity(identity, document, resolution)?;
    if !resolution.exact_demand_match {
        return Err(ProviderMaterializationError::NonExactResolution);
    }
    install_provider_materialization_schema(config)?;

    let digest_hex = canonical_sha256_hex(document.canonical_text);
    let materialization_ref = canonical_provider_materialization_ref(identity, &digest_hex)?;

    // Hold this connection for the full transition. Session advisory locks are
    // released automatically if any later error drops the connection.
    let mut guard = Client::connect(config.database_url(), NoTls)?;
    let lock_key = provider_revision_lock_key(identity);
    guard.query_one("SELECT pg_advisory_lock($1)", &[&lock_key])?;

    if let Some(binding) = existing_revision_binding(&mut guard, identity)? {
        if binding.canonical_sha256_hex != digest_hex {
            return Err(ProviderMaterializationError::RematerializationDigestMismatch);
        }
        rehydrate_matching_canonical_payload(&mut guard, identity, document.canonical_text)?;
    }

    let mut store = PostgresSourceStore::connect(config)?;
    let refs = store.persist_resolved_source(document, resolution, slices)?;

    let binding = revision_binding_for_refs(
        &mut guard,
        &refs.external_source_revision_ref,
        &refs.source_resolution_ref,
    )?;
    validate_revision_binding(identity, document, resolution, &refs.document_ref, &digest_hex, &binding)?;

    guard.execute(
        r#"INSERT INTO source_provenance.provider_materialization
           (materialization_ref,external_source_revision_ref,document_ref,canonical_ref,
            provider_ref,dataset_ref,dataset_revision_ref,split_ref,external_version_ref,
            citation,source_ref,jurisdiction_ref,source_url,acquisition_receipt_ref,
            canonical_sha256_hex,canonical_byte_length,allow_byte_eviction,
            retain_full_text_by_default,candidate_only,creates_semantic_authority,
            creates_legal_authority,applicability_promoted,claim_truth_promoted)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,
                   TRUE,FALSE,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT (materialization_ref) DO NOTHING"#,
        &[
            &materialization_ref,
            &refs.external_source_revision_ref,
            &refs.document_ref,
            &binding.canonical_ref,
            &identity.provider_ref,
            &identity.dataset_ref,
            &identity.dataset_revision_ref,
            &identity.split_ref,
            &identity.external_version_ref,
            &identity.citation,
            &identity.source_ref,
            &identity.jurisdiction_ref,
            &identity.source_url,
            &identity.acquisition_receipt_ref,
            &digest_hex,
            &binding.canonical_byte_length,
        ],
    )?;

    let reopened = load_provider_materialization(config, &materialization_ref)?;
    if reopened.identity != *identity
        || reopened.external_source_revision_ref != refs.external_source_revision_ref
        || reopened.document_ref != refs.document_ref
        || reopened.canonical_ref != binding.canonical_ref
        || reopened.canonical_sha256_hex != digest_hex
    {
        return Err(ProviderMaterializationError::ExistingRowConflict);
    }

    guard.query_one("SELECT pg_advisory_unlock($1)", &[&lock_key])?;
    Ok(reopened)
}

/// Reopen durable materialisation metadata regardless of byte residency.
pub fn load_provider_materialization(
    config: &DatabaseConfig,
    materialization_ref: &str,
) -> Result<PersistedProviderMaterialization, ProviderMaterializationError> {
    required("materialization_ref", materialization_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT m.external_source_revision_ref,m.document_ref,m.canonical_ref,
                      m.provider_ref,m.dataset_ref,m.dataset_revision_ref,m.split_ref,
                      m.external_version_ref,m.citation,m.source_ref,m.jurisdiction_ref,
                      m.source_url,m.acquisition_receipt_ref,m.canonical_sha256_hex,
                      m.canonical_byte_length,(c.payload IS NOT NULL),
                      m.allow_byte_eviction,m.retain_full_text_by_default,
                      m.candidate_only,m.creates_semantic_authority,m.creates_legal_authority,
                      m.applicability_promoted,m.claim_truth_promoted
               FROM source_provenance.provider_materialization m
               JOIN corpus.canonical_content c ON c.canonical_ref=m.canonical_ref
               WHERE m.materialization_ref=$1"#,
            &[&materialization_ref],
        )?
        .ok_or_else(|| ProviderMaterializationError::NotFound(materialization_ref.to_owned()))?;

    let policy = ProviderMaterializationPolicy {
        allow_byte_eviction: row.get(16),
        retain_full_text_by_default: row.get(17),
        creates_semantic_authority: row.get(19),
        creates_legal_authority: row.get(20),
        promotes_applicability: row.get(21),
        promotes_claim_truth: row.get(22),
    };
    if !policy.allow_byte_eviction
        || policy.retain_full_text_by_default
        || !row.get::<_, bool>(18)
        || policy.creates_semantic_authority
        || policy.creates_legal_authority
        || policy.promotes_applicability
        || policy.promotes_claim_truth
    {
        return Err(ProviderMaterializationError::ExistingRowConflict);
    }

    Ok(PersistedProviderMaterialization {
        materialization_ref: materialization_ref.to_owned(),
        external_source_revision_ref: row.get(0),
        document_ref: row.get(1),
        canonical_ref: row.get(2),
        identity: ProviderMaterializationIdentity {
            provider_ref: row.get(3),
            dataset_ref: row.get(4),
            dataset_revision_ref: row.get(5),
            split_ref: row.get(6),
            external_version_ref: row.get(7),
            citation: row.get(8),
            source_ref: row.get(9),
            jurisdiction_ref: row.get(10),
            source_url: row.get(11),
            acquisition_receipt_ref: row.get(12),
        },
        canonical_sha256_hex: row.get(13),
        canonical_byte_length: row.get(14),
        bytes_resident: row.get(15),
        policy,
    })
}

/// Remove only re-materialisable cache bytes. The canonical row is locked before
/// checking whether a retained/local document shares it, so the ownership check
/// and payload removal form one atomic lifecycle transition.
pub fn evict_provider_materialization_bytes(
    config: &DatabaseConfig,
    materialization_ref: &str,
) -> Result<PersistedProviderMaterialization, ProviderMaterializationError> {
    let persisted = load_provider_materialization(config, materialization_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;

    let row = tx.query_one(
        r#"SELECT encode(content_sha256,'hex')
           FROM corpus.canonical_content
           WHERE canonical_ref=$1
           FOR UPDATE"#,
        &[&persisted.canonical_ref],
    )?;
    let digest: String = row.get(0);
    if digest != persisted.canonical_sha256_hex {
        return Err(ProviderMaterializationError::ExistingRowConflict);
    }

    let shared: bool = tx
        .query_one(
            r#"SELECT EXISTS(
                 SELECT 1 FROM corpus.document d
                 LEFT JOIN corpus.external_source_revision r ON r.document_ref=d.document_ref
                 WHERE d.canonical_ref=$1
                   AND (r.external_source_revision_ref IS NULL
                        OR d.adapter_ref IS DISTINCT FROM 'oalc-governed-provider')
               )"#,
            &[&persisted.canonical_ref],
        )?
        .get(0);
    if shared {
        return Err(ProviderMaterializationError::SharedCanonicalPayload);
    }

    tx.execute(
        "UPDATE corpus.canonical_content SET payload=NULL WHERE canonical_ref=$1",
        &[&persisted.canonical_ref],
    )?;
    tx.commit()?;
    load_provider_materialization(config, materialization_ref)
}

/// Reinstall bytes only after exact pin/digest agreement. The canonical row is
/// locked and its immutable content digest rechecked in the same transaction as
/// the payload update.
pub fn rehydrate_provider_materialization(
    config: &DatabaseConfig,
    materialization_ref: &str,
    identity: &ProviderMaterializationIdentity,
    canonical_text: &str,
) -> Result<PersistedProviderMaterialization, ProviderMaterializationError> {
    validate_provider_pin(identity)?;
    let persisted = load_provider_materialization(config, materialization_ref)?;
    if persisted.identity != *identity {
        return Err(ProviderMaterializationError::ExistingRowConflict);
    }
    verify_provider_rematerialization(&persisted.canonical_sha256_hex, canonical_text)?;

    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    let row = tx.query_one(
        r#"SELECT encode(content_sha256,'hex')
           FROM corpus.canonical_content
           WHERE canonical_ref=$1
           FOR UPDATE"#,
        &[&persisted.canonical_ref],
    )?;
    let digest: String = row.get(0);
    if digest != persisted.canonical_sha256_hex {
        return Err(ProviderMaterializationError::ExistingRowConflict);
    }
    tx.execute(
        "UPDATE corpus.canonical_content SET payload=$2, uncompressed_byte_length=$3 WHERE canonical_ref=$1",
        &[
            &persisted.canonical_ref,
            &canonical_text.as_bytes(),
            &(canonical_text.len() as i64),
        ],
    )?;
    tx.commit()?;
    load_provider_materialization(config, materialization_ref)
}

/// Reopen a persisted exact source slice only while verified bytes are resident.
pub fn load_provider_slice_text(
    config: &DatabaseConfig,
    materialization_ref: &str,
    source_slice_ref: &str,
) -> Result<String, ProviderMaterializationError> {
    let persisted = load_provider_materialization(config, materialization_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT s.start_char,s.end_char,x.slice_sha256,c.payload
               FROM corpus.external_source_slice x
               JOIN corpus.span s ON s.span_ref=x.span_ref
               JOIN corpus.document d ON d.document_ref=s.document_ref
               JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
               WHERE x.source_slice_ref=$1
                 AND x.external_source_revision_ref=$2
                 AND d.document_ref=$3"#,
            &[
                &source_slice_ref,
                &persisted.external_source_revision_ref,
                &persisted.document_ref,
            ],
        )?
        .ok_or_else(|| ProviderMaterializationError::WrongSliceOwner(source_slice_ref.to_owned()))?;
    let payload: Option<Vec<u8>> = row.get(3);
    let payload = payload.ok_or(ProviderMaterializationError::BytesNotResident)?;
    let start: i32 = row.get(0);
    let end: i32 = row.get(1);
    let start = usize::try_from(start)
        .map_err(|_| ProviderMaterializationError::WrongSliceOwner(source_slice_ref.to_owned()))?;
    let end = usize::try_from(end)
        .map_err(|_| ProviderMaterializationError::WrongSliceOwner(source_slice_ref.to_owned()))?;
    if start >= end || end > payload.len() {
        return Err(ProviderMaterializationError::WrongSliceOwner(source_slice_ref.to_owned()));
    }
    let literal = &payload[start..end];
    let expected: Vec<u8> = row.get(2);
    if sha256(literal).as_slice() != expected.as_slice() {
        return Err(ProviderMaterializationError::SliceDigestMismatch);
    }
    String::from_utf8(literal.to_vec()).map_err(|_| ProviderMaterializationError::InvalidUtf8)
}

fn validate_identity(
    identity: &ProviderMaterializationIdentity,
    document: &ResolvedExternalDocument<'_>,
    resolution: &ExactResolutionReceipt<'_>,
) -> Result<(), ProviderMaterializationError> {
    let checks = [
        ("provider_ref", identity.provider_ref.as_str(), document.provider_ref),
        ("dataset_ref", identity.dataset_ref.as_str(), document.dataset_ref),
        (
            "dataset_revision_ref",
            identity.dataset_revision_ref.as_str(),
            document.dataset_revision_ref,
        ),
        (
            "external_version_ref",
            identity.external_version_ref.as_str(),
            document.external_version_ref,
        ),
        ("citation", identity.citation.as_str(), document.citation),
        ("source_ref", identity.source_ref.as_str(), document.source_ref),
        (
            "jurisdiction_ref",
            identity.jurisdiction_ref.as_str(),
            document.jurisdiction_ref,
        ),
        (
            "acquisition_receipt_ref",
            identity.acquisition_receipt_ref.as_str(),
            resolution.resolution_evidence_ref,
        ),
    ];
    for (name, expected, actual) in checks {
        if expected != actual {
            return Err(ProviderMaterializationError::IdentityMismatch(name));
        }
    }
    if identity.source_url.as_deref() != document.source_url {
        return Err(ProviderMaterializationError::IdentityMismatch("source_url"));
    }
    Ok(())
}

#[derive(Debug)]
struct RevisionBinding {
    external_source_revision_ref: String,
    document_ref: String,
    canonical_ref: String,
    canonical_sha256_hex: String,
    canonical_byte_length: i64,
    provider_ref: String,
    dataset_ref: String,
    dataset_revision_ref: String,
    external_version_ref: String,
    citation: String,
    source_ref: String,
    jurisdiction_ref: String,
    source_url: Option<String>,
    source_resolution_ref: Option<String>,
    exact_demand_match: Option<bool>,
    resolution_evidence_ref: Option<String>,
}

fn provider_revision_lock_key(identity: &ProviderMaterializationIdentity) -> i64 {
    let digest = framed_digest(&[
        &identity.provider_ref,
        &identity.dataset_ref,
        &identity.dataset_revision_ref,
        &identity.external_version_ref,
        &identity.citation,
        &identity.jurisdiction_ref,
    ]);
    i64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3],
        digest[4], digest[5], digest[6], digest[7],
    ])
}

fn existing_revision_binding(
    client: &mut Client,
    identity: &ProviderMaterializationIdentity,
) -> Result<Option<RevisionBinding>, ProviderMaterializationError> {
    let row = client.query_opt(
        r#"SELECT r.external_source_revision_ref,r.document_ref,d.canonical_ref,
                  encode(c.content_sha256,'hex'),c.uncompressed_byte_length,
                  r.provider_ref,r.dataset_ref,r.dataset_revision_ref,r.external_version_ref,
                  r.citation,r.source_ref,r.jurisdiction_ref,r.source_url
           FROM corpus.external_source_revision r
           JOIN corpus.document d ON d.document_ref=r.document_ref
           JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
           WHERE r.provider_ref=$1 AND r.dataset_ref=$2 AND r.dataset_revision_ref=$3
             AND r.external_version_ref=$4 AND r.citation=$5 AND r.jurisdiction_ref=$6"#,
        &[
            &identity.provider_ref,
            &identity.dataset_ref,
            &identity.dataset_revision_ref,
            &identity.external_version_ref,
            &identity.citation,
            &identity.jurisdiction_ref,
        ],
    )?;
    Ok(row.map(|row| RevisionBinding {
        external_source_revision_ref: row.get(0),
        document_ref: row.get(1),
        canonical_ref: row.get(2),
        canonical_sha256_hex: row.get(3),
        canonical_byte_length: row.get(4),
        provider_ref: row.get(5),
        dataset_ref: row.get(6),
        dataset_revision_ref: row.get(7),
        external_version_ref: row.get(8),
        citation: row.get(9),
        source_ref: row.get(10),
        jurisdiction_ref: row.get(11),
        source_url: row.get(12),
        source_resolution_ref: None,
        exact_demand_match: None,
        resolution_evidence_ref: None,
    }))
}

fn revision_binding_for_refs(
    client: &mut Client,
    revision_ref: &str,
    resolution_ref: &str,
) -> Result<RevisionBinding, ProviderMaterializationError> {
    let row = client.query_opt(
        r#"SELECT r.external_source_revision_ref,r.document_ref,d.canonical_ref,
                  encode(c.content_sha256,'hex'),c.uncompressed_byte_length,
                  r.provider_ref,r.dataset_ref,r.dataset_revision_ref,r.external_version_ref,
                  r.citation,r.source_ref,r.jurisdiction_ref,r.source_url,
                  s.source_resolution_ref,s.exact_demand_match,s.resolution_evidence_ref
           FROM corpus.external_source_revision r
           JOIN corpus.document d ON d.document_ref=r.document_ref
           JOIN corpus.canonical_content c ON c.canonical_ref=d.canonical_ref
           JOIN evidence.external_source_resolution s
             ON s.external_source_revision_ref=r.external_source_revision_ref
           WHERE r.external_source_revision_ref=$1 AND s.source_resolution_ref=$2"#,
        &[&revision_ref, &resolution_ref],
    )?.ok_or(ProviderMaterializationError::RevisionBindingConflict)?;
    Ok(RevisionBinding {
        external_source_revision_ref: row.get(0),
        document_ref: row.get(1),
        canonical_ref: row.get(2),
        canonical_sha256_hex: row.get(3),
        canonical_byte_length: row.get(4),
        provider_ref: row.get(5),
        dataset_ref: row.get(6),
        dataset_revision_ref: row.get(7),
        external_version_ref: row.get(8),
        citation: row.get(9),
        source_ref: row.get(10),
        jurisdiction_ref: row.get(11),
        source_url: row.get(12),
        source_resolution_ref: Some(row.get(13)),
        exact_demand_match: Some(row.get(14)),
        resolution_evidence_ref: Some(row.get(15)),
    })
}

fn validate_revision_binding(
    identity: &ProviderMaterializationIdentity,
    document: &ResolvedExternalDocument<'_>,
    resolution: &ExactResolutionReceipt<'_>,
    returned_document_ref: &str,
    expected_digest: &str,
    binding: &RevisionBinding,
) -> Result<(), ProviderMaterializationError> {
    let ok = binding.document_ref == returned_document_ref
        && binding.provider_ref == identity.provider_ref
        && binding.dataset_ref == identity.dataset_ref
        && binding.dataset_revision_ref == identity.dataset_revision_ref
        && binding.external_version_ref == identity.external_version_ref
        && binding.citation == identity.citation
        && binding.source_ref == identity.source_ref
        && binding.jurisdiction_ref == identity.jurisdiction_ref
        && binding.source_url.as_deref() == document.source_url
        && binding.canonical_sha256_hex == expected_digest
        && binding.source_resolution_ref.as_deref() == Some(resolution_source_ref(binding).as_str())
        && binding.exact_demand_match == Some(true)
        && binding.resolution_evidence_ref.as_deref() == Some(resolution.resolution_evidence_ref);
    if ok {
        Ok(())
    } else {
        Err(ProviderMaterializationError::RevisionBindingConflict)
    }
}

fn resolution_source_ref(binding: &RevisionBinding) -> String {
    binding.source_resolution_ref.clone().unwrap_or_default()
}

fn rehydrate_matching_canonical_payload(
    client: &mut Client,
    identity: &ProviderMaterializationIdentity,
    canonical_text: &str,
) -> Result<(), ProviderMaterializationError> {
    let digest = canonical_sha256_hex(canonical_text);
    let updated = client.execute(
        r#"UPDATE corpus.canonical_content c
           SET payload=$7, uncompressed_byte_length=$8
           FROM corpus.document d, corpus.external_source_revision r
           WHERE r.document_ref=d.document_ref AND d.canonical_ref=c.canonical_ref
             AND r.provider_ref=$1 AND r.dataset_ref=$2 AND r.dataset_revision_ref=$3
             AND r.external_version_ref=$4 AND r.citation=$5 AND r.jurisdiction_ref=$6
             AND encode(c.content_sha256,'hex')=$9"#,
        &[
            &identity.provider_ref,
            &identity.dataset_ref,
            &identity.dataset_revision_ref,
            &identity.external_version_ref,
            &identity.citation,
            &identity.jurisdiction_ref,
            &canonical_text.as_bytes(),
            &(canonical_text.len() as i64),
            &digest,
        ],
    )?;
    if updated == 0 {
        return Err(ProviderMaterializationError::RematerializationDigestMismatch);
    }
    Ok(())
}

fn framed_digest(parts: &[&str]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    hasher.finalize().into()
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
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

    fn identity() -> ProviderMaterializationIdentity {
        ProviderMaterializationIdentity {
            provider_ref: "provider:oalc-hf".into(),
            dataset_ref: "isaacus/open-australian-legal-corpus".into(),
            dataset_revision_ref: "4f4a1e12d6b73bd77a45b9cc71c4ebd907edb922".into(),
            split_ref: "train".into(),
            external_version_ref: "case:[1992] HCA 23".into(),
            citation: "Mabo v Queensland (No 2) [1992] HCA 23".into(),
            source_ref: "oalc:case:[1992]-HCA-23".into(),
            jurisdiction_ref: "AU".into(),
            source_url: Some("hf://datasets/isaacus/open-australian-legal-corpus".into()),
            acquisition_receipt_ref: "receipt:oalc:fixture".into(),
        }
    }

    #[test]
    fn pin_and_content_both_contribute_to_materialization_ref() {
        let value = identity();
        let a = canonical_provider_materialization_ref(&value, &canonical_sha256_hex("a")).unwrap();
        let b = canonical_provider_materialization_ref(&value, &canonical_sha256_hex("b")).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn moving_revision_names_fail_closed() {
        let mut value = identity();
        value.dataset_revision_ref = "main".into();
        assert!(matches!(
            validate_provider_pin(&value),
            Err(ProviderMaterializationError::MutableProviderRevision(_))
        ));
    }

    #[test]
    fn legacy_revision_lock_key_is_split_independent() {
        let left = identity();
        let mut right = left.clone();
        right.split_ref = "validation".into();
        assert_eq!(provider_revision_lock_key(&left), provider_revision_lock_key(&right));

        right.external_version_ref = "case:[2003] HCA 2".into();
        assert_ne!(provider_revision_lock_key(&left), provider_revision_lock_key(&right));
    }
}
