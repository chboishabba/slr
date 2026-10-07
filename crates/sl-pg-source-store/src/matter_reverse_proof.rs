//! Reverse proof-search projection over one persisted/shared Matter controversy.
//!
//! The projection only exposes already persisted open obligations, residuals,
//! discriminator refs and evidence queries. It does not create an acquisition
//! obligation, perform access, reopen dependencies, or decide relevance/truth.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{validate_matter_controversy_draft, MatterControversyDraft, MatterControversyError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatterProceduralGoal {
    IdentifyCommonGround,
    IsolateResidualControversy,
    DecideEvidenceNeeded,
    PrepareForAdjudication,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterReverseProofSearch {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub goal: MatterProceduralGoal,
    pub open_obligation_refs: Vec<String>,
    pub candidate_residual_refs: Vec<String>,
    pub requested_discriminator_ref: Option<String>,
    pub target_evidence_queries: Vec<String>,
    pub potential_reopening_refs: Vec<String>,
    pub creates_actual_reopening: bool,
    pub creates_access_authority: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum MatterReverseProofError {
    #[error(transparent)]
    Controversy(#[from] MatterControversyError),
}

pub fn project_matter_reverse_proof_search(
    controversy: &MatterControversyDraft,
    goal: MatterProceduralGoal,
) -> Result<MatterReverseProofSearch, MatterReverseProofError> {
    validate_matter_controversy_draft(controversy)?;

    let mut open_obligation_refs = controversy
        .obligations
        .iter()
        .filter(|obligation| obligation.discharge_ref.is_none())
        .map(|obligation| obligation.obligation_ref.clone())
        .collect::<Vec<_>>();
    open_obligation_refs.sort();
    open_obligation_refs.dedup();

    let mut candidate_residual_refs = controversy
        .residuals
        .iter()
        .map(|residual| residual.residual_ref.clone())
        .collect::<Vec<_>>();
    candidate_residual_refs.sort();
    candidate_residual_refs.dedup();

    let requested_discriminator_ref = controversy
        .residuals
        .iter()
        .find(|residual| !residual.requested_discriminator_ref.trim().is_empty())
        .map(|residual| residual.requested_discriminator_ref.clone());

    let mut target_evidence_queries = controversy
        .residuals
        .iter()
        .map(|residual| residual.target_evidence_query.clone())
        .collect::<Vec<_>>();
    target_evidence_queries.sort();
    target_evidence_queries.dedup();

    let potential_reopening_refs = controversy
        .residuals
        .iter()
        .flat_map(|residual| residual.potential_reopening_refs.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Ok(MatterReverseProofSearch {
        controversy_ref: controversy.controversy_ref.clone(),
        matter_ref: controversy.matter_ref.clone(),
        goal,
        open_obligation_refs,
        candidate_residual_refs,
        requested_discriminator_ref,
        target_evidence_queries,
        potential_reopening_refs,
        creates_actual_reopening: false,
        creates_access_authority: false,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
    })
}
