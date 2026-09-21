//! Theorem-bearing bridge for consumer adequacy.
//!
//! Runtime coordinate coverage is useful for routing but is not a proof of
//! query-indexed factorisation.  This module is the only legal-runtime surface
//! allowed to turn a coordinate-complete adequacy candidate into a
//! theorem-backed ConsumerAdequate receipt.  The promotion requires an explicit
//! external formal witness reference bound to the exact query and projection
//! digest.
//!
//! Conversely, exact non-factorability witnesses compile constructively into
//! typed research demands with provenance for the lost coordinate.

use serde::{Deserialize, Serialize};

use crate::{
    ConsumerAdequacyDisposition, ConsumerAdequacyReceipt, ConsumerAxis,
    ConsumerResearchDemand, ConsumerResearchDemandKind, ProjectionGraph,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactorsThroughWitnessReceipt {
    pub query_ref: String,
    pub projection_digest: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub exact_query_indexed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TheoremBackedConsumerAdequacyReceipt {
    pub query_ref: String,
    pub projection_digest: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub runtime_receipt: ConsumerAdequacyReceipt,
    pub factors_through_formally_proved: bool,
    pub consumer_adequate: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn certify_consumer_adequacy(
    runtime: &ConsumerAdequacyReceipt,
    graph: &ProjectionGraph,
    witness: &FactorsThroughWitnessReceipt,
) -> Result<TheoremBackedConsumerAdequacyReceipt, String> {
    if runtime.disposition != ConsumerAdequacyDisposition::Adequate
        || !runtime.missing_axes.is_empty()
        || !runtime.missing_semantic_refs.is_empty()
        || !runtime.research_demands.is_empty()
    {
        return Err(
            "formal consumer adequacy requires a coordinate-complete runtime candidate".into(),
        );
    }
    if runtime.query_ref != witness.query_ref {
        return Err("formal adequacy witness query_ref does not match runtime query".into());
    }
    if graph.deterministic_digest != witness.projection_digest {
        return Err("formal adequacy witness projection digest does not match runtime projection".into());
    }
    if witness.theorem_module_ref.trim().is_empty()
        || witness.theorem_ref.trim().is_empty()
        || witness.theorem_artifact_digest.trim().is_empty()
    {
        return Err("formal adequacy witness must name theorem module, theorem and artifact digest".into());
    }
    if !witness.exact_query_indexed {
        return Err("formal adequacy witness must be exact and query-indexed".into());
    }
    if !witness.candidate_only
        || witness.creates_semantic_authority
        || witness.creates_claim_truth
        || !runtime.candidate_only
        || runtime.creates_semantic_authority
        || runtime.creates_claim_truth
        || !graph.projection_only
        || graph.creates_semantic_authority
    {
        return Err("formal adequacy witness crossed non-promotion boundary".into());
    }

    let mut runtime_receipt = runtime.clone();
    runtime_receipt.factors_through_formally_proved = true;

    Ok(TheoremBackedConsumerAdequacyReceipt {
        query_ref: runtime.query_ref.clone(),
        projection_digest: graph.deterministic_digest.clone(),
        theorem_module_ref: witness.theorem_module_ref.clone(),
        theorem_ref: witness.theorem_ref.clone(),
        theorem_artifact_digest: witness.theorem_artifact_digest.clone(),
        runtime_receipt,
        factors_through_formally_proved: true,
        consumer_adequate: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonFactorabilityWitnessReceipt {
    pub query_ref: String,
    pub projection_digest: String,
    pub lost_axis: ConsumerAxis,
    pub left_world_ref: String,
    pub right_world_ref: String,
    pub shared_projection_ref: String,
    pub left_answer_ref: String,
    pub right_answer_ref: String,
    pub theorem_module_ref: String,
    pub theorem_ref: String,
    pub theorem_artifact_digest: String,
    pub exact_fibre_collision: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactConsumerResidual {
    pub residual_ref: String,
    pub query_ref: String,
    pub projection_digest: String,
    pub lost_axis: ConsumerAxis,
    pub demand: ConsumerResearchDemand,
    pub nonfactorability_theorem_ref: String,
    pub witness_refs: Vec<String>,
    pub reason_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn demand_kind(axis: ConsumerAxis) -> ConsumerResearchDemandKind {
    match axis {
        ConsumerAxis::SemanticIdentity => ConsumerResearchDemandKind::ResolveSemanticIdentity,
        ConsumerAxis::SourceRevision | ConsumerAxis::SourceSpan => {
            ConsumerResearchDemandKind::AcquireSource
        }
        ConsumerAxis::Provenance => ConsumerResearchDemandKind::RecoverProvenance,
        ConsumerAxis::Treatment => ConsumerResearchDemandKind::ReviewTreatment,
        ConsumerAxis::Temporal => ConsumerResearchDemandKind::ResolveTemporalCoordinate,
        ConsumerAxis::Jurisdiction => ConsumerResearchDemandKind::ResolveJurisdiction,
        ConsumerAxis::FactualPredicate => ConsumerResearchDemandKind::ReviewFact,
        ConsumerAxis::BurdenOrException => {
            ConsumerResearchDemandKind::ReviewBurdenOrException
        }
    }
}

fn axis_slug(axis: ConsumerAxis) -> &'static str {
    match axis {
        ConsumerAxis::SemanticIdentity => "semantic-identity",
        ConsumerAxis::SourceRevision => "source-revision",
        ConsumerAxis::SourceSpan => "source-span",
        ConsumerAxis::Provenance => "provenance",
        ConsumerAxis::Treatment => "treatment",
        ConsumerAxis::Temporal => "temporal",
        ConsumerAxis::Jurisdiction => "jurisdiction",
        ConsumerAxis::FactualPredicate => "factual-predicate",
        ConsumerAxis::BurdenOrException => "burden-or-exception",
    }
}

pub fn compile_nonfactorability_residual(
    witness: &NonFactorabilityWitnessReceipt,
    target_ref: impl Into<String>,
) -> Result<ExactConsumerResidual, String> {
    if witness.query_ref.trim().is_empty()
        || witness.projection_digest.trim().is_empty()
        || witness.left_world_ref.trim().is_empty()
        || witness.right_world_ref.trim().is_empty()
        || witness.shared_projection_ref.trim().is_empty()
        || witness.left_answer_ref.trim().is_empty()
        || witness.right_answer_ref.trim().is_empty()
        || witness.theorem_ref.trim().is_empty()
        || witness.theorem_module_ref.trim().is_empty()
        || witness.theorem_artifact_digest.trim().is_empty()
    {
        return Err("nonfactorability witness is incomplete".into());
    }
    if witness.left_world_ref == witness.right_world_ref {
        return Err("nonfactorability witness must compare distinct worlds".into());
    }
    if witness.left_answer_ref == witness.right_answer_ref {
        return Err("nonfactorability witness must expose distinct consumer answers".into());
    }
    if !witness.exact_fibre_collision {
        return Err("nonfactorability residual requires an exact fibre collision".into());
    }
    if !witness.candidate_only
        || witness.creates_semantic_authority
        || witness.creates_claim_truth
    {
        return Err("nonfactorability witness crossed non-promotion boundary".into());
    }

    let target_ref = target_ref.into();
    if target_ref.trim().is_empty() {
        return Err("nonfactorability residual target_ref must be non-empty".into());
    }
    let residual_ref = format!(
        "consumer-residual:{}:{}:{}",
        witness.query_ref,
        axis_slug(witness.lost_axis),
        witness.theorem_artifact_digest
    );
    let reason_ref = format!(
        "nonfactorability:{}:{}",
        witness.theorem_ref,
        axis_slug(witness.lost_axis)
    );

    Ok(ExactConsumerResidual {
        residual_ref,
        query_ref: witness.query_ref.clone(),
        projection_digest: witness.projection_digest.clone(),
        lost_axis: witness.lost_axis,
        demand: ConsumerResearchDemand {
            axis: witness.lost_axis,
            kind: demand_kind(witness.lost_axis),
            target_ref,
            reason_ref: reason_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        nonfactorability_theorem_ref: witness.theorem_ref.clone(),
        witness_refs: vec![
            witness.left_world_ref.clone(),
            witness.right_world_ref.clone(),
            witness.shared_projection_ref.clone(),
            witness.left_answer_ref.clone(),
            witness.right_answer_ref.clone(),
        ],
        reason_ref,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use crate::{ProjectionKind, ProjectionNode};

    fn graph() -> ProjectionGraph {
        ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![ProjectionNode {
                semantic_ref: "case:fixture".into(),
                semantic_kind: "CaseAuthority".into(),
                manifestation_refs: vec![],
                source_revision_refs: vec!["revision:fixture".into()],
                span_refs: vec!["span:fixture".into()],
                projection_role: "IssueProof".into(),
            }],
            edges: vec![],
            deterministic_digest: "sha256:projection".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn runtime() -> ConsumerAdequacyReceipt {
        ConsumerAdequacyReceipt {
            query_ref: "query:fixture".into(),
            disposition: ConsumerAdequacyDisposition::Adequate,
            paid_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
            ]),
            missing_axes: BTreeSet::new(),
            missing_semantic_refs: BTreeSet::new(),
            research_demands: vec![],
            unresolved_refs: BTreeSet::new(),
            factors_through_formally_proved: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn runtime_adequacy_does_not_promote_without_exact_theorem_witness() {
        let mut witness = FactorsThroughWitnessReceipt {
            query_ref: "query:fixture".into(),
            projection_digest: "sha256:projection".into(),
            theorem_module_ref: "DASHI.Law.ConsumerDirectedLegalFollowAdequacyExact".into(),
            theorem_ref: "fixtureFactorsThrough".into(),
            theorem_artifact_digest: "sha256:agda-fixture".into(),
            exact_query_indexed: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        assert!(certify_consumer_adequacy(&runtime(), &graph(), &witness).is_err());
        witness.exact_query_indexed = true;
        let certified = certify_consumer_adequacy(&runtime(), &graph(), &witness).unwrap();
        assert!(certified.consumer_adequate);
        assert!(certified.factors_through_formally_proved);
        assert!(certified.runtime_receipt.factors_through_formally_proved);
        assert!(!certified.creates_semantic_authority);
        assert!(!certified.creates_claim_truth);
    }

    #[test]
    fn exact_collision_compiles_to_exact_treatment_residual() {
        let witness = NonFactorabilityWitnessReceipt {
            query_ref: "query:treatment".into(),
            projection_digest: "sha256:projection".into(),
            lost_axis: ConsumerAxis::Treatment,
            left_world_ref: "world:treatment-missing".into(),
            right_world_ref: "world:treatment-paid".into(),
            shared_projection_ref: "projection:same-surface".into(),
            left_answer_ref: "answer:unresolved".into(),
            right_answer_ref: "answer:resolved".into(),
            theorem_module_ref: "DASHI.Law.ConsumerDirectedLegalFollowAdequacyExact".into(),
            theorem_ref: "coarseTreatmentDefect".into(),
            theorem_artifact_digest: "sha256:collision".into(),
            exact_fibre_collision: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let residual =
            compile_nonfactorability_residual(&witness, "case:target").unwrap();
        assert_eq!(residual.lost_axis, ConsumerAxis::Treatment);
        assert_eq!(
            residual.demand.kind,
            ConsumerResearchDemandKind::ReviewTreatment
        );
        assert!(residual
            .witness_refs
            .contains(&"world:treatment-missing".into()));
        assert!(!residual.creates_semantic_authority);
    }
}
