//! Rust-native loading and validation of retained governed-live legal artifacts.
//!
//! This module deliberately performs no network I/O.  It consumes an already
//! persisted acquisition receipt plus the retained source artifact.  The goal is
//! to keep the R6/R7 execution path single-language: Rust owns receipt parsing,
//! fail-closed validation and downstream artifact production; shell may sequence
//! commands but does not carry semantics.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const CULLEN_RESIDUAL_REF: &str = "residual:cullen-positive-operational-act";
pub const CULLEN_PROPOSITION_REF: &str = "prop:cullen-positive-operational-duty";
pub const CULLEN_MNC: &str = "[2026] HCA 19";
pub const CANDIDATE_ONLY_AUTHORITY: &str = "experimental_candidate_only";

#[derive(Debug, Clone, Deserialize)]
pub struct AcquisitionBinding {
    pub residual_ref: String,
    pub proposition_ref: String,
    pub scheduled_producer_ref: String,
    pub hypothesis_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DocumentFetchReceipt {
    pub network_requests: u64,
    pub source_revision_ref: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayReceipt {
    pub network_requests: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanonicalTextReceipt {
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GovernedOfficialJudgmentAcquisitionReceipt {
    pub schema_version: String,
    pub authority: String,
    pub provider: String,
    pub medium_neutral_citation: String,
    pub landing_page_network_requests: u64,
    pub binding: AcquisitionBinding,
    pub document_fetch: DocumentFetchReceipt,
    pub replay_run: ReplayReceipt,
    pub canonical_text: CanonicalTextReceipt,
    pub document_source_identity_ref: Option<String>,
    pub source_identity_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CullenArtifactInputs {
    pub receipt_path: PathBuf,
    pub docx_path: PathBuf,
    pub output_path: PathBuf,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub body_only_canonical_text_sha256: String,
    pub residual_ref: String,
    pub proposition_ref: String,
    pub producer_ref: String,
    pub hypothesis_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveArtifactError {
    MissingReceipt,
    MissingDocx,
    InvalidJson,
    UnexpectedSchema,
    AuthorityEscalation,
    UnexpectedProvider,
    UnexpectedCitation,
    LandingWasNotPersisted,
    UnexpectedDocumentFetchCount,
    ReplayWasNotZeroNetwork,
    UnexpectedResidual,
    UnexpectedProposition,
    MissingDocumentIdentity,
    MissingSourceRevision,
    MissingCanonicalTextHash,
    MissingProducer,
    MissingHypothesis,
}

pub fn load_and_validate_cullen_inputs(
    receipt_path: impl AsRef<Path>,
    docx_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<CullenArtifactInputs, LiveArtifactError> {
    let receipt_path = receipt_path.as_ref().to_path_buf();
    let docx_path = docx_path.as_ref().to_path_buf();
    let output_path = output_path.as_ref().to_path_buf();

    if !receipt_path.is_file() {
        return Err(LiveArtifactError::MissingReceipt);
    }
    if !docx_path.is_file() {
        return Err(LiveArtifactError::MissingDocx);
    }

    let bytes = fs::read(&receipt_path).map_err(|_| LiveArtifactError::MissingReceipt)?;
    let receipt: GovernedOfficialJudgmentAcquisitionReceipt =
        serde_json::from_slice(&bytes).map_err(|_| LiveArtifactError::InvalidJson)?;

    if receipt.schema_version != "sl.governed_official_judgment_acquisition.v0_1" {
        return Err(LiveArtifactError::UnexpectedSchema);
    }
    if receipt.authority != CANDIDATE_ONLY_AUTHORITY {
        return Err(LiveArtifactError::AuthorityEscalation);
    }
    if receipt.provider != "HighCourtAustralia" {
        return Err(LiveArtifactError::UnexpectedProvider);
    }
    if receipt.medium_neutral_citation != CULLEN_MNC {
        return Err(LiveArtifactError::UnexpectedCitation);
    }
    if receipt.landing_page_network_requests != 0 {
        return Err(LiveArtifactError::LandingWasNotPersisted);
    }
    if receipt.document_fetch.network_requests != 1 {
        return Err(LiveArtifactError::UnexpectedDocumentFetchCount);
    }
    if receipt.replay_run.network_requests != 0 {
        return Err(LiveArtifactError::ReplayWasNotZeroNetwork);
    }
    if receipt.binding.residual_ref != CULLEN_RESIDUAL_REF {
        return Err(LiveArtifactError::UnexpectedResidual);
    }
    if receipt.binding.proposition_ref != CULLEN_PROPOSITION_REF {
        return Err(LiveArtifactError::UnexpectedProposition);
    }
    if receipt.binding.scheduled_producer_ref.is_empty() {
        return Err(LiveArtifactError::MissingProducer);
    }
    if receipt.binding.hypothesis_ref.is_empty() {
        return Err(LiveArtifactError::MissingHypothesis);
    }
    if receipt.document_fetch.source_revision_ref.is_empty() {
        return Err(LiveArtifactError::MissingSourceRevision);
    }
    if receipt.canonical_text.sha256.is_empty() {
        return Err(LiveArtifactError::MissingCanonicalTextHash);
    }

    let document_ref = receipt
        .document_source_identity_ref
        .or(receipt.source_identity_ref)
        .filter(|value| !value.is_empty())
        .ok_or(LiveArtifactError::MissingDocumentIdentity)?;

    Ok(CullenArtifactInputs {
        receipt_path,
        docx_path,
        output_path,
        document_ref,
        source_revision_ref: receipt.document_fetch.source_revision_ref,
        body_only_canonical_text_sha256: receipt.canonical_text.sha256,
        residual_ref: receipt.binding.residual_ref,
        proposition_ref: receipt.binding.proposition_ref,
        producer_ref: receipt.binding.scheduled_producer_ref,
        hypothesis_ref: receipt.binding.hypothesis_ref,
    })
}

pub fn cli_paths(default_output_name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let mut args = std::env::args_os().skip(1);
    let base = PathBuf::from("/tmp/sensiblaw-live-legal");
    let receipt = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("governed-official-judgment-acquisition-v01.json"));
    let docx = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join("judgment.docx"));
    let output = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| base.join(default_output_name));
    (receipt, docx, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_pin_cullen_live_residual() {
        assert_eq!(CULLEN_RESIDUAL_REF, "residual:cullen-positive-operational-act");
        assert_eq!(CULLEN_PROPOSITION_REF, "prop:cullen-positive-operational-duty");
        assert_eq!(CULLEN_MNC, "[2026] HCA 19");
        assert_eq!(CANDIDATE_ONLY_AUTHORITY, "experimental_candidate_only");
    }
}
