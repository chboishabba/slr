//! Durable external-system object bindings.
//!
//! The external product remains authoritative. These rows preserve identity,
//! access context and canonical SLR linkage only; they are not a synchronization
//! database and cannot promote legal or semantic meaning.

use postgres::{Client, NoTls};
use serde_json;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    validate_external_object_binding, DatabaseConfig, ExternalLegalSystemError,
    ExternalObjectBinding,
};

pub const EXTERNAL_LEGAL_SYSTEM_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS integration;
CREATE TABLE IF NOT EXISTS integration.external_object_binding (
  binding_ref TEXT PRIMARY KEY,
  system_ref TEXT NOT NULL,
  tenant_ref TEXT NOT NULL,
  object_kind_ref TEXT NOT NULL,
  external_object_ref TEXT NOT NULL,
  packet_sha256 TEXT NOT NULL,
  packet_json TEXT NOT NULL,
  authoritative_externally BOOLEAN NOT NULL CHECK (authoritative_externally),
  copied_as_system_of_record BOOLEAN NOT NULL CHECK (NOT copied_as_system_of_record),
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE(system_ref,tenant_ref,object_kind_ref,external_object_ref)
);
CREATE INDEX IF NOT EXISTS external_binding_matter_idx
  ON integration.external_object_binding ((packet_json::jsonb ->> 'canonical_matter_ref'));
CREATE INDEX IF NOT EXISTS external_binding_source_idx
  ON integration.external_object_binding ((packet_json::jsonb ->> 'canonical_source_revision_ref'));
"#;

#[derive(Debug, Error)]
pub enum ExternalLegalSystemStoreError {
    #[error(transparent)]
    Domain(#[from] ExternalLegalSystemError),
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("persisted external-system binding changed under an immutable identity")]
    ExistingRowConflict,
    #[error("external-system binding not found: {0}")]
    NotFound(String),
}

fn system_ref(value: crate::ExternalLegalSystem) -> &'static str {
    match value {
        crate::ExternalLegalSystem::Actionstep => "actionstep",
        crate::ExternalLegalSystem::NetDocuments => "netdocuments",
    }
}

fn object_kind_ref(value: crate::ExternalObjectKind) -> &'static str {
    match value {
        crate::ExternalObjectKind::Matter => "matter",
        crate::ExternalObjectKind::Contact => "contact",
        crate::ExternalObjectKind::Document => "document",
        crate::ExternalObjectKind::DocumentVersion => "document_version",
        crate::ExternalObjectKind::Folder => "folder",
        crate::ExternalObjectKind::Task => "task",
        crate::ExternalObjectKind::FileNote => "file_note",
    }
}

fn digest(value: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(value.as_bytes()))
}

pub fn external_binding_ref(binding: &ExternalObjectBinding) -> String {
    let mut hasher = Sha256::new();
    for part in [
        system_ref(binding.system),
        binding.tenant_ref.as_str(),
        object_kind_ref(binding.object_kind),
        binding.external_object_ref.as_str(),
    ] {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("external-binding:sha256:{:x}", hasher.finalize())
}

pub fn install_external_legal_system_schema(
    config: &DatabaseConfig,
) -> Result<(), ExternalLegalSystemStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(EXTERNAL_LEGAL_SYSTEM_SCHEMA_SQL)?;
    Ok(())
}

pub fn persist_external_object_binding(
    config: &DatabaseConfig,
    binding: &ExternalObjectBinding,
) -> Result<String, ExternalLegalSystemStoreError> {
    validate_external_object_binding(binding)?;
    install_external_legal_system_schema(config)?;
    let binding_ref = external_binding_ref(binding);
    let body = serde_json::to_string(binding)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.execute(
        r#"INSERT INTO integration.external_object_binding
           (binding_ref,system_ref,tenant_ref,object_kind_ref,external_object_ref,
            packet_sha256,packet_json,authoritative_externally,copied_as_system_of_record,
            candidate_only,creates_semantic_authority,creates_legal_authority,
            applicability_promoted,claim_truth_promoted)
           VALUES($1,$2,$3,$4,$5,$6,$7,TRUE,FALSE,TRUE,FALSE,FALSE,FALSE,FALSE)
           ON CONFLICT(binding_ref) DO NOTHING"#,
        &[
            &binding_ref,
            &system_ref(binding.system),
            &binding.tenant_ref,
            &object_kind_ref(binding.object_kind),
            &binding.external_object_ref,
            &digest(&body),
            &body,
        ],
    )?;
    let reopened = load_external_object_binding(config, &binding_ref)?;
    if reopened != *binding {
        return Err(ExternalLegalSystemStoreError::ExistingRowConflict);
    }
    Ok(binding_ref)
}

pub fn load_external_object_binding(
    config: &DatabaseConfig,
    binding_ref: &str,
) -> Result<ExternalObjectBinding, ExternalLegalSystemStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT packet_sha256,packet_json,authoritative_externally,copied_as_system_of_record,
                      candidate_only,creates_semantic_authority,creates_legal_authority,
                      applicability_promoted,claim_truth_promoted
               FROM integration.external_object_binding WHERE binding_ref=$1"#,
            &[&binding_ref],
        )?
        .ok_or_else(|| ExternalLegalSystemStoreError::NotFound(binding_ref.to_owned()))?;
    let body: String = row.get(1);
    if digest(&body) != row.get::<_, String>(0)
        || !row.get::<_, bool>(2)
        || row.get::<_, bool>(3)
        || !row.get::<_, bool>(4)
        || row.get::<_, bool>(5)
        || row.get::<_, bool>(6)
        || row.get::<_, bool>(7)
        || row.get::<_, bool>(8)
    {
        return Err(ExternalLegalSystemStoreError::ExistingRowConflict);
    }
    let binding: ExternalObjectBinding = serde_json::from_str(&body)?;
    validate_external_object_binding(&binding)?;
    Ok(binding)
}
