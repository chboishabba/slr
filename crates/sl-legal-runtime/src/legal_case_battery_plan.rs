//! S21 source-driven legal case battery plan compiler.
//!
//! Converts a case-battery specimen into a typed initial frontier and the
//! existing adversarial hypothesis family.  It does not infer cross-matter
//! joins. Source/citation seeds are acquisition/navigation inputs only.

use std::collections::BTreeSet;

use sensiblaw_proof_search_loop::{
    frontier::{ProofFrontier, ProofResidual, ResidualStatus},
};

use crate::{
    compile_frontier_adversarial_search, AdversarialProofGraph,
    AdversarialSearchDemand, CaseBatteryKind, LegalCaseBatterySpecimen,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseBatteryInitialPlan {
    pub kind: CaseBatteryKind,
    pub consumer_ref: String,
    pub question_ref: String,
    pub frontier: ProofFrontier,
    pub adversarial_demands: Vec<AdversarialSearchDemand>,
    pub source_seed_refs: Vec<String>,
    pub citation_seed_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn baseline_residuals(specimen: &LegalCaseBatterySpecimen) -> Vec<ProofResidual> {
    let source_residual = ProofResidual {
        residual_ref: format!("residual:{}:primary-source", specimen.consumer_ref),
        proposition_ref: format!("proposition:{}:source-grounded-question", specimen.question_ref),
        producer_class_ref: "producer:primary-legal-source".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 100,
        dependency_refs: vec![],
        status: ResidualStatus::Open,
    };
    let authority_residual = ProofResidual {
        residual_ref: format!("residual:{}:authority-treatment", specimen.consumer_ref),
        proposition_ref: format!("proposition:{}:authority-chain", specimen.question_ref),
        producer_class_ref: "producer:authority-treatment".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: Some("reviewed-current-authority-chain".into()),
        salience: 90,
        dependency_refs: specimen.citation_seed_refs.clone(),
        status: ResidualStatus::Open,
    };
    let wrongtype_residual = ProofResidual {
        residual_ref: format!("residual:{}:wrongtype-discriminator", specimen.consumer_ref),
        proposition_ref: format!("proposition:{}:typed-legal-requirements", specimen.question_ref),
        producer_class_ref: "producer:wrongtype-discriminator".into(),
        jurisdiction_ref: Some("AU".into()),
        authority_requirement_ref: None,
        salience: 80,
        dependency_refs: vec![],
        status: ResidualStatus::Underidentified,
    };
    vec![source_residual, authority_residual, wrongtype_residual]
}

pub fn compile_case_battery_initial_plan(
    specimen: &LegalCaseBatterySpecimen,
) -> Result<CaseBatteryInitialPlan, String> {
    specimen.validate()?;

    let frontier = ProofFrontier {
        consumer_ref: specimen.consumer_ref.clone(),
        frontier_ref: format!("frontier:{}:initial", specimen.consumer_ref),
        residuals: baseline_residuals(specimen),
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };
    let graph = AdversarialProofGraph::new();
    let adversarial_demands = compile_frontier_adversarial_search(&frontier, &graph);

    // Every explicitly required adversarial role must have a generated demand
    // except CounterDefeater, which only becomes meaningful after an actual
    // reviewed defeat/contestation exists.
    let generated_roles = adversarial_demands
        .iter()
        .map(|demand| demand.role)
        .collect::<BTreeSet<_>>();
    for role in &specimen.required_adversarial_roles {
        if *role == crate::AdversarialSearchRole::CounterDefeater {
            continue;
        }
        if !generated_roles.contains(role) {
            return Err(format!(
                "initial case-battery plan failed to generate required adversarial role {role:?}"
            ));
        }
    }

    Ok(CaseBatteryInitialPlan {
        kind: specimen.kind,
        consumer_ref: specimen.consumer_ref.clone(),
        question_ref: specimen.question_ref.clone(),
        frontier,
        adversarial_demands,
        source_seed_refs: specimen.seed_source_refs.clone(),
        citation_seed_refs: specimen.citation_seed_refs.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{legal_case_battery, AdversarialSearchRole};

    #[test]
    fn all_case_battery_specs_compile_to_adversarial_initial_frontiers() {
        for specimen in legal_case_battery() {
            let plan = compile_case_battery_initial_plan(&specimen).unwrap();
            assert_eq!(plan.consumer_ref, specimen.consumer_ref);
            assert!(plan.frontier.open_residuals().count() >= 2);
            assert!(plan
                .adversarial_demands
                .iter()
                .any(|demand| demand.role == AdversarialSearchRole::Defeater));
            assert!(!plan.creates_semantic_authority);
            assert!(!plan.creates_claim_truth);
        }
    }

    #[test]
    fn initial_plan_does_not_fabricate_cross_matter_dependency_join() {
        for specimen in legal_case_battery() {
            let plan = compile_case_battery_initial_plan(&specimen).unwrap();
            assert!(plan
                .frontier
                .residuals
                .iter()
                .flat_map(|residual| residual.dependency_refs.iter())
                .all(|reference| {
                    reference != "coordinate:mabo-native-title"
                        && reference != "shared-join:mabo"
                }));
        }
    }

    #[test]
    fn yindjibarndi_keeps_yunupingu_as_citation_seed_not_paid_join() {
        let specimen = legal_case_battery()
            .into_iter()
            .find(|item| item.kind == CaseBatteryKind::YindjibarndiYunupingu)
            .unwrap();
        let plan = compile_case_battery_initial_plan(&specimen).unwrap();
        assert_eq!(plan.citation_seed_refs, vec!["case:au:hca:2025:6"]);
        assert!(!plan.frontier.satisfied_payment_refs.iter().any(|payment| {
            payment.contains("mabo") || payment.contains("yunupingu")
        }));
    }
}
