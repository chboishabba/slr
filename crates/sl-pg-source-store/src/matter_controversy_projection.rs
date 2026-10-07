//! Read-only persona projections over one shared Matter controversy.
//!
//! These projections reorganise persisted coordinates for different legal
//! operators. They never rewrite epistemic status, choose a winner, assign
//! normative weight, determine credibility, or mutate the canonical Matter.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    validate_matter_controversy_draft, MatterControversyDraft, MatterControversyError,
    MatterDisagreementKind, MatterEpistemicStatus, MatterResponseMode, MatterReverseProofSearch,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientMatterProjection {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub proposition_refs: Vec<String>,
    pub reviewed_proposition_refs: Vec<String>,
    pub candidate_proposition_refs: Vec<String>,
    pub disputed_proposition_refs: Vec<String>,
    pub response_refs: Vec<String>,
    pub unresolved_residual_refs: Vec<String>,
    pub source_refs: Vec<String>,
    pub reviewed_evidence_refs: Vec<String>,
    pub normative_order_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolicitorMatterProjection {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub proposition_refs: Vec<String>,
    pub common_ground_proposition_refs: Vec<String>,
    pub typed_response_refs: Vec<String>,
    pub residual_refs: Vec<String>,
    pub open_obligation_refs: Vec<String>,
    pub source_refs: Vec<String>,
    pub reviewed_evidence_refs: Vec<String>,
    pub normative_order_refs: Vec<String>,
    pub reverse_search: MatterReverseProofSearch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CourtMatterProjection {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub common_ground_proposition_refs: Vec<String>,
    pub disputed_occurrence_refs: Vec<String>,
    pub disputed_characterisation_refs: Vec<String>,
    pub disputed_causation_refs: Vec<String>,
    pub disputed_evidence_refs: Vec<String>,
    pub legal_consequence_refs: Vec<String>,
    pub open_evidentiary_obligation_refs: Vec<String>,
    pub open_legal_obligation_refs: Vec<String>,
    pub source_refs: Vec<String>,
    pub reviewed_evidence_refs: Vec<String>,
    pub normative_order_refs: Vec<String>,
    pub determines_credibility: bool,
    pub determines_ultimate_fact: bool,
    pub assigns_normative_weight: bool,
    pub enters_final_judgment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatterPersonaProjections {
    pub controversy_ref: String,
    pub matter_ref: String,
    pub client: ClientMatterProjection,
    pub solicitor: SolicitorMatterProjection,
    pub court: CourtMatterProjection,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub canonical_world_mutated: bool,
}

#[derive(Debug, Error)]
pub enum MatterPersonaProjectionError {
    #[error(transparent)]
    Controversy(#[from] MatterControversyError),
    #[error("reverse proof projection is owned by another controversy or Matter")]
    ReverseOwnerMismatch,
}

pub fn project_matter_personas(
    controversy: &MatterControversyDraft,
    reverse: &MatterReverseProofSearch,
) -> Result<MatterPersonaProjections, MatterPersonaProjectionError> {
    validate_matter_controversy_draft(controversy)?;
    if reverse.controversy_ref != controversy.controversy_ref
        || reverse.matter_ref != controversy.matter_ref
        || reverse.creates_actual_reopening
        || reverse.creates_access_authority
        || reverse.creates_semantic_authority
        || reverse.claim_truth_promoted
    {
        return Err(MatterPersonaProjectionError::ReverseOwnerMismatch);
    }

    let proposition_refs = sorted(
        controversy
            .propositions
            .iter()
            .map(|p| p.proposition_ref.clone()),
    );
    let reviewed_proposition_refs = sorted(
        controversy
            .propositions
            .iter()
            .filter(|p| p.reviewed_evidence_ref.is_some())
            .map(|p| p.proposition_ref.clone()),
    );
    let candidate_proposition_refs = sorted(
        controversy
            .propositions
            .iter()
            .filter(|p| p.reviewed_evidence_ref.is_none())
            .map(|p| p.proposition_ref.clone()),
    );
    let disputed_proposition_refs = sorted(
        controversy
            .propositions
            .iter()
            .filter(|p| matches!(p.epistemic_status, MatterEpistemicStatus::Disputed | MatterEpistemicStatus::Unresolved))
            .map(|p| p.proposition_ref.clone()),
    );
    let common_ground_proposition_refs = sorted(
        controversy
            .propositions
            .iter()
            .filter(|p| p.epistemic_status == MatterEpistemicStatus::Admitted)
            .map(|p| p.proposition_ref.clone()),
    );
    let source_refs = sorted(controversy.propositions.iter().map(|p| p.source_ref.clone()));
    let reviewed_evidence_refs = sorted(
        controversy
            .propositions
            .iter()
            .filter_map(|p| p.reviewed_evidence_ref.clone()),
    );
    let normative_order_refs = sorted(
        controversy
            .propositions
            .iter()
            .map(|p| p.normative_order_ref.clone()),
    );
    let response_refs = sorted(controversy.responses.iter().map(|r| r.response_ref.clone()));
    let residual_refs = sorted(controversy.residuals.iter().map(|r| r.residual_ref.clone()));

    let disputed_occurrence_refs = sorted(
        controversy
            .responses
            .iter()
            .filter(|response| response.mode == MatterResponseMode::DenyOccurrence)
            .map(|response| response.response_ref.clone())
            .chain(
                controversy
                    .residuals
                    .iter()
                    .filter(|residual| residual.kind == MatterDisagreementKind::Node)
                    .map(|residual| residual.residual_ref.clone()),
            ),
    );
    let disputed_characterisation_refs = residuals_by_kind(
        controversy,
        MatterDisagreementKind::Characterisation,
    );
    let disputed_causation_refs = residuals_by_kind(controversy, MatterDisagreementKind::Causal);
    let disputed_evidence_refs = residuals_by_kind(controversy, MatterDisagreementKind::Evidence);
    let legal_consequence_refs = residuals_by_kind(
        controversy,
        MatterDisagreementKind::LegalConsequence,
    );

    let open_evidentiary_obligation_refs = sorted(
        controversy
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.discharge_ref.is_none()
                    && matches!(
                        obligation.kind,
                        crate::MatterObligationKind::Evidence
                            | crate::MatterObligationKind::Discriminator
                    )
            })
            .map(|obligation| obligation.obligation_ref.clone()),
    );
    let open_legal_obligation_refs = sorted(
        controversy
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.discharge_ref.is_none()
                    && matches!(
                        obligation.kind,
                        crate::MatterObligationKind::Premise
                            | crate::MatterObligationKind::Response
                            | crate::MatterObligationKind::Adjudicative
                    )
            })
            .map(|obligation| obligation.obligation_ref.clone()),
    );

    let client = ClientMatterProjection {
        controversy_ref: controversy.controversy_ref.clone(),
        matter_ref: controversy.matter_ref.clone(),
        proposition_refs: proposition_refs.clone(),
        reviewed_proposition_refs,
        candidate_proposition_refs,
        disputed_proposition_refs,
        response_refs: response_refs.clone(),
        unresolved_residual_refs: residual_refs.clone(),
        source_refs: source_refs.clone(),
        reviewed_evidence_refs: reviewed_evidence_refs.clone(),
        normative_order_refs: normative_order_refs.clone(),
    };
    let solicitor = SolicitorMatterProjection {
        controversy_ref: controversy.controversy_ref.clone(),
        matter_ref: controversy.matter_ref.clone(),
        proposition_refs,
        common_ground_proposition_refs: common_ground_proposition_refs.clone(),
        typed_response_refs: response_refs,
        residual_refs,
        open_obligation_refs: reverse.open_obligation_refs.clone(),
        source_refs: source_refs.clone(),
        reviewed_evidence_refs: reviewed_evidence_refs.clone(),
        normative_order_refs: normative_order_refs.clone(),
        reverse_search: reverse.clone(),
    };
    let court = CourtMatterProjection {
        controversy_ref: controversy.controversy_ref.clone(),
        matter_ref: controversy.matter_ref.clone(),
        common_ground_proposition_refs,
        disputed_occurrence_refs,
        disputed_characterisation_refs,
        disputed_causation_refs,
        disputed_evidence_refs,
        legal_consequence_refs,
        open_evidentiary_obligation_refs,
        open_legal_obligation_refs,
        source_refs,
        reviewed_evidence_refs,
        normative_order_refs,
        determines_credibility: false,
        determines_ultimate_fact: false,
        assigns_normative_weight: false,
        enters_final_judgment: false,
    };

    Ok(MatterPersonaProjections {
        controversy_ref: controversy.controversy_ref.clone(),
        matter_ref: controversy.matter_ref.clone(),
        client,
        solicitor,
        court,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
        canonical_world_mutated: false,
    })
}

fn residuals_by_kind(
    controversy: &MatterControversyDraft,
    kind: MatterDisagreementKind,
) -> Vec<String> {
    sorted(
        controversy
            .residuals
            .iter()
            .filter(move |residual| residual.kind == kind)
            .map(|residual| residual.residual_ref.clone()),
    )
}

fn sorted(values: impl IntoIterator<Item = String>) -> Vec<String> {
    values.into_iter().collect::<BTreeSet<_>>().into_iter().collect()
}
