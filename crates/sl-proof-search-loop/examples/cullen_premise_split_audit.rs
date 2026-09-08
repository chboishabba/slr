use sensiblaw_proof_search_loop::cullen_premise_audit::{
    audit_cullen_duty_premises, legacy_statutory_police_function_paid,
    police_function_implies_statutory_power, premise_audit_closes_consumer,
    premise_audit_is_duty_payment, CullenDutyRouteAuditStatus,
    PremiseObservationStatus, SourceLocatedPremiseObservation,
    FORESEEABLE_PHYSICAL_INJURY_RISK, POLICE_FUNCTION_CONTEXT,
    POSITIVE_OPERATIONAL_ACT, STATUTORY_POWER_INVOKED,
};
use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::transition::{
    apply_assessments, ResearchTermination, ResidualAssessment, ResidualAssessmentKind,
};

const SOURCE_REVISION: &str =
    "source-revision:sha256:f171fcaa304de4e1a8be9b7e2a200a181025a81456fa89dec18516805cad15b9";
const DOCUMENT: &str = "document:hca:[2026]-HCA-19:docx";

fn observation(
    premise_ref: &str,
    status: PremiseObservationStatus,
    evidence_refs: &[&str],
) -> SourceLocatedPremiseObservation {
    SourceLocatedPremiseObservation {
        premise_ref: premise_ref.into(),
        source_revision_ref: SOURCE_REVISION.into(),
        status,
        evidence_refs: evidence_refs.iter().map(|value| (*value).to_string()).collect(),
        candidate_only: true,
    }
}

fn main() {
    let audit = audit_cullen_duty_premises(
        observation(
            POSITIVE_OPERATIONAL_ACT,
            PremiseObservationStatus::SourceSupportedCandidate,
            &[
                &format!("{DOCUMENT}#paragraph-91"),
                &format!("{DOCUMENT}#paragraph-148"),
            ],
        ),
        observation(
            FORESEEABLE_PHYSICAL_INJURY_RISK,
            PremiseObservationStatus::SourceSupportedCandidate,
            &[
                &format!("{DOCUMENT}#paragraph-89"),
                &format!("{DOCUMENT}#paragraph-148"),
            ],
        ),
        observation(
            POLICE_FUNCTION_CONTEXT,
            PremiseObservationStatus::SourceSupportedCandidate,
            &[&format!("{DOCUMENT}#paragraph-89")],
        ),
        observation(
            STATUTORY_POWER_INVOKED,
            PremiseObservationStatus::SourceContradictedCandidate,
            &[&format!("{DOCUMENT}#paragraph-148")],
        ),
    )
    .expect("source-located Cullen premise audit");

    assert_eq!(
        audit.route_status,
        CullenDutyRouteAuditStatus::LegacyPremiseConflationDetected
    );
    assert!(!legacy_statutory_police_function_paid(&audit));
    assert!(!police_function_implies_statutory_power());
    assert!(!premise_audit_is_duty_payment(&audit));
    assert!(!premise_audit_closes_consumer(&audit));

    let frontier = ProofFrontier {
        consumer_ref: "consumer:cullen-positive-operational-duty".into(),
        frontier_ref: "frontier:cullen:premise-audit:v0".into(),
        residuals: vec![ProofResidual {
            residual_ref: "residual:cullen-positive-operational-act".into(),
            proposition_ref: "prop:cullen-positive-operational-duty".into(),
            producer_class_ref: "producer:exact-primary-authority".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("official-primary-case".into()),
            salience: 10,
            dependency_refs: vec![SOURCE_REVISION.into()],
            status: ResidualStatus::Open,
        }],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };

    let (next, transition) = apply_assessments(
        &frontier,
        "frontier:cullen:premise-audit:v1",
        &[ResidualAssessment {
            residual_ref: "residual:cullen-positive-operational-act".into(),
            kind: ResidualAssessmentKind::Narrowed,
            observed_proof_reduction: 1,
            assessment_ref: "assessment:cullen:legacy-statutory-function-premise-conflation".into(),
            assessment_authority: "experimental_candidate_only",
        }],
    )
    .expect("recompute Cullen frontier after premise split audit");

    assert_eq!(next.residuals[0].status, ResidualStatus::Open);
    assert_eq!(transition.termination, ResearchTermination::Continue);

    println!(
        "cullen_premise_split_audit=PASS positive_act=supported foreseeable_risk=supported police_function=supported statutory_power=contradicted legacy_bundled_premise=unpaid frontier_status=Open remaining_coordinate=rule-premise-reconstruction network=0 authority=experimental_candidate_only"
    );
}
