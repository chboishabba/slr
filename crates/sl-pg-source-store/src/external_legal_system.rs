//! External legal-system bindings for practice/document systems.
//!
//! These adapters preserve the external product as the system of record. They
//! bind external identities into SensibLaw's existing Matter/source substrate;
//! they do not copy operational authority, legal authority, or claim truth.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::provider_materialization::{
    validate_provider_pin, ProviderMaterializationError, ProviderMaterializationIdentity,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalLegalSystem {
    Actionstep,
    NetDocuments,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalObjectKind {
    Matter,
    Contact,
    Document,
    DocumentVersion,
    Folder,
    Task,
    FileNote,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalObjectBinding {
    pub system: ExternalLegalSystem,
    pub tenant_ref: String,
    pub object_kind: ExternalObjectKind,
    pub external_object_ref: String,
    /// Provider-side observation/version coordinate. For mutable operational
    /// objects this is an observation receipt, not a claim of immutable state.
    pub external_revision_ref: Option<String>,
    /// Receipt supplied by the transport/provider adapter for the external
    /// identity semantics used by this binding. For document versions this is
    /// the explicit payment that the selected version coordinate is immutable;
    /// the semantic layer does not infer immutability from string shape alone.
    #[serde(default)]
    pub external_identity_receipt_ref: Option<String>,
    pub parent_external_ref: Option<String>,
    pub canonical_matter_ref: Option<String>,
    pub canonical_source_revision_ref: Option<String>,
    /// Principal whose provider-side permissions governed this observation.
    pub principal_ref: Option<String>,
    pub access_context_ref: String,
    pub acquisition_receipt_ref: String,
    pub authoritative_externally: bool,
    pub copied_as_system_of_record: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionstepMatterBindingInput {
    pub tenant_ref: String,
    pub action_id: String,
    pub observation_ref: String,
    pub matter_ref: String,
    pub user_ref: String,
    pub access_context_ref: String,
    pub acquisition_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionstepObjectBindingInput {
    pub tenant_ref: String,
    pub object_kind: ExternalObjectKind,
    pub object_id: String,
    pub observation_ref: String,
    pub action_id: String,
    pub matter_ref: String,
    pub user_ref: String,
    pub access_context_ref: String,
    pub acquisition_receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetDocumentsVersionInput {
    pub repository_ref: String,
    pub document_id: String,
    pub version_id: String,
    /// Provider/transport receipt that establishes that `version_id` denotes a
    /// fixed version coordinate rather than a mutable alias. This layer does
    /// not itself call or interpret a NetDocuments transport API.
    pub version_identity_receipt_ref: String,
    pub citation: String,
    pub jurisdiction_ref: String,
    pub source_url: Option<String>,
    pub access_context_ref: String,
    pub acquisition_receipt_ref: String,
}

#[derive(Debug, Error)]
pub enum ExternalLegalSystemError {
    #[error("required external legal-system coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("external object binding attempts to promote authority or replace the provider as system of record")]
    AuthorityPromotion,
    #[error("Actionstep child binding must name a child object kind")]
    InvalidActionstepObjectKind,
    #[error("NetDocuments version identity must be immutable")]
    MutableNetDocumentsVersion,
    #[error("NetDocuments immutable version identity requires an explicit provider/transport receipt")]
    MissingNetDocumentsVersionIdentityReceipt,
    #[error(transparent)]
    Provider(#[from] ProviderMaterializationError),
}

fn required(name: &'static str, value: &str) -> Result<(), ExternalLegalSystemError> {
    if value.trim().is_empty() {
        Err(ExternalLegalSystemError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

pub fn actionstep_matter_binding(
    input: &ActionstepMatterBindingInput,
) -> Result<ExternalObjectBinding, ExternalLegalSystemError> {
    for (name, value) in [
        ("tenant_ref", input.tenant_ref.as_str()),
        ("action_id", input.action_id.as_str()),
        ("observation_ref", input.observation_ref.as_str()),
        ("matter_ref", input.matter_ref.as_str()),
        ("user_ref", input.user_ref.as_str()),
        ("access_context_ref", input.access_context_ref.as_str()),
        (
            "acquisition_receipt_ref",
            input.acquisition_receipt_ref.as_str(),
        ),
    ] {
        required(name, value)?;
    }
    Ok(ExternalObjectBinding {
        system: ExternalLegalSystem::Actionstep,
        tenant_ref: input.tenant_ref.clone(),
        object_kind: ExternalObjectKind::Matter,
        external_object_ref: input.action_id.clone(),
        external_revision_ref: Some(input.observation_ref.clone()),
        external_identity_receipt_ref: Some(input.observation_ref.clone()),
        parent_external_ref: None,
        canonical_matter_ref: Some(input.matter_ref.clone()),
        canonical_source_revision_ref: None,
        principal_ref: Some(input.user_ref.clone()),
        access_context_ref: input.access_context_ref.clone(),
        acquisition_receipt_ref: input.acquisition_receipt_ref.clone(),
        authoritative_externally: true,
        copied_as_system_of_record: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

/// Bind Actionstep child objects to the same canonical Matter while retaining
/// the provider action as their operational parent. The enum is intentionally
/// provider-neutral; a transport adapter may map provider-native names such as
/// participants/contacts into the declared object kind it has actually observed.
pub fn actionstep_object_binding(
    input: &ActionstepObjectBindingInput,
) -> Result<ExternalObjectBinding, ExternalLegalSystemError> {
    if input.object_kind == ExternalObjectKind::Matter
        || input.object_kind == ExternalObjectKind::DocumentVersion
    {
        return Err(ExternalLegalSystemError::InvalidActionstepObjectKind);
    }
    for (name, value) in [
        ("tenant_ref", input.tenant_ref.as_str()),
        ("object_id", input.object_id.as_str()),
        ("observation_ref", input.observation_ref.as_str()),
        ("action_id", input.action_id.as_str()),
        ("matter_ref", input.matter_ref.as_str()),
        ("user_ref", input.user_ref.as_str()),
        ("access_context_ref", input.access_context_ref.as_str()),
        (
            "acquisition_receipt_ref",
            input.acquisition_receipt_ref.as_str(),
        ),
    ] {
        required(name, value)?;
    }
    Ok(ExternalObjectBinding {
        system: ExternalLegalSystem::Actionstep,
        tenant_ref: input.tenant_ref.clone(),
        object_kind: input.object_kind,
        external_object_ref: input.object_id.clone(),
        external_revision_ref: Some(input.observation_ref.clone()),
        external_identity_receipt_ref: Some(input.observation_ref.clone()),
        parent_external_ref: Some(input.action_id.clone()),
        canonical_matter_ref: Some(input.matter_ref.clone()),
        canonical_source_revision_ref: None,
        principal_ref: Some(input.user_ref.clone()),
        access_context_ref: input.access_context_ref.clone(),
        acquisition_receipt_ref: input.acquisition_receipt_ref.clone(),
        authoritative_externally: true,
        copied_as_system_of_record: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

/// Translate one provider-confirmed immutable NetDocuments document version
/// into the generic provider-materialisation identity already owned by SLR.
/// Transport/auth and document retrieval remain outside this semantic layer.
pub fn netdocuments_provider_identity(
    input: &NetDocumentsVersionInput,
) -> Result<ProviderMaterializationIdentity, ExternalLegalSystemError> {
    for (name, value) in [
        ("repository_ref", input.repository_ref.as_str()),
        ("document_id", input.document_id.as_str()),
        ("version_id", input.version_id.as_str()),
        ("citation", input.citation.as_str()),
        ("jurisdiction_ref", input.jurisdiction_ref.as_str()),
        ("access_context_ref", input.access_context_ref.as_str()),
        (
            "acquisition_receipt_ref",
            input.acquisition_receipt_ref.as_str(),
        ),
    ] {
        required(name, value)?;
    }
    if input.version_identity_receipt_ref.trim().is_empty() {
        return Err(ExternalLegalSystemError::MissingNetDocumentsVersionIdentityReceipt);
    }
    let normalized = input.version_id.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "main" | "master" | "latest" | "head")
        || normalized.starts_with("refs/heads/")
    {
        return Err(ExternalLegalSystemError::MutableNetDocumentsVersion);
    }
    let identity = ProviderMaterializationIdentity {
        provider_ref: "netdocuments".into(),
        dataset_ref: input.repository_ref.clone(),
        dataset_revision_ref: input.version_id.clone(),
        split_ref: "document_version".into(),
        external_version_ref: format!("{}:{}", input.document_id, input.version_id),
        citation: input.citation.clone(),
        source_ref: format!(
            "netdocuments:{}:{}",
            input.repository_ref, input.document_id
        ),
        jurisdiction_ref: input.jurisdiction_ref.clone(),
        source_url: input.source_url.clone(),
        acquisition_receipt_ref: input.acquisition_receipt_ref.clone(),
    };
    validate_provider_pin(&identity)?;
    Ok(identity)
}

pub fn netdocuments_version_binding(
    input: &NetDocumentsVersionInput,
    canonical_source_revision_ref: &str,
) -> Result<ExternalObjectBinding, ExternalLegalSystemError> {
    let _ = netdocuments_provider_identity(input)?;
    required("canonical_source_revision_ref", canonical_source_revision_ref)?;
    Ok(ExternalObjectBinding {
        system: ExternalLegalSystem::NetDocuments,
        tenant_ref: input.repository_ref.clone(),
        object_kind: ExternalObjectKind::DocumentVersion,
        external_object_ref: format!("{}:{}", input.document_id, input.version_id),
        external_revision_ref: Some(input.version_id.clone()),
        external_identity_receipt_ref: Some(input.version_identity_receipt_ref.clone()),
        parent_external_ref: Some(input.document_id.clone()),
        canonical_matter_ref: None,
        canonical_source_revision_ref: Some(canonical_source_revision_ref.into()),
        principal_ref: None,
        access_context_ref: input.access_context_ref.clone(),
        acquisition_receipt_ref: input.acquisition_receipt_ref.clone(),
        authoritative_externally: true,
        copied_as_system_of_record: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

pub fn validate_external_object_binding(
    binding: &ExternalObjectBinding,
) -> Result<(), ExternalLegalSystemError> {
    for (name, value) in [
        ("tenant_ref", binding.tenant_ref.as_str()),
        ("external_object_ref", binding.external_object_ref.as_str()),
        ("access_context_ref", binding.access_context_ref.as_str()),
        (
            "acquisition_receipt_ref",
            binding.acquisition_receipt_ref.as_str(),
        ),
    ] {
        required(name, value)?;
    }
    if !binding.authoritative_externally
        || binding.copied_as_system_of_record
        || !binding.candidate_only
        || binding.creates_semantic_authority
        || binding.creates_legal_authority
        || binding.applicability_promoted
        || binding.claim_truth_promoted
    {
        return Err(ExternalLegalSystemError::AuthorityPromotion);
    }
    if binding.object_kind == ExternalObjectKind::Matter
        && binding
            .canonical_matter_ref
            .as_deref()
            .map_or(true, |s| s.trim().is_empty())
    {
        return Err(ExternalLegalSystemError::EmptyCoordinate(
            "canonical_matter_ref",
        ));
    }
    if binding.object_kind == ExternalObjectKind::DocumentVersion {
        if binding
            .canonical_source_revision_ref
            .as_deref()
            .map_or(true, |s| s.trim().is_empty())
        {
            return Err(ExternalLegalSystemError::EmptyCoordinate(
                "canonical_source_revision_ref",
            ));
        }
        if binding
            .external_identity_receipt_ref
            .as_deref()
            .map_or(true, |s| s.trim().is_empty())
        {
            return Err(ExternalLegalSystemError::MissingNetDocumentsVersionIdentityReceipt);
        }
    }
    Ok(())
}
