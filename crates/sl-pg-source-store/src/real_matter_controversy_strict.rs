//! Replay-strict public materialiser for REAL-MATTER controversy.
//!
//! The low-level constructor is idempotent and may encounter already-existing
//! rows. This public wrapper reopens every controversy child after construction
//! and proves the persisted meaning still equals the caller's reviewed/typed
//! request before returning a receipt.

use thiserror::Error;

use crate::{
    load_legal_controversy_matter, load_legal_controversy_residual,
    load_legal_proof_obligation, load_reverse_legal_proof_search,
    load_typed_response_edge, DatabaseConfig, LegalControversyStoreError,
    ObligationKind,
};
use super::real_matter_controversy as raw;

#[derive(Debug, Error)]
pub enum RealMatterControversyMaterializationError {
    #[error(transparent)]
    Construction(#[from] raw::RealMatterControversyError),
    #[error(transparent)]
    Store(#[from] LegalControversyStoreError),
    #[error("persisted controversy meaning differs from the requested reviewed/typed coordinates")]
    PersistedMeaningMismatch,
}

pub fn materialize_real_matter_controversy(
    config: &DatabaseConfig,
    draft: &raw::RealMatterControversyDraft,
) -> Result<raw::RealMatterControversyReceipt, RealMatterControversyMaterializationError> {
    let receipt = raw::materialize_real_matter_controversy(config, draft)?;

    let matter = load_legal_controversy_matter(config, &receipt.controversy_ref)?;
    if matter.matter_ref != draft.matter_ref
        || !matter.proposition_fibre_refs.contains(&receipt.applicant_fibre_ref)
        || !matter.proposition_fibre_refs.contains(&receipt.respondent_fibre_ref)
        || !matter.response_refs.contains(&receipt.response_ref)
        || !matter.residual_refs.contains(&receipt.residual_ref)
        || !matter.obligation_refs.contains(&receipt.obligation_ref)
    {
        return Err(RealMatterControversyMaterializationError::PersistedMeaningMismatch);
    }

    let response = load_typed_response_edge(config, &receipt.response_ref)?;
    if response.controversy_ref != receipt.controversy_ref
        || response.target_fibre_ref != receipt.applicant_fibre_ref
        || response.response_fibre_ref != receipt.respondent_fibre_ref
        || response.mode != draft.response_mode
        || response.response_reference != draft.respondent_reviewed_evidence_ref
        || !response.candidate_only
        || response.creates_semantic_authority
        || response.creates_legal_authority
        || response.applicability_promoted
        || response.claim_truth_promoted
    {
        return Err(RealMatterControversyMaterializationError::PersistedMeaningMismatch);
    }

    let residual = load_legal_controversy_residual(config, &receipt.residual_ref)?;
    if residual.controversy_ref != receipt.controversy_ref
        || residual.kind != draft.disagreement_kind
        || residual.applicant_fibre_ref != receipt.applicant_fibre_ref
        || residual.respondent_fibre_ref != receipt.respondent_fibre_ref
        || residual.unresolved_question != draft.unresolved_question
        || residual.requested_discriminator.as_deref() != Some(draft.requested_discriminator.as_str())
        || residual.target_evidence_query.as_deref() != Some(draft.target_evidence_query.as_str())
        || !residual.candidate_only
        || residual.creates_semantic_authority
        || residual.creates_legal_authority
        || residual.applicability_promoted
        || residual.claim_truth_promoted
    {
        return Err(RealMatterControversyMaterializationError::PersistedMeaningMismatch);
    }

    let obligation = load_legal_proof_obligation(config, &receipt.obligation_ref)?;
    if obligation.controversy_ref != receipt.controversy_ref
        || obligation.proposition_fibre_ref != receipt.applicant_fibre_ref
        || obligation.kind != ObligationKind::Discriminator
        || obligation.required_by != receipt.residual_ref
        || !obligation.is_open
        || obligation.discharge_reference.is_some()
        || !obligation.candidate_only
        || obligation.creates_semantic_authority
        || obligation.creates_legal_authority
        || obligation.applicability_promoted
        || obligation.claim_truth_promoted
    {
        return Err(RealMatterControversyMaterializationError::PersistedMeaningMismatch);
    }

    let reverse = load_reverse_legal_proof_search(config, &receipt.reverse_ref)?;
    if reverse.controversy_ref != receipt.controversy_ref
        || reverse.goal != draft.procedural_goal
        || !reverse.open_obligation_refs.contains(&receipt.obligation_ref)
        || !reverse.candidate_residual_refs.contains(&receipt.residual_ref)
        || reverse.requested_discriminator != draft.requested_discriminator
        || reverse.target_evidence_query != draft.target_evidence_query
        || !reverse.candidate_only
        || reverse.creates_semantic_authority
        || reverse.creates_legal_authority
        || reverse.applicability_promoted
        || reverse.claim_truth_promoted
    {
        return Err(RealMatterControversyMaterializationError::PersistedMeaningMismatch);
    }

    Ok(receipt)
}
