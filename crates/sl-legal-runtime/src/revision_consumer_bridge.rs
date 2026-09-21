//! S15/S18 bridge: revision invalidation -> stale projection coordinates ->
//! consumer research reopening.
//!
//! A ProjectionGraph that still contains an old source revision must not keep
//! paying SourceRevision/SourceSpan/Provenance after the legal world has moved
//! to a new pinned source manifestation.  We derive a read-only invalidated
//! projection with stale coordinates removed and a new deterministic digest,
//! then run the ordinary S15.6 adequacy compiler over that derived projection.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::{
    affected_proof_cone, compile_consumer_adequacy, diff_world_revisions,
    revision_rereview_plan, ConsumerAdequacyCompilation, ConsumerCoverage,
    ConsumerQueryDemand, KernelCheckedFactorsThroughWitness, LegalWorldCoordinate,
    NonFactorabilityWitnessReceipt, OperationalResearchState, ProjectionGraph,
    RevisionDependencyIndex, RevisionInvalidationReceipt, RevisionReReviewPlan,
    AffectedProofCone,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionInvalidatedProjection {
    pub graph: ProjectionGraph,
    pub stale_semantic_refs: BTreeSet<String>,
    pub stale_revision_refs: BTreeSet<String>,
    pub source_projection_digest: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn sha256(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn invalidate_projection_for_revision(
    graph: &ProjectionGraph,
    invalidation: &RevisionInvalidationReceipt,
) -> Result<RevisionInvalidatedProjection, String> {
    if !graph.projection_only
        || graph.creates_semantic_authority
        || !invalidation.candidate_only
        || invalidation.creates_semantic_authority
        || invalidation.creates_claim_truth
    {
        return Err("revision projection invalidation crossed non-promotion boundary".into());
    }

    let stale_revision_refs = invalidation
        .changes
        .iter()
        .filter_map(|change| change.old_revision_ref.clone())
        .collect::<BTreeSet<_>>();

    let mut derived = graph.clone();
    let mut stale_semantic_refs = BTreeSet::new();
    for node in &mut derived.nodes {
        let stale = node
            .source_revision_refs
            .iter()
            .any(|revision| stale_revision_refs.contains(revision));
        if stale {
            stale_semantic_refs.insert(node.semantic_ref.clone());
            // Spans are manifestation-relative.  Once the pinned manifestation
            // changes, neither the old revision nor its old span may continue
            // paying consumer provenance.
            node.source_revision_refs.clear();
            node.span_refs.clear();
        }
    }

    let mut digest_parts = vec![
        "revision-invalidated-projection:v1".to_owned(),
        graph.deterministic_digest.clone(),
        invalidation.from_world_ref.clone(),
        invalidation.to_world_ref.clone(),
    ];
    digest_parts.extend(stale_revision_refs.iter().cloned());
    digest_parts.extend(stale_semantic_refs.iter().cloned());
    let refs = digest_parts.iter().map(String::as_str).collect::<Vec<_>>();
    derived.deterministic_digest = sha256(&refs);
    derived.projection_only = true;
    derived.creates_semantic_authority = false;

    Ok(RevisionInvalidatedProjection {
        graph: derived,
        stale_semantic_refs,
        stale_revision_refs,
        source_projection_digest: graph.deterministic_digest.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionReopenedConsumerResearch {
    pub invalidation: RevisionInvalidationReceipt,
    pub affected_proof_cone: AffectedProofCone,
    pub rereview_plan: RevisionReReviewPlan,
    pub invalidated_projection: RevisionInvalidatedProjection,
    pub adequacy: ConsumerAdequacyCompilation,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compile_revision_reopened_consumer_research(
    old_world: &LegalWorldCoordinate,
    new_world: &LegalWorldCoordinate,
    dependencies: &RevisionDependencyIndex,
    demand: &ConsumerQueryDemand,
    graph: &ProjectionGraph,
    coverage: &ConsumerCoverage,
    operational_state: OperationalResearchState,
    formal_adequacy: Option<&KernelCheckedFactorsThroughWitness>,
    nonfactorability_witnesses: &[NonFactorabilityWitnessReceipt],
) -> Result<RevisionReopenedConsumerResearch, String> {
    let invalidation = diff_world_revisions(old_world, new_world)?;
    let affected_proof_cone = affected_proof_cone(&invalidation, dependencies)?;
    let rereview_plan = revision_rereview_plan(&invalidation, &affected_proof_cone)?;
    let invalidated_projection = invalidate_projection_for_revision(graph, &invalidation)?;

    // A FactorsThrough witness is bound to the derived projection digest.
    // Passing a witness for the old projection therefore fails in the ordinary
    // S15.6 certifier rather than silently surviving the world change.
    let adequacy = compile_consumer_adequacy(
        demand,
        &invalidated_projection.graph,
        coverage,
        operational_state,
        formal_adequacy,
        nonfactorability_witnesses,
    )?;

    Ok(RevisionReopenedConsumerResearch {
        invalidation,
        affected_proof_cone,
        rereview_plan,
        invalidated_projection,
        adequacy,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConsumerAdequacyCompilation, ConsumerAxis, ConsumerResearchDemandKind,
        NonFactorabilityWitnessReceipt, ProjectionKind, ProjectionNode,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn world(name: &str, revision: &str) -> LegalWorldCoordinate {
        LegalWorldCoordinate {
            world_ref: name.into(),
            matter_ref: "matter:mabo-fixture".into(),
            jurisdiction_ref: "AU".into(),
            as_at: "2026-09-21".into(),
            source_revisions: BTreeMap::from([("source:case".into(), revision.into())]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn graph() -> ProjectionGraph {
        ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![ProjectionNode {
                semantic_ref: "prop:mabo".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:old".into()],
                source_revision_refs: vec!["rev:1".into()],
                span_refs: vec!["span:old".into()],
                projection_role: "IssueProof".into(),
            }],
            edges: vec![],
            deterministic_digest: "sha256:old-projection".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn demand() -> ConsumerQueryDemand {
        ConsumerQueryDemand {
            query_ref: "query:mabo".into(),
            required_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
            ]),
            required_semantic_refs: BTreeSet::from(["prop:mabo".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        }
    }

    fn dependencies() -> RevisionDependencyIndex {
        RevisionDependencyIndex {
            source_to_propositions: BTreeMap::from([(
                "source:case".into(),
                BTreeSet::from(["prop:mabo".into()]),
            )]),
            proposition_dependents: BTreeMap::from([(
                "prop:mabo".into(),
                BTreeSet::from(["proof:mabo-query".into()]),
            )]),
        }
    }

    #[test]
    fn changed_revision_removes_old_projection_payment_and_reopens_closed_frontier() {
        let old = world("world:old", "rev:1");
        let new = world("world:new", "rev:2");
        let invalidation = diff_world_revisions(&old, &new).unwrap();
        let invalidated = invalidate_projection_for_revision(&graph(), &invalidation).unwrap();

        assert_eq!(
            invalidated.stale_semantic_refs,
            BTreeSet::from(["prop:mabo".into()])
        );
        assert!(invalidated.graph.nodes[0].source_revision_refs.is_empty());
        assert!(invalidated.graph.nodes[0].span_refs.is_empty());
        assert_ne!(
            invalidated.graph.deterministic_digest,
            "sha256:old-projection"
        );

        let result = compile_revision_reopened_consumer_research(
            &old,
            &new,
            &dependencies(),
            &demand(),
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        let ConsumerAdequacyCompilation::NeedsResearch(research) = result.adequacy else {
            panic!("stale projection coordinates must reopen consumer research");
        };
        assert!(research.operational_frontier_was_closed);
        assert!(research
            .runtime_receipt
            .missing_axes
            .contains(&ConsumerAxis::SourceRevision));
        assert!(research
            .runtime_receipt
            .missing_axes
            .contains(&ConsumerAxis::SourceSpan));
        assert!(research
            .runtime_receipt
            .missing_axes
            .contains(&ConsumerAxis::Provenance));
        assert!(!result.creates_claim_truth);
    }

    #[test]
    fn exact_revision_defect_can_become_exact_source_residual() {
        let old = world("world:old", "rev:1");
        let new = world("world:new", "rev:2");
        let invalidation = diff_world_revisions(&old, &new).unwrap();
        let invalidated = invalidate_projection_for_revision(&graph(), &invalidation).unwrap();
        let defect = NonFactorabilityWitnessReceipt {
            query_ref: "query:mabo".into(),
            projection_digest: invalidated.graph.deterministic_digest.clone(),
            lost_axis: ConsumerAxis::SourceRevision,
            left_world_ref: "world:old".into(),
            right_world_ref: "world:new".into(),
            shared_projection_ref: "projection:stale-source-erased".into(),
            left_answer_ref: "answer:old-source".into(),
            right_answer_ref: "answer:new-source".into(),
            theorem_module_ref: "DASHI.Law.ClosedIsNotAdequateExact".into(),
            theorem_ref: "revisionSourceDefect".into(),
            theorem_artifact_digest:
                "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                    .into(),
            exact_fibre_collision: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };

        let result = compile_revision_reopened_consumer_research(
            &old,
            &new,
            &dependencies(),
            &demand(),
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[defect],
        )
        .unwrap();
        let ConsumerAdequacyCompilation::NeedsResearch(research) = result.adequacy else {
            panic!("revision defect must reopen research");
        };
        assert!(research.reopened_by_nonfactorability);
        assert!(research.exact_residuals.iter().any(|residual| {
            residual.lost_axis == ConsumerAxis::SourceRevision
                && residual.demand.kind == ConsumerResearchDemandKind::AcquireSource
        }));
    }
}
