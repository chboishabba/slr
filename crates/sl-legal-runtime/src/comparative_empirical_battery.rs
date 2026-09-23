//! M11.4 cross-domain empirical/regression battery.
//!
//! One generic comparison programme must distinguish:
//! 1. world/state changes while theory does not;
//! 2. theory changes while world does not;
//! 3. observation changes while world does not;
//! 4. consumer projection changes while world does not;
//! 5. a legal defeater changes the route;
//! 6. an irrelevant revision does not change the query answer/projection.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    compile_query_world_impact, run_observation_refinement,
    run_pabai_comparative_regression, run_personal_professional_comparison,
    run_same_world_theory_change, run_state_change_regularity_invariant,
    temporal_comparison_from_query_world_impact, ChangeLayer, ConsumerAxis,
    LegalWorldCoordinate, ProjectionGraph, ProjectionKind, ProjectionNode,
    QueryDependencySlice, RevisionDependencyIndex,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeEmpiricalBatteryReceipt {
    pub world_change_theory_invariant: bool,
    pub theory_change_world_invariant: bool,
    pub observation_change_world_invariant: bool,
    pub consumer_projection_change_world_invariant: bool,
    pub legal_defeater_changes_route: bool,
    pub irrelevant_revision_changes_query_projection: bool,
    pub irrelevant_revision_reopens_research: bool,
    pub all_modes_candidate_only: bool,
    pub any_mode_creates_semantic_authority: bool,
    pub any_mode_creates_claim_truth: bool,
}

fn temporal_world(
    world_ref: &str,
    relevant_revision: &str,
    irrelevant_revision: &str,
) -> LegalWorldCoordinate {
    LegalWorldCoordinate {
        world_ref: world_ref.into(),
        matter_ref: "matter:m11-battery".into(),
        jurisdiction_ref: "AU".into(),
        as_at: "2026-09-23".into(),
        source_revisions: BTreeMap::from([
            ("source:relevant".into(), relevant_revision.into()),
            ("source:noise".into(), irrelevant_revision.into()),
        ]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn temporal_graph() -> ProjectionGraph {
    ProjectionGraph {
        kind: ProjectionKind::IssueProof,
        nodes: vec![
            ProjectionNode {
                semantic_ref: "prop:q".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:q".into()],
                source_revision_refs: vec!["rev:relevant:1".into()],
                span_refs: vec!["span:q".into()],
                projection_role: "IssueProof".into(),
            },
            ProjectionNode {
                semantic_ref: "prop:noise".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:noise".into()],
                source_revision_refs: vec!["rev:noise:1".into()],
                span_refs: vec!["span:noise".into()],
                projection_role: "IssueProof".into(),
            },
        ],
        edges: vec![],
        deterministic_digest: "sha256:m11-battery-temporal".into(),
        projection_only: true,
        creates_semantic_authority: false,
    }
}

fn temporal_dependencies() -> RevisionDependencyIndex {
    RevisionDependencyIndex {
        source_to_propositions: BTreeMap::from([
            (
                "source:relevant".into(),
                BTreeSet::from(["prop:q".into()]),
            ),
            (
                "source:noise".into(),
                BTreeSet::from(["prop:noise".into()]),
            ),
        ]),
        proposition_dependents: BTreeMap::new(),
    }
}

fn temporal_slice() -> QueryDependencySlice {
    QueryDependencySlice {
        query_ref: "query:m11-battery".into(),
        required_axes: BTreeSet::from([
            ConsumerAxis::SemanticIdentity,
            ConsumerAxis::SourceRevision,
        ]),
        semantic_refs: BTreeSet::from(["prop:q".into()]),
        proof_refs: BTreeSet::new(),
        source_refs: BTreeSet::from(["source:relevant".into()]),
        source_revision_refs: BTreeSet::from(["rev:relevant:1".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn run_comparative_empirical_battery(
) -> Result<ComparativeEmpiricalBatteryReceipt, String> {
    let state = run_state_change_regularity_invariant()?;
    let theory = run_same_world_theory_change()?;
    let observation = run_observation_refinement()?;
    let fibres = run_personal_professional_comparison()?;
    let pabai = run_pabai_comparative_regression()?;

    let old = temporal_world("world:battery:t0", "rev:relevant:1", "rev:noise:1");
    let irrelevant =
        temporal_world("world:battery:t1", "rev:relevant:1", "rev:noise:2");
    let temporal_impact = compile_query_world_impact(
        &old,
        &irrelevant,
        &temporal_dependencies(),
        &temporal_slice(),
        &temporal_graph(),
    )?;
    let temporal = temporal_comparison_from_query_world_impact(&temporal_impact)?;

    let consumer_projection_change_world_invariant =
        fibres.personal_to_lawyer.world_ref == fibres.personal_to_regulator.world_ref
            && (fibres.personal_to_lawyer.shared_visible_coordinate_refs
                != fibres.personal_to_regulator.shared_visible_coordinate_refs
                || fibres
                    .personal_to_lawyer
                    .right_dependency_irrelevant_coordinate_refs
                    != fibres
                        .personal_to_regulator
                        .right_dependency_irrelevant_coordinate_refs);

    Ok(ComparativeEmpiricalBatteryReceipt {
        world_change_theory_invariant: state.state_changed
            && !state.theory_changed
            && state
                .typed_changes
                .changed_layers
                .contains(&ChangeLayer::World),
        theory_change_world_invariant: theory.world_identity_held_fixed
            && theory
                .typed_changes
                .changed_layers
                .contains(&ChangeLayer::Theory)
            && theory
                .typed_changes
                .invariant_layers
                .contains(&ChangeLayer::World),
        observation_change_world_invariant: observation.world_identity_held_fixed
            && observation
                .typed_changes
                .changed_layers
                .contains(&ChangeLayer::Observation)
            && observation
                .typed_changes
                .invariant_layers
                .contains(&ChangeLayer::World),
        consumer_projection_change_world_invariant,
        legal_defeater_changes_route: pabai
            .w0_to_w1
            .changed_route_refs
            .contains("route:pabai:comparative-duty")
            && pabai
                .w0_to_w1_distinction
                .delta_refs
                .contains("delta:pabai:w0-w1:defeater"),
        irrelevant_revision_changes_query_projection: temporal.query_projection_changed,
        irrelevant_revision_reopens_research: temporal.reopens_consumer_research,
        all_modes_candidate_only: state.candidate_only
            && theory.candidate_only
            && observation.candidate_only
            && fibres.candidate_only
            && pabai.candidate_only
            && temporal.candidate_only,
        any_mode_creates_semantic_authority: state.creates_semantic_authority
            || theory.creates_semantic_authority
            || observation.creates_semantic_authority
            || fibres.creates_semantic_authority
            || pabai.creates_semantic_authority
            || temporal.creates_semantic_authority,
        any_mode_creates_claim_truth: state.creates_claim_truth
            || theory.creates_claim_truth
            || observation.creates_claim_truth
            || fibres.creates_claim_truth
            || pabai.creates_claim_truth
            || temporal.creates_claim_truth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_comparison_battery_distinguishes_all_six_change_patterns() {
        let receipt = run_comparative_empirical_battery().unwrap();

        assert!(receipt.world_change_theory_invariant);
        assert!(receipt.theory_change_world_invariant);
        assert!(receipt.observation_change_world_invariant);
        assert!(receipt.consumer_projection_change_world_invariant);
        assert!(receipt.legal_defeater_changes_route);
        assert!(!receipt.irrelevant_revision_changes_query_projection);
        assert!(!receipt.irrelevant_revision_reopens_research);
        assert!(receipt.all_modes_candidate_only);
        assert!(!receipt.any_mode_creates_semantic_authority);
        assert!(!receipt.any_mode_creates_claim_truth);
    }
}
