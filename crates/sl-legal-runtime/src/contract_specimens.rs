//! Contract-law regression specimens for the generic legal runtime.
//!
//! The important invariant is that Mann and Waltons enter through existing
//! evidence, WrongType, LegalFollow, projection and MatterRuntime surfaces.
//! This module is fixture/configuration code; it does not add a contract-law
//! reducer or a fifth AustralianCalibrationKind.

use std::collections::BTreeMap;

use sensiblaw_core::canonical_evidence::{EvidenceObservation, EvidenceSpan};
use sensiblaw_legal_follow_plan::{
    mann_paterson_trace, waltons_estoppel_trace, AustralianContractTrace,
};
use sensiblaw_reviewed_evidence_payment::{
    ReviewedCanonicalEvidence, ReviewedEvidenceCoordinate,
};
use sensiblaw_proof_search_loop::waltons_proposition_review::{
    EstoppelRequirementRole, PropositionEvidenceDisposition,
    ReviewedWaltonsPropositionEvidenceReceipt,
};

use crate::{
    action_for_residual, compile_matter_runtime_from_state, evaluate_source_realised_rule,
    project_matter_issue_workbench_from_state, project_reviewed_world_to_wrong_type,
    residuals_for_evaluation, AuthorityRole, EvidenceDisposition,
    InformationAction, LegalElementKind, LegalEvaluationContext, LegalProjectionState,
    LegalRuntimeError, MatterEventProjection, MatterRuntime, MatterWorkbenchSeed,
    ProjectionContext, PropositionState, PropositionStatus, SourceRealisedLegalRule,
    WrongElementRequirement, WrongTypeIssueState, WrongTypeRuleBundle,
};

fn reviewed_text(
    observation_ref: &str,
    source_revision_ref: &str,
    span_ref: &str,
    predicate_ref: &str,
    value_ref: &str,
) -> Result<ReviewedCanonicalEvidence, LegalRuntimeError> {
    let observation = EvidenceObservation {
        observation_ref: observation_ref.into(),
        source_revision_ref: source_revision_ref.into(),
        span: EvidenceSpan::text(source_revision_ref, span_ref, 0, 64)
            .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?,
        predicate_ref: predicate_ref.into(),
        value_ref: value_ref.into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let review = ReviewedEvidenceCoordinate {
        review_ref: format!("review:{observation_ref}"),
        consumer_id: "consumer:contract-unseen-matter".into(),
        requirement_id: format!("requirement:{observation_ref}"),
        coordinate: sensiblaw_consumer_residual::EvidenceCoordinateKind::Authority,
        source_ref: Some(format!("manifestation:{source_revision_ref}")),
        evidence_ref: observation.observation_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    ReviewedCanonicalEvidence::from_reviewed_coordinate(
        &review,
        observation,
        format!("payment:{observation_ref}"),
    )
    .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))
}

fn waltons_element_ref(role: EstoppelRequirementRole) -> &'static str {
    match role {
        EstoppelRequirementRole::AssumptionOrExpectation => "element:estoppel:assumption",
        EstoppelRequirementRole::Reliance => "element:estoppel:reliance",
        EstoppelRequirementRole::Detriment => "element:estoppel:detriment",
        EstoppelRequirementRole::Unconscionability => "element:estoppel:unconscionability",
    }
}

pub fn waltons_estoppel_bundle() -> WrongTypeRuleBundle {
    WrongTypeRuleBundle {
        wrong_type_ref: "wrong:contract:estoppel:waltons".into(),
        elements: vec![
            WrongElementRequirement {
                element_ref: waltons_element_ref(EstoppelRequirementRole::AssumptionOrExpectation).into(),
                kind: LegalElementKind::Other,
                proposition_ref: EstoppelRequirementRole::AssumptionOrExpectation.proposition_ref().into(),
                required: true,
            },
            WrongElementRequirement {
                element_ref: waltons_element_ref(EstoppelRequirementRole::Reliance).into(),
                kind: LegalElementKind::Other,
                proposition_ref: EstoppelRequirementRole::Reliance.proposition_ref().into(),
                required: true,
            },
            WrongElementRequirement {
                element_ref: waltons_element_ref(EstoppelRequirementRole::Detriment).into(),
                kind: LegalElementKind::Damage,
                proposition_ref: EstoppelRequirementRole::Detriment.proposition_ref().into(),
                required: true,
            },
            WrongElementRequirement {
                element_ref: waltons_element_ref(EstoppelRequirementRole::Unconscionability).into(),
                kind: LegalElementKind::Other,
                proposition_ref: EstoppelRequirementRole::Unconscionability.proposition_ref().into(),
                required: true,
            },
        ],
        source_rule_refs: vec!["case:au:hca:1988:7".into()],
    }
}

fn waltons_evidence_disposition(
    value: PropositionEvidenceDisposition,
) -> EvidenceDisposition {
    match value {
        PropositionEvidenceDisposition::Supports => EvidenceDisposition::Supports,
        PropositionEvidenceDisposition::Contests => EvidenceDisposition::Contests,
        PropositionEvidenceDisposition::ContextOnly => EvidenceDisposition::DoesNotAddress,
    }
}

pub fn project_waltons_reviewed_receipts_to_issue(
    receipts: &[ReviewedWaltonsPropositionEvidenceReceipt],
) -> Result<WrongTypeIssueState, LegalRuntimeError> {
    let links = receipts
        .iter()
        .filter_map(|receipt| {
            receipt.reviewed_evidence.as_ref().map(|reviewed| {
                (
                    reviewed,
                    waltons_element_ref(receipt.role),
                    waltons_evidence_disposition(receipt.disposition),
                )
            })
        })
        .collect::<Vec<_>>();
    project_reviewed_world_to_wrong_type(&waltons_estoppel_bundle(), &links)
}

fn mann_bundle() -> WrongTypeRuleBundle {
    WrongTypeRuleBundle {
        wrong_type_ref: "wrong:contract:mann:termination-restitution".into(),
        elements: vec![
            WrongElementRequirement {
                element_ref: "element:mann:contract".into(),
                kind: LegalElementKind::Other,
                proposition_ref: "prop:mann:contract-existed".into(),
                required: true,
            },
            WrongElementRequirement {
                element_ref: "element:mann:repudiation-termination".into(),
                kind: LegalElementKind::Breach,
                proposition_ref: "prop:mann:repudiation-termination".into(),
                required: true,
            },
            WrongElementRequirement {
                element_ref: "element:mann:statutory-s38".into(),
                kind: LegalElementKind::Statutory,
                proposition_ref: "prop:mann:dbc-act-s38-overlay".into(),
                required: true,
            },
            WrongElementRequirement {
                element_ref: "element:mann:restitution".into(),
                kind: LegalElementKind::Remedy,
                proposition_ref: "prop:mann:restitution-quantum-meruit".into(),
                required: true,
            },
        ],
        source_rule_refs: vec![
            "case:au:hca:2019:32".into(),
            "legislation:vic:domestic-building-contracts-act-1995:s38".into(),
        ],
    }
}

fn mann_rule() -> SourceRealisedLegalRule {
    SourceRealisedLegalRule {
        rule_ref: "rule:mann:termination-restitution-with-s38-overlay".into(),
        source_revision_ref: "source-revision:hca:2019:32".into(),
        source_span_refs: vec![
            "span:hca:2019:32:termination-restitution".into(),
            "span:vic:dbc-act-1995:s38".into(),
        ],
        conclusion_ref: "prop:mann:remedy-disposition".into(),
        premise_refs: vec![
            "prop:mann:contract-existed".into(),
            "prop:mann:repudiation-termination".into(),
        ],
        exception_refs: Vec::new(),
        defeater_refs: vec!["prop:mann:statutory-limit-s38".into()],
        burden_refs: vec!["prop:mann:remedy-burden".into()],
        jurisdiction_ref: "AU".into(),
        valid_from: "2019-10-09".into(),
        valid_to: None,
        authority_role: AuthorityRole::Binding,
        source_realised: true,
        candidate_only: true,
    }
}

fn mann_issue() -> Result<WrongTypeIssueState, LegalRuntimeError> {
    let hca = reviewed_text(
        "observation:mann:hca32",
        "source-revision:hca:2019:32",
        "span:hca:2019:32:termination-restitution",
        "predicate:authority-reports",
        "value:mann-termination-restitution",
    )?;
    let statute = reviewed_text(
        "observation:mann:vic-dbc-s38",
        "source-revision:vic:dbc-act-1995:current",
        "span:vic:dbc-act-1995:s38",
        "predicate:legislation-provision",
        "value:domestic-building-contracts-act-1995-s38",
    )?;
    project_reviewed_world_to_wrong_type(
        &mann_bundle(),
        &[
            (&hca, "element:mann:contract", EvidenceDisposition::Supports),
            (&hca, "element:mann:repudiation-termination", EvidenceDisposition::Supports),
            (&statute, "element:mann:statutory-s38", EvidenceDisposition::Supports),
            (&hca, "element:mann:restitution", EvidenceDisposition::Contests),
        ],
    )
}

fn mann_projection_state(issue: WrongTypeIssueState) -> Result<(WrongTypeIssueState, LegalProjectionState), LegalRuntimeError> {
    let propositions = BTreeMap::from([
        (
            "prop:mann:contract-existed".into(),
            PropositionState {
                proposition_ref: "prop:mann:contract-existed".into(),
                status: PropositionStatus::Established,
                source_refs: vec!["source-revision:hca:2019:32".into()],
            },
        ),
        (
            "prop:mann:repudiation-termination".into(),
            PropositionState {
                proposition_ref: "prop:mann:repudiation-termination".into(),
                status: PropositionStatus::Established,
                source_refs: vec!["source-revision:hca:2019:32".into()],
            },
        ),
        (
            "prop:mann:statutory-limit-s38".into(),
            PropositionState {
                proposition_ref: "prop:mann:statutory-limit-s38".into(),
                status: PropositionStatus::Unresolved,
                source_refs: vec!["source-revision:vic:dbc-act-1995:current".into()],
            },
        ),
        (
            "prop:mann:remedy-burden".into(),
            PropositionState {
                proposition_ref: "prop:mann:remedy-burden".into(),
                status: PropositionStatus::Unresolved,
                source_refs: vec!["source-revision:hca:2019:32".into()],
            },
        ),
    ]);
    let context = LegalEvaluationContext {
        jurisdiction_ref: "AU".into(),
        as_at: "2026-09-20".into(),
        propositions,
        wrong_type: issue.clone(),
    };
    let evaluation = evaluate_source_realised_rule(&mann_rule(), &context)?;
    let residuals = residuals_for_evaluation(&evaluation, &issue);
    let selected_action: Option<InformationAction> = residuals
        .iter()
        .find_map(action_for_residual);
    Ok((
        issue,
        LegalProjectionState {
            evaluation,
            residuals,
            selected_action,
        },
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnseenContractMatterReceipt {
    pub matter_ref: String,
    pub trace: AustralianContractTrace,
    pub runtime: MatterRuntime,
    pub used_calibration_enum: bool,
    pub contract_specific_reducer_added: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
}

pub fn build_mann_unseen_matter_runtime() -> Result<UnseenContractMatterReceipt, LegalRuntimeError> {
    let (issue, state) = mann_projection_state(mann_issue()?)?;
    let anchor = issue
        .elements
        .iter()
        .flat_map(|element| element.evidence.iter())
        .next()
        .ok_or_else(|| LegalRuntimeError::Projection("Mann specimen has no reviewed evidence".into()))?
        .observation_ref
        .clone();
    let workbench = project_matter_issue_workbench_from_state(
        "matter:au:hca:2019:32",
        &issue,
        &state,
        MatterWorkbenchSeed {
            events: vec![MatterEventProjection {
                event_ref: "event:mann:termination-remedy-dispute".into(),
                label: "Mann termination/restitution dispute".into(),
                time_ref: "2019-10-09".into(),
                observation_refs: vec![anchor.clone()],
                entity_refs: Vec::new(),
                candidate_only: true,
            }],
            observation_time_refs: BTreeMap::from([(anchor.clone(), "2019-10-09".into())]),
            ..MatterWorkbenchSeed::default()
        },
    )
    .map_err(LegalRuntimeError::Projection)?;

    let runtime = compile_matter_runtime_from_state(
        workbench,
        &issue,
        &state,
        ProjectionContext {
            temporal_refs: BTreeMap::from([(anchor, "2019-10-09".into())]),
            jurisdiction_refs: BTreeMap::new(),
        },
    )
    .map_err(LegalRuntimeError::Projection)?;

    Ok(UnseenContractMatterReceipt {
        matter_ref: "matter:au:hca:2019:32".into(),
        trace: mann_paterson_trace(),
        runtime,
        used_calibration_enum: false,
        contract_specific_reducer_added: false,
        candidate_only: true,
        creates_semantic_authority: false,
    })
}

pub fn waltons_estoppel_materialisation_specimen() -> AustralianContractTrace {
    waltons_estoppel_trace()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MatterCommand;

    #[test]
    fn waltons_wrong_type_bundle_is_generic_and_unpaid_until_reviewed_evidence_arrives() {
        let issue = project_waltons_reviewed_receipts_to_issue(&[]).unwrap();
        assert_eq!(issue.elements.len(), 4);
        assert!(issue
            .elements
            .iter()
            .all(|element| element.disposition == crate::ElementDisposition::Unresolved));
        assert!(!issue.applicability_promoted);
        assert!(!issue.liability_promoted);
    }

    #[test]
    fn waltons_estoppel_enters_as_follow_trace_not_new_runtime_semantics() {
        let trace = waltons_estoppel_materialisation_specimen();
        assert!(trace.nodes.contains_key("case:au:hca:1988:7"));
        assert!(trace.nodes.contains_key("requirement:estoppel:detriment"));
        assert!(!trace.creates_legal_authority);
    }

    #[test]
    fn mann_is_a_fifth_unseen_matter_without_calibration_enum_or_contract_reducer() {
        let mut receipt = build_mann_unseen_matter_runtime().unwrap();
        assert!(!receipt.used_calibration_enum);
        assert!(!receipt.contract_specific_reducer_added);
        assert!(!receipt.creates_semantic_authority);
        assert!(receipt.runtime.workbench.issue_element_count() >= 4);

        let anchor = receipt.runtime.workbench.observations[0].observation_ref.clone();
        let selection = receipt
            .runtime
            .dispatch(MatterCommand::SelectObject(anchor.clone()))
            .unwrap();
        assert!(selection.projection.contains(&anchor));
        assert!(!selection.creates_semantic_authority);

        let explanation = receipt.runtime.dispatch(MatterCommand::Explain(anchor)).unwrap();
        assert!(!explanation.creates_semantic_authority);
    }

    #[test]
    fn mann_keeps_contested_remedy_open_instead_of_promoting_liability() {
        let receipt = build_mann_unseen_matter_runtime().unwrap();
        let restitution = receipt
            .runtime
            .workbench
            .observations
            .iter()
            .flat_map(|observation| observation.element_refs.iter())
            .any(|element| element == "element:mann:restitution");
        assert!(restitution);
        let element = receipt
            .runtime
            .workbench
            .issue
            .nodes
            .iter()
            .find(|node| node.node_ref == "element:mann:restitution")
            .unwrap();
        assert!(!element.residual_refs.is_empty());
        assert!(!receipt.creates_semantic_authority);
    }
}
