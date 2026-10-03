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
            "gov:1", &obligation, "purpose:investigation",
            GovernanceAccessState::RequiresAuthorization,
            PrivacyExposureState::Unknown,
            AiUseState::NotApplicable,
            ServiceEvidenceState { service_change_state: "implemented".into(), evidence_state: "compile_checked".into() },
            vec!["control:access".into()], vec!["evidence:scope".into()], vec!["security:baseline".into()],
        ).unwrap();
        let after = acquisition_pareto_frontier(&obligation, &routes).unwrap();
        assert_eq!(before, after);
        assert_eq!(after.frontier_route_refs, vec!["blocked"]);
        assert!(!packet.creates_priority);
        assert!(!packet.creates_access_authority);
        assert!(!packet.creates_semantic_authority);
    }

    #[test]
    fn governed_packet_requires_evidence_for_claimed_state() {
        let result = build_inv_governance_packet(
            "gov:1", &obligation(), "purpose:investigation",
            GovernanceAccessState::Authorized,
            PrivacyExposureState::ContainsPii,
            AiUseState::IntendedUseDeclared,
            ServiceEvidenceState { service_change_state: "validated".into(), evidence_state: "runtime_observed".into() },
            vec!["control:1".into()], vec![], vec!["security:1".into()],
        );
        assert!(matches!(result, Err(GovernanceControlCaseError::MissingEvidence)));
    }
}
