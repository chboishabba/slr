use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::AcquisitionObligation;

pub const INV_GOVERNANCE_SCHEMA: &str = "itir.gov1.inv-governance.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceAccessState {
    Public,
    Authorized,
    RequiresAuthorization,
    ProhibitedOrUnavailable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyExposureState {
    NoPii,
    ContainsPii,
    Sensitive,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiUseState {
    NotApplicable,
    IntendedUseDeclared,
    RiskReviewRequired,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceEvidenceState {
    pub service_change_state: String,
    pub evidence_state: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquisitionGovernancePacket {
    pub schema: String,
    pub packet_ref: String,
    pub obligation_ref: String,
    pub comparison_ref: String,
    pub source_revision_refs: Vec<String>,
    pub purpose_ref: String,
    pub access_state: GovernanceAccessState,
    pub privacy_state: PrivacyExposureState,
    pub security_refs: Vec<String>,
    pub ai_use_state: AiUseState,
    pub service_evidence_state: ServiceEvidenceState,
    pub control_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub creates_semantic_authority: bool,
    pub creates_access_authority: bool,
    pub creates_priority: bool,
    pub certification_claim: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum GovernanceControlCaseError {
    #[error("governance packet requires stable identity and purpose")]
    MissingIdentity,
    #[error("governed state requires explicit control/evidence/security refs")]
    MissingEvidence,
    #[error("governance packet may not be bound to a promotion-bearing obligation")]
    PromotionBoundary,
}

fn valid(value: &str) -> bool {
    !value.trim().is_empty()
}

fn nonempty_refs(values: &[String]) -> bool {
    !values.is_empty() && values.iter().all(|value| valid(value))
}

pub fn build_inv_governance_packet(
    packet_ref: &str,
    obligation: &AcquisitionObligation,
    purpose_ref: &str,
    access_state: GovernanceAccessState,
    privacy_state: PrivacyExposureState,
    ai_use_state: AiUseState,
    service_evidence_state: ServiceEvidenceState,
    control_refs: Vec<String>,
    evidence_refs: Vec<String>,
    security_refs: Vec<String>,
) -> Result<AcquisitionGovernancePacket, GovernanceControlCaseError> {
    if !valid(packet_ref)
        || !valid(&obligation.obligation_ref)
        || !valid(&obligation.comparison_ref)
        || !valid(purpose_ref)
        || obligation.source_revision_refs.is_empty()
        || obligation.source_revision_refs.iter().any(|value| !valid(value))
        || !valid(&service_evidence_state.service_change_state)
        || !valid(&service_evidence_state.evidence_state)
    {
        return Err(GovernanceControlCaseError::MissingIdentity);
    }
    if !obligation.candidate_only
        || obligation.creates_semantic_authority
        || obligation.claim_truth_promoted
    {
        return Err(GovernanceControlCaseError::PromotionBoundary);
    }
    if !nonempty_refs(&control_refs)
        || !nonempty_refs(&evidence_refs)
        || !nonempty_refs(&security_refs)
    {
        return Err(GovernanceControlCaseError::MissingEvidence);
    }

    Ok(AcquisitionGovernancePacket {
        schema: INV_GOVERNANCE_SCHEMA.into(),
        packet_ref: packet_ref.into(),
        obligation_ref: obligation.obligation_ref.clone(),
        comparison_ref: obligation.comparison_ref.clone(),
        source_revision_refs: obligation.source_revision_refs.clone(),
        purpose_ref: purpose_ref.into(),
        access_state,
        privacy_state,
        security_refs,
        ai_use_state,
        service_evidence_state,
        control_refs,
        evidence_refs,
        creates_semantic_authority: false,
        creates_access_authority: false,
        creates_priority: false,
        certification_claim: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquisition_pareto_frontier, AccessDisposition, AcquisitionObligation,
        AcquisitionRouteCandidate, DuplicateRelation, EvidenceIndependence,
        RecordAvailability,
    };

    fn obligation() -> AcquisitionObligation {
        AcquisitionObligation {
            obligation_ref: "acquisition:1".into(),
            comparison_ref: "comparison:1".into(),
            residual_obligation_ref: "residual:1".into(),
            source_revision_refs: vec!["revision:a".into(), "revision:b".into()],
            target_description: "obtain primary record".into(),
            current_availability: RecordAvailability::NotLocated,
            authority_or_access_constraint_ref: "access:public-records".into(),
            dependency_target_refs: vec!["assessment:case".into()],
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        }
    }

    fn route(id: &str, access: AccessDisposition) -> AcquisitionRouteCandidate {
        AcquisitionRouteCandidate {
            route_ref: id.into(),
            obligation_ref: obligation().obligation_ref,
            route_description: "route".into(),
            source_locator_ref: format!("locator:{id}"),
            access_disposition: access,
            authority_receipt_ref: None,
            provenance_genealogy_ref: format!("genealogy:{id}"),
            independence: EvidenceIndependence::Unknown,
            independence_receipt_ref: None,
            duplicate_relation: DuplicateRelation::Unknown,
            information_gain: 10,
            dependency_closure_impact: 10,
            residual_coverage: 10,
            provenance_novelty: 10,
            acquisition_cost: 10,
            axis_estimation_receipt_ref: "estimate:1".into(),
            candidate_only: true,
            creates_acquisition_authority: false,
        }
    }

    #[test]
    fn governance_packet_does_not_remove_blocked_frontier_route_or_create_priority() {
        let obligation = obligation();
        let routes = vec![route("blocked", AccessDisposition::RequiresAuthorization)];
        let before = acquisition_pareto_frontier(&obligation, &routes).unwrap();
        let packet = build_inv_governance_packet(
            "gov:1",
            &obligation,
            "purpose:investigation",
            GovernanceAccessState::RequiresAuthorization,
            PrivacyExposureState::Unknown,
            AiUseState::NotApplicable,
            ServiceEvidenceState {
                service_change_state: "implemented".into(),
                evidence_state: "compile_checked".into(),
            },
            vec!["control:access".into()],
            vec!["evidence:scope".into()],
            vec!["security:baseline".into()],
        )
        .unwrap();
        let after = acquisition_pareto_frontier(&obligation, &routes).unwrap();
        assert_eq!(before, after);
        assert_eq!(after.frontier_route_refs, vec!["blocked"]);
        assert!(!packet.creates_priority);
        assert!(!packet.creates_access_authority);
        assert!(!packet.creates_semantic_authority);
        assert!(!packet.certification_claim);
    }

    #[test]
    fn governed_packet_requires_evidence_for_claimed_state() {
        let result = build_inv_governance_packet(
            "gov:1",
            &obligation(),
            "purpose:investigation",
            GovernanceAccessState::Authorized,
            PrivacyExposureState::ContainsPii,
            AiUseState::IntendedUseDeclared,
            ServiceEvidenceState {
                service_change_state: "validated".into(),
                evidence_state: "runtime_observed".into(),
            },
            vec!["control:1".into()],
            vec![],
            vec!["security:1".into()],
        );
        assert!(matches!(
            result,
            Err(GovernanceControlCaseError::MissingEvidence)
        ));
    }
}
