//! Source-located audit of the material premises used by the Cullen duty route.
//!
//! The retained judgment distinguishes a police-function context from an
//! intervention undertaken pursuant to statutory power.  This module prevents
//! those coordinates from being collapsed merely because both concern police.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PremiseObservationStatus {
    SourceSupportedCandidate,
    SourceContradictedCandidate,
    Underidentified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocatedPremiseObservation {
    pub premise_ref: String,
    pub source_revision_ref: String,
    pub status: PremiseObservationStatus,
    pub evidence_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullenDutyRouteAuditStatus {
    LegacyPremiseConflationDetected,
    RevisedRuleStillRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CullenDutyPremiseAudit {
    pub positive_operational_act: SourceLocatedPremiseObservation,
    pub foreseeable_physical_injury_risk: SourceLocatedPremiseObservation,
    pub police_function_context: SourceLocatedPremiseObservation,
    pub statutory_power_invoked: SourceLocatedPremiseObservation,
    pub route_status: CullenDutyRouteAuditStatus,
    pub audit_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CullenPremiseAuditError {
    MissingEvidence(&'static str),
    PremiseNotCandidateOnly(&'static str),
    UnexpectedPremiseRef(&'static str),
}

pub const POSITIVE_OPERATIONAL_ACT: &str = "prop:Cullen:positive-operational-act";
pub const FORESEEABLE_PHYSICAL_INJURY_RISK: &str =
    "prop:Cullen:foreseeable-physical-injury-risk";
pub const POLICE_FUNCTION_CONTEXT: &str = "prop:Cullen:police-function-context";
pub const STATUTORY_POWER_INVOKED: &str = "prop:Cullen:statutory-power-invoked";
pub const LEGACY_STATUTORY_POLICE_FUNCTION: &str = "prop:Cullen:statutory-police-function";

fn validate_observation(
    observation: &SourceLocatedPremiseObservation,
    expected_ref: &'static str,
    label: &'static str,
) -> Result<(), CullenPremiseAuditError> {
    if observation.premise_ref != expected_ref {
        return Err(CullenPremiseAuditError::UnexpectedPremiseRef(label));
    }
    if !observation.candidate_only {
        return Err(CullenPremiseAuditError::PremiseNotCandidateOnly(label));
    }
    if observation.evidence_refs.is_empty() || observation.evidence_refs.iter().any(String::is_empty) {
        return Err(CullenPremiseAuditError::MissingEvidence(label));
    }
    Ok(())
}

pub fn audit_cullen_duty_premises(
    positive_operational_act: SourceLocatedPremiseObservation,
    foreseeable_physical_injury_risk: SourceLocatedPremiseObservation,
    police_function_context: SourceLocatedPremiseObservation,
    statutory_power_invoked: SourceLocatedPremiseObservation,
) -> Result<CullenDutyPremiseAudit, CullenPremiseAuditError> {
    validate_observation(&positive_operational_act, POSITIVE_OPERATIONAL_ACT, "positive-operational-act")?;
    validate_observation(
        &foreseeable_physical_injury_risk,
        FORESEEABLE_PHYSICAL_INJURY_RISK,
        "foreseeable-physical-injury-risk",
    )?;
    validate_observation(&police_function_context, POLICE_FUNCTION_CONTEXT, "police-function-context")?;
    validate_observation(&statutory_power_invoked, STATUTORY_POWER_INVOKED, "statutory-power-invoked")?;

    let conflation_detected =
        police_function_context.status == PremiseObservationStatus::SourceSupportedCandidate
            && statutory_power_invoked.status
                != PremiseObservationStatus::SourceSupportedCandidate;

    Ok(CullenDutyPremiseAudit {
        positive_operational_act,
        foreseeable_physical_injury_risk,
        police_function_context,
        statutory_power_invoked,
        route_status: if conflation_detected {
            CullenDutyRouteAuditStatus::LegacyPremiseConflationDetected
        } else {
            CullenDutyRouteAuditStatus::RevisedRuleStillRequired
        },
        audit_authority: "experimental_candidate_only",
    })
}

/// The historical bundled `statutoryPoliceFunction` coordinate cannot be paid
/// merely because the police were performing a police function.
pub fn legacy_statutory_police_function_paid(audit: &CullenDutyPremiseAudit) -> bool {
    audit.police_function_context.status == PremiseObservationStatus::SourceSupportedCandidate
        && audit.statutory_power_invoked.status == PremiseObservationStatus::SourceSupportedCandidate
}

pub const fn police_function_implies_statutory_power() -> bool {
    false
}

pub const fn premise_audit_is_duty_payment(_audit: &CullenDutyPremiseAudit) -> bool {
    false
}

pub const fn premise_audit_closes_consumer(_audit: &CullenDutyPremiseAudit) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        premise_ref: &str,
        status: PremiseObservationStatus,
        evidence: &str,
    ) -> SourceLocatedPremiseObservation {
        SourceLocatedPremiseObservation {
            premise_ref: premise_ref.into(),
            source_revision_ref: "source-revision:cullen".into(),
            status,
            evidence_refs: vec![evidence.into()],
            candidate_only: true,
        }
    }

    #[test]
    fn police_function_does_not_manufacture_statutory_power() {
        let audit = audit_cullen_duty_premises(
            observation(POSITIVE_OPERATIONAL_ACT, PremiseObservationStatus::SourceSupportedCandidate, "paragraph:91"),
            observation(FORESEEABLE_PHYSICAL_INJURY_RISK, PremiseObservationStatus::SourceSupportedCandidate, "paragraph:89"),
            observation(POLICE_FUNCTION_CONTEXT, PremiseObservationStatus::SourceSupportedCandidate, "paragraph:89"),
            observation(STATUTORY_POWER_INVOKED, PremiseObservationStatus::SourceContradictedCandidate, "paragraph:148"),
        )
        .unwrap();
        assert_eq!(
            audit.route_status,
            CullenDutyRouteAuditStatus::LegacyPremiseConflationDetected
        );
        assert!(!legacy_statutory_police_function_paid(&audit));
        assert!(!police_function_implies_statutory_power());
        assert!(!premise_audit_is_duty_payment(&audit));
        assert!(!premise_audit_closes_consumer(&audit));
    }
}
