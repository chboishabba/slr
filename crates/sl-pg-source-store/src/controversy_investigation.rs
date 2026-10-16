//! REAL-MATTER controversy -> investigation -> reviewed-evidence reopening.
//!
//! This is deliberately a bridge over the existing controversy, acquisition,
//! review and selective-reopening owners. Acquisition may locate bytes; only an
//! explicit reviewed-evidence coordinate can supply a legal proof obligation,
//! and supplying evidence never adjudicates the residual automatically.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    AcquisitionPriorityReceipt, AcquisitionRouteCandidate, AcquisitionUpdate,
    LegalControversyMatter, LegalControversyResidual, LegalProofObligation,
    LegalPropositionFibre, RecordAvailability, SelectiveReopeningReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControversyAcquisitionObligation {
    pub obligation_ref: String,
    pub matter_ref: String,
    pub controversy_ref: String,
    pub legal_obligation_ref: String,
    pub residual_ref: String,
    pub source_revision_refs: Vec<String>,
    pub target_description: String,
    pub requested_discriminator: String,
    pub target_evidence_query: String,
    pub current_availability: RecordAvailability,
    pub authority_or_access_constraint_ref: String,
    pub dependency_target_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControversyReviewedEvidenceDischarge {
    pub reviewed_evidence_ref: String,
    pub requirement_ref: String,
    pub source_revision_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControversyEvidenceStage {
    pub stage_ref: String,
    pub matter_ref: String,
    pub controversy_ref: String,
    pub legal_obligation_ref: String,
    pub residual_ref: String,
    pub acquisition_obligation_ref: String,
    pub acquisition_receipt_ref: String,
    pub acquired_source_revision_ref: String,
    pub reviewed_evidence_ref: String,
    pub reopening_dependency_graph_ref: String,
    pub directly_reopened_refs: Vec<String>,
    pub transitively_reopened_refs: Vec<String>,
    pub unrelated_refs_not_reopened: Vec<String>,
    pub evidence_supplied: bool,
    pub controversy_reopened: bool,
    pub requires_residual_reassessment: bool,
    pub residual_resolved: bool,
    pub determines_credibility: bool,
    pub determines_ultimate_fact: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ControversyInvestigationError {
    #[error("invalid or promotion-bearing controversy investigation identity")]
    InvalidIdentity,
    #[error("legal proof obligation is not an open discriminator owned by the declared residual")]
    WrongLegalObligation,
    #[error("controversy residual has no discriminator/query target")]
    MissingTarget,
    #[error("controversy acquisition route is not scoped to the requested obligation")]
    WrongRouteObligation,
    #[error("authorized acquisition route requires an authority receipt")]
    MissingAuthorityReceipt,
    #[error("known independence/dependence requires an independence receipt")]
    MissingIndependenceReceipt,
    #[error("acquisition update does not supply the expected acquired source")]
    WrongAcquisitionUpdate,
    #[error("reviewed evidence does not bind the exact acquired source")]
    ReviewedEvidenceSourceMismatch,
    #[error("reviewed evidence requirement does not bind the exact legal proof obligation")]
    ReviewedEvidenceRequirementMismatch,
    #[error("selective reopening does not reach the legal proof obligation and controversy")]
    ReopeningMissesControversy,
}

fn valid(value: &str) -> bool {
    !value.trim().is_empty()
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{prefix}:sha256:{:x}", hasher.finalize())
}

pub fn controversy_obligation_from_parts(
    matter: &LegalControversyMatter,
    residual: &LegalControversyResidual,
    legal_obligation: &LegalProofObligation,
    fibres: &[LegalPropositionFibre],
    mut source_revision_refs: Vec<String>,
    authority_or_access_constraint_ref: &str,
    mut dependency_target_refs: Vec<String>,
) -> Result<ControversyAcquisitionObligation, ControversyInvestigationError> {
    use crate::ObligationKind;

    if !valid(&matter.matter_ref)
        || !valid(&matter.controversy_ref)
        || !valid(&residual.residual_ref)
        || !valid(&legal_obligation.obligation_ref)
        || !valid(authority_or_access_constraint_ref)
        || !matter.candidate_only
        || matter.creates_semantic_authority
        || matter.creates_legal_authority
        || matter.applicability_promoted
        || matter.claim_truth_promoted
        || !residual.candidate_only
        || residual.creates_semantic_authority
        || residual.creates_legal_authority
        || residual.applicability_promoted
        || residual.claim_truth_promoted
    {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }
    if residual.controversy_ref != matter.controversy_ref
        || legal_obligation.controversy_ref != matter.controversy_ref
        || legal_obligation.kind != ObligationKind::Discriminator
        || legal_obligation.required_by != residual.residual_ref
        || !legal_obligation.is_open
        || legal_obligation.discharge_reference.is_some()
        || !legal_obligation.candidate_only
        || legal_obligation.creates_semantic_authority
        || legal_obligation.creates_legal_authority
        || legal_obligation.applicability_promoted
        || legal_obligation.claim_truth_promoted
        || !matter.obligation_refs.contains(&legal_obligation.obligation_ref)
        || !matter.residual_refs.contains(&residual.residual_ref)
    {
        return Err(ControversyInvestigationError::WrongLegalObligation);
    }
    if !fibres.iter().any(|f| f.fibre_ref == residual.applicant_fibre_ref)
        || !fibres.iter().any(|f| f.fibre_ref == residual.respondent_fibre_ref)
        || fibres.iter().any(|f| {
            !matter.proposition_fibre_refs.contains(&f.fibre_ref)
                || !f.candidate_only
                || f.creates_semantic_authority
                || f.creates_legal_authority
                || f.applicability_promoted
                || f.claim_truth_promoted
        })
    {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }
    let requested_discriminator = residual
        .requested_discriminator
        .clone()
        .filter(|value| valid(value))
        .ok_or(ControversyInvestigationError::MissingTarget)?;
    let target_evidence_query = residual
        .target_evidence_query
        .clone()
        .filter(|value| valid(value))
        .ok_or(ControversyInvestigationError::MissingTarget)?;

    source_revision_refs.sort();
    source_revision_refs.dedup();
    // Two adversarial proposition fibres may legitimately be grounded in the
    // same source revision (for example, competing constructions of one
    // judgment or instrument). Require source ancestry, not artificial source
    // multiplicity.
    if source_revision_refs.is_empty() || source_revision_refs.iter().any(|r| !valid(r)) {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }
    dependency_target_refs.extend([
        legal_obligation.obligation_ref.clone(),
        residual.residual_ref.clone(),
        matter.controversy_ref.clone(),
    ]);
    dependency_target_refs.sort();
    dependency_target_refs.dedup();
    if dependency_target_refs.iter().any(|r| !valid(r)) {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }

    let obligation_ref = stable_ref(
        "controversy-acquisition",
        &[
            &matter.matter_ref,
            &matter.controversy_ref,
            &legal_obligation.obligation_ref,
            &residual.residual_ref,
            &requested_discriminator,
            &target_evidence_query,
        ],
    );
    Ok(ControversyAcquisitionObligation {
        obligation_ref,
        matter_ref: matter.matter_ref.clone(),
        controversy_ref: matter.controversy_ref.clone(),
        legal_obligation_ref: legal_obligation.obligation_ref.clone(),
        residual_ref: residual.residual_ref.clone(),
        source_revision_refs,
        target_description: target_evidence_query.clone(),
        requested_discriminator,
        target_evidence_query,
        current_availability: RecordAvailability::NotLocated,
        authority_or_access_constraint_ref: authority_or_access_constraint_ref.into(),
        dependency_target_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        claim_truth_promoted: false,
    })
}

fn validate_route(
    obligation: &ControversyAcquisitionObligation,
    route: &AcquisitionRouteCandidate,
) -> Result<(), ControversyInvestigationError> {
    use crate::{AccessDisposition, EvidenceIndependence};
    if !obligation.candidate_only
        || obligation.creates_semantic_authority
        || obligation.creates_legal_authority
        || obligation.claim_truth_promoted
        || obligation.current_availability != RecordAvailability::NotLocated
        || !route.candidate_only
        || route.creates_acquisition_authority
        || !valid(&route.route_ref)
        || !valid(&route.route_description)
        || !valid(&route.source_locator_ref)
        || !valid(&route.provenance_genealogy_ref)
        || !valid(&route.axis_estimation_receipt_ref)
    {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }
    if route.obligation_ref != obligation.obligation_ref {
        return Err(ControversyInvestigationError::WrongRouteObligation);
    }
    if route.access_disposition == AccessDisposition::Authorized
        && route.authority_receipt_ref.as_deref().is_none_or(|s| !valid(s))
    {
        return Err(ControversyInvestigationError::MissingAuthorityReceipt);
    }
    if route.independence != EvidenceIndependence::Unknown
        && route.independence_receipt_ref.as_deref().is_none_or(|s| !valid(s))
    {
        return Err(ControversyInvestigationError::MissingIndependenceReceipt);
    }
    Ok(())
}

/// Canonical persistence order for controversy-owned acquisition routes.
///
/// Route order is presentation/discovery state, not semantic state. Exact
/// replay therefore sorts by durable `route_ref` and rejects duplicate durable
/// identities rather than making replay depend on caller order.
pub fn canonicalize_controversy_acquisition_routes(
    routes: &[AcquisitionRouteCandidate],
) -> Result<Vec<AcquisitionRouteCandidate>, ControversyInvestigationError> {
    let mut canonical = routes.to_vec();
    canonical.sort_by(|left, right| left.route_ref.cmp(&right.route_ref));
    if canonical
        .windows(2)
        .any(|pair| pair[0].route_ref == pair[1].route_ref)
    {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }
    Ok(canonical)
}

/// Use the same direct mixed-orientation Pareto relation as REL/INV without
/// pretending a legal controversy obligation is a REL comparison residual.
pub fn controversy_acquisition_pareto_frontier(
    obligation: &ControversyAcquisitionObligation,
    routes: &[AcquisitionRouteCandidate],
) -> Result<AcquisitionPriorityReceipt, ControversyInvestigationError> {
    for route in routes {
        validate_route(obligation, route)?;
    }
    let mut frontier = Vec::new();
    for (index, candidate) in routes.iter().enumerate() {
        let dominated = routes.iter().enumerate().any(|(other_index, other)| {
            index != other_index
                && crate::investigation_acquisition::dominance_witness(other, candidate).is_some()
        });
        if !dominated {
            frontier.push(candidate);
        }
    }
    frontier.sort_by(|left, right| left.route_ref.cmp(&right.route_ref));
    Ok(AcquisitionPriorityReceipt {
        obligation_ref: obligation.obligation_ref.clone(),
        frontier_route_refs: frontier.iter().map(|r| r.route_ref.clone()).collect(),
        executable_frontier_route_refs: frontier
            .iter()
            .filter(|r| r.access_disposition.executable())
            .map(|r| r.route_ref.clone())
            .collect(),
        blocked_frontier_route_refs: frontier
            .iter()
            .filter(|r| !r.access_disposition.executable())
            .map(|r| r.route_ref.clone())
            .collect(),
        scalar_score_used: false,
        creates_semantic_authority: false,
        creates_screening_or_acquisition_decision: false,
    })
}

pub fn validate_controversy_acquisition_update(
    obligation: &ControversyAcquisitionObligation,
    update: &AcquisitionUpdate,
) -> Result<(), ControversyInvestigationError> {
    if update.obligation_ref != obligation.obligation_ref
        || update.before != RecordAvailability::NotLocated
        || update.after != RecordAvailability::Present
        || update.acquired_source_revision_ref.as_deref().is_none_or(|s| !valid(s))
        || !valid(&update.acquisition_receipt_ref)
    {
        return Err(ControversyInvestigationError::WrongAcquisitionUpdate);
    }
    Ok(())
}

pub fn build_controversy_evidence_stage(
    obligation: &ControversyAcquisitionObligation,
    update: &AcquisitionUpdate,
    reviewed: &ControversyReviewedEvidenceDischarge,
    reopening: &SelectiveReopeningReceipt,
) -> Result<ControversyEvidenceStage, ControversyInvestigationError> {
    validate_controversy_acquisition_update(obligation, update)?;
    let acquired_source = update
        .acquired_source_revision_ref
        .as_ref()
        .ok_or(ControversyInvestigationError::WrongAcquisitionUpdate)?;
    if reviewed.source_revision_ref != *acquired_source || reopening.changed_source_revision_ref != *acquired_source {
        return Err(ControversyInvestigationError::ReviewedEvidenceSourceMismatch);
    }
    if reviewed.requirement_ref != obligation.legal_obligation_ref {
        return Err(ControversyInvestigationError::ReviewedEvidenceRequirementMismatch);
    }
    if !valid(&reviewed.reviewed_evidence_ref)
        || reopening.creates_semantic_authority
        || !valid(&reopening.dependency_graph_ref)
    {
        return Err(ControversyInvestigationError::InvalidIdentity);
    }
    let affected = reopening
        .directly_affected_refs
        .iter()
        .chain(reopening.transitively_affected_refs.iter())
        .collect::<Vec<_>>();
    if !affected.iter().any(|r| *r == &obligation.legal_obligation_ref)
        || !affected.iter().any(|r| *r == &obligation.controversy_ref)
    {
        return Err(ControversyInvestigationError::ReopeningMissesControversy);
    }

    let stage_ref = stable_ref(
        "controversy-evidence-stage",
        &[
            &obligation.matter_ref,
            &obligation.controversy_ref,
            &obligation.legal_obligation_ref,
            &update.acquisition_receipt_ref,
            &reviewed.reviewed_evidence_ref,
        ],
    );
    Ok(ControversyEvidenceStage {
        stage_ref,
        matter_ref: obligation.matter_ref.clone(),
        controversy_ref: obligation.controversy_ref.clone(),
        legal_obligation_ref: obligation.legal_obligation_ref.clone(),
        residual_ref: obligation.residual_ref.clone(),
        acquisition_obligation_ref: obligation.obligation_ref.clone(),
        acquisition_receipt_ref: update.acquisition_receipt_ref.clone(),
        acquired_source_revision_ref: acquired_source.clone(),
        reviewed_evidence_ref: reviewed.reviewed_evidence_ref.clone(),
        reopening_dependency_graph_ref: reopening.dependency_graph_ref.clone(),
        directly_reopened_refs: reopening.directly_affected_refs.clone(),
        transitively_reopened_refs: reopening.transitively_affected_refs.clone(),
        unrelated_refs_not_reopened: reopening.unrelated_refs_not_reopened.clone(),
        evidence_supplied: true,
        controversy_reopened: true,
        requires_residual_reassessment: true,
        residual_resolved: false,
        determines_credibility: false,
        determines_ultimate_fact: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
