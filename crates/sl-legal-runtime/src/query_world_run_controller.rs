//! Query-world autonomous research controller.
//!
//! This module is the single policy seam above:
//!   * query-scoped world/revision impact,
//!   * world-relative coverage invalidation,
//!   * stale formal witness filtering, and
//!   * theorem-bearing consumer adequacy.
//!
//! It does not acquire sources or fabricate reviewed deltas.  It decides
//! whether an automated run may preserve operational closure, must reopen
//! exact research, needs a fresh adequacy proof, or has a current
//! theorem-backed ConsumerAdequate receipt.

use serde::{Deserialize, Serialize};

use crate::{
    compile_query_world_research, ConsumerAdequacyCompilation, ConsumerCoverage,
    ConsumerQueryDemand, ExactConsumerResidual, ExplicitlyUnresolvedCompilation,
    KernelCheckedFactorsThroughWitness, KernelCheckedNonFactorabilityWitness,
    LegalWorldCoordinate, NeedsResearchCompilation, OperationalResearchState,
    ProjectionGraph, QueryDependencySlice, QueryWorldFormalWitnessDisposition,
    QueryWorldImpact, QueryWorldResearchOutcome, RevisionDependencyIndex,
    TheoremBackedConsumerAdequacyReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryWorldRunDecisionKind {
    PreserveOperationalClosureByInvariance,
    ReopenExactResearch,
    RequireFreshAdequacyWitness,
    ConsumerAdequate,
    ExplicitlyUnresolved,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryWorldRunDecision {
    pub kind: QueryWorldRunDecisionKind,
    pub query_ref: String,
    pub impact: QueryWorldImpact,
    pub formal_witnesses: QueryWorldFormalWitnessDisposition,
    pub exact_residuals: Vec<ExactConsumerResidual>,
    pub unproved_demand_reason_refs: Vec<String>,
    pub theorem_adequacy: Option<TheoremBackedConsumerAdequacyReceipt>,
    pub explicitly_unresolved: Option<ExplicitlyUnresolvedCompilation>,
    pub operational_frontier_closed: bool,
    pub run_may_stop: bool,
    pub consumer_adequate_formally_proved: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn parts(
    outcome: QueryWorldResearchOutcome,
) -> (
    QueryWorldImpact,
    ConsumerAdequacyCompilation,
    QueryWorldFormalWitnessDisposition,
    bool,
) {
    match outcome {
        QueryWorldResearchOutcome::NoWorldCoordinateChange {
            impact,
            adequacy,
            formal_witnesses,
        } => (impact, adequacy, formal_witnesses, false),
        QueryWorldResearchOutcome::WorldChangedConsumerInvariant {
            impact,
            adequacy,
            formal_witnesses,
        } => (impact, adequacy, formal_witnesses, true),
        QueryWorldResearchOutcome::WorldChangedConsumerResidual {
            impact,
            adequacy,
            formal_witnesses,
        } => (impact, adequacy, formal_witnesses, false),
    }
}

fn decision_from_adequacy(
    impact: QueryWorldImpact,
    formal_witnesses: QueryWorldFormalWitnessDisposition,
    world_change_consumer_invariant: bool,
    adequacy: ConsumerAdequacyCompilation,
    operational_state: OperationalResearchState,
) -> Result<QueryWorldRunDecision, String> {
    let query_ref = impact.query_ref.clone();

    let (
        kind,
        exact_residuals,
        unproved_demand_reason_refs,
        theorem_adequacy,
        explicitly_unresolved,
        operational_frontier_closed,
        run_may_stop,
        consumer_adequate_formally_proved,
    ) = match adequacy {
        ConsumerAdequacyCompilation::ConsumerAdequate(receipt) => {
            if !receipt.consumer_adequate || !receipt.factors_through_formally_proved {
                return Err(
                    "ConsumerAdequate compilation lacked theorem-backed factorisation".into(),
                );
            }
            (
                QueryWorldRunDecisionKind::ConsumerAdequate,
                Vec::new(),
                Vec::new(),
                Some(receipt),
                None,
                operational_state == OperationalResearchState::CurrentFrontierClosed,
                true,
                true,
            )
        }
        ConsumerAdequacyCompilation::NeedsResearch(NeedsResearchCompilation {
            exact_residuals,
            unproved_demand_reason_refs,
            ..
        }) => (
            QueryWorldRunDecisionKind::ReopenExactResearch,
            exact_residuals,
            unproved_demand_reason_refs,
            None,
            None,
            false,
            false,
            false,
        ),
        ConsumerAdequacyCompilation::ExplicitlyUnresolved(receipt) => {
            let closed = receipt.operational_frontier_closed;
            (
                QueryWorldRunDecisionKind::ExplicitlyUnresolved,
                Vec::new(),
                Vec::new(),
                None,
                Some(receipt),
                closed,
                true,
                false,
            )
        }
        ConsumerAdequacyCompilation::CurrentFrontierClosedWithoutAdequacy(receipt) => {
            let kind = if world_change_consumer_invariant {
                QueryWorldRunDecisionKind::PreserveOperationalClosureByInvariance
            } else {
                QueryWorldRunDecisionKind::RequireFreshAdequacyWitness
            };
            (
                kind,
                Vec::new(),
                Vec::new(),
                None,
                None,
                true,
                true,
                false,
            )
        }
        ConsumerAdequacyCompilation::BudgetExhaustedWithoutAdequacy(_) => (
            QueryWorldRunDecisionKind::BudgetExhausted,
            Vec::new(),
            Vec::new(),
            None,
            None,
            false,
            true,
            false,
        ),
        ConsumerAdequacyCompilation::AdequacyProofRequired { runtime_receipt, .. } => (
            QueryWorldRunDecisionKind::RequireFreshAdequacyWitness,
            Vec::new(),
            Vec::new(),
            None,
            None,
            operational_state == OperationalResearchState::CurrentFrontierClosed
                && runtime_receipt.research_demands.is_empty(),
            operational_state == OperationalResearchState::CurrentFrontierClosed,
            false,
        ),
    };

    if kind == QueryWorldRunDecisionKind::PreserveOperationalClosureByInvariance {
        if impact.reopens_consumer_research
            || impact.query_projection_digest_changed
            || formal_witnesses.positive_witness_stale
            || formal_witnesses.stale_nonfactorability_witness_count > 0
        {
            return Err(
                "cannot preserve operational closure across a non-invariant query-world transition"
                    .into(),
            );
        }
    }

    if kind == QueryWorldRunDecisionKind::ReopenExactResearch
        && exact_residuals.is_empty()
        && unproved_demand_reason_refs.is_empty()
    {
        return Err("reopened research has neither exact nor unresolved demands".into());
    }

    Ok(QueryWorldRunDecision {
        kind,
        query_ref,
        impact,
        formal_witnesses,
        exact_residuals,
        unproved_demand_reason_refs,
        theorem_adequacy,
        explicitly_unresolved,
        operational_frontier_closed,
        run_may_stop,
        consumer_adequate_formally_proved,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn decide_query_world_run(
    old_world: &LegalWorldCoordinate,
    new_world: &LegalWorldCoordinate,
    dependencies: &RevisionDependencyIndex,
    slice: &QueryDependencySlice,
    demand: &ConsumerQueryDemand,
    graph: &ProjectionGraph,
    coverage: &ConsumerCoverage,
    operational_state: OperationalResearchState,
    formal_adequacy: Option<&KernelCheckedFactorsThroughWitness>,
    nonfactorability_witnesses: &[KernelCheckedNonFactorabilityWitness],
) -> Result<QueryWorldRunDecision, String> {
    let outcome = compile_query_world_research(
        old_world,
        new_world,
        dependencies,
        slice,
        demand,
        graph,
        coverage,
        operational_state,
        formal_adequacy,
        nonfactorability_witnesses,
    )?;
    let (impact, adequacy, formal_witnesses, invariant) = parts(outcome);
    decision_from_adequacy(
        impact,
        formal_witnesses,
        invariant,
        adequacy,
        operational_state,
    )
}



pub fn query_world_run_controller_self_check() -> Result<(), String> {
    use crate::{
        kernel_checked_factors_through_witness, AgdaFactorsThroughTypecheckReceipt,
        ConsumerAxis, ProjectionKind, ProjectionNode,
    };
    use std::collections::{BTreeMap, BTreeSet};

    let world = |world_ref: &str, a_revision: &str, b_revision: &str| {
        LegalWorldCoordinate {
            world_ref: world_ref.into(),
            matter_ref: "matter:query-world-controller-self-check".into(),
            jurisdiction_ref: "AU".into(),
            as_at: "2026-09-21".into(),
            source_revisions: BTreeMap::from([
                ("source:a".into(), a_revision.into()),
                ("source:b".into(), b_revision.into()),
            ]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    };
    let graph = ProjectionGraph {
        kind: ProjectionKind::IssueProof,
        nodes: vec![
            ProjectionNode {
                semantic_ref: "prop:q".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:q".into()],
                source_revision_refs: vec!["rev:a:1".into()],
                span_refs: vec!["span:q".into()],
                projection_role: "IssueProof".into(),
            },
            ProjectionNode {
                semantic_ref: "prop:other".into(),
                semantic_kind: "Proposition".into(),
                manifestation_refs: vec!["manifestation:other".into()],
                source_revision_refs: vec!["rev:b:1".into()],
                span_refs: vec!["span:other".into()],
                projection_role: "IssueProof".into(),
            },
        ],
        edges: Vec::new(),
        deterministic_digest: "sha256:query-world-controller-self-check".into(),
        projection_only: true,
        creates_semantic_authority: false,
    };
    let dependencies = RevisionDependencyIndex {
        source_to_propositions: BTreeMap::from([
            ("source:a".into(), BTreeSet::from(["prop:q".into()])),
            ("source:b".into(), BTreeSet::from(["prop:other".into()])),
        ]),
        proposition_dependents: BTreeMap::new(),
    };
    let axes = BTreeSet::from([
        ConsumerAxis::SemanticIdentity,
        ConsumerAxis::SourceRevision,
        ConsumerAxis::SourceSpan,
        ConsumerAxis::Provenance,
    ]);
    let slice = QueryDependencySlice {
        query_ref: "query:q".into(),
        required_axes: axes.clone(),
        semantic_refs: BTreeSet::from(["prop:q".into()]),
        proof_refs: BTreeSet::new(),
        source_refs: BTreeSet::from(["source:a".into()]),
        source_revision_refs: BTreeSet::from(["rev:a:1".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    let demand = ConsumerQueryDemand {
        query_ref: "query:q".into(),
        required_axes: axes.clone(),
        required_semantic_refs: BTreeSet::from(["prop:q".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
    };
    let coverage = ConsumerCoverage {
        paid_axes: axes,
        ..ConsumerCoverage::default()
    };

    let old = world("world:old", "rev:a:1", "rev:b:1");

    let irrelevant = world("world:irrelevant", "rev:a:1", "rev:b:2");
    let invariant = decide_query_world_run(
        &old,
        &irrelevant,
        &dependencies,
        &slice,
        &demand,
        &graph,
        &coverage,
        OperationalResearchState::CurrentFrontierClosed,
        None,
        &[],
    )?;
    if invariant.kind
        != QueryWorldRunDecisionKind::PreserveOperationalClosureByInvariance
        || !invariant.operational_frontier_closed
        || !invariant.run_may_stop
        || invariant.consumer_adequate_formally_proved
    {
        return Err("query-world controller self-check failed invariant closure path".into());
    }

    let relevant = world("world:relevant", "rev:a:2", "rev:b:1");
    let reopened = decide_query_world_run(
        &old,
        &relevant,
        &dependencies,
        &slice,
        &demand,
        &graph,
        &coverage,
        OperationalResearchState::CurrentFrontierClosed,
        None,
        &[],
    )?;
    if reopened.kind != QueryWorldRunDecisionKind::ReopenExactResearch
        || reopened.operational_frontier_closed
        || reopened.run_may_stop
        || reopened.consumer_adequate_formally_proved
    {
        return Err("query-world controller self-check failed relevant reopening path".into());
    }

    let same_impact = crate::compile_query_world_impact(
        &old,
        &old,
        &dependencies,
        &slice,
        &graph,
    )?;
    let digest = same_impact.new_projection.graph.deterministic_digest.clone();
    let witness = kernel_checked_factors_through_witness(
        &AgdaFactorsThroughTypecheckReceipt {
            schema_version: "sl.formal.agda_factors_through_typecheck.v0_1".into(),
            verifier: "agda".into(),
            command_ref:
                "agda -i . DASHI/Law/QueryWorldAutonomousRunControllerExact.agda".into(),
            exit_code: 0,
            query_ref: "query:q".into(),
            projection_digest: digest,
            theorem_module_ref: "DASHI.Law.QueryWorldAutonomousRunControllerExact".into(),
            theorem_ref: "freshFactorsThroughMayCertifyCurrentWorld".into(),
            theorem_artifact_digest:
                "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
                    .into(),
            exact_query_indexed: true,
            factors_through_claim: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
    )?;
    let adequate = decide_query_world_run(
        &old,
        &old,
        &dependencies,
        &slice,
        &demand,
        &graph,
        &coverage,
        OperationalResearchState::CurrentFrontierClosed,
        Some(&witness),
        &[],
    )?;
    if adequate.kind != QueryWorldRunDecisionKind::ConsumerAdequate
        || !adequate.run_may_stop
        || !adequate.consumer_adequate_formally_proved
    {
        return Err("query-world controller self-check failed theorem adequacy path".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        kernel_checked_factors_through_witness, AgdaFactorsThroughTypecheckReceipt,
        ConsumerAxis, ProjectionKind, ProjectionNode,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn world(
        world_ref: &str,
        as_at: &str,
        jurisdiction: &str,
        a_revision: &str,
        b_revision: &str,
    ) -> LegalWorldCoordinate {
        LegalWorldCoordinate {
            world_ref: world_ref.into(),
            matter_ref: "matter:controller".into(),
            jurisdiction_ref: jurisdiction.into(),
            as_at: as_at.into(),
            source_revisions: BTreeMap::from([
                ("source:a".into(), a_revision.into()),
                ("source:b".into(), b_revision.into()),
            ]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn graph() -> ProjectionGraph {
        ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![
                ProjectionNode {
                    semantic_ref: "prop:q".into(),
                    semantic_kind: "Proposition".into(),
                    manifestation_refs: vec!["manifestation:q".into()],
                    source_revision_refs: vec!["rev:a:1".into()],
                    span_refs: vec!["span:q".into()],
                    projection_role: "IssueProof".into(),
                },
                ProjectionNode {
                    semantic_ref: "prop:other".into(),
                    semantic_kind: "Proposition".into(),
                    manifestation_refs: vec!["manifestation:other".into()],
                    source_revision_refs: vec!["rev:b:1".into()],
                    span_refs: vec!["span:other".into()],
                    projection_role: "IssueProof".into(),
                },
            ],
            edges: Vec::new(),
            deterministic_digest: "sha256:controller-base".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn dependencies() -> RevisionDependencyIndex {
        RevisionDependencyIndex {
            source_to_propositions: BTreeMap::from([
                ("source:a".into(), BTreeSet::from(["prop:q".into()])),
                ("source:b".into(), BTreeSet::from(["prop:other".into()])),
            ]),
            proposition_dependents: BTreeMap::new(),
        }
    }

    fn slice(required_axes: BTreeSet<ConsumerAxis>) -> QueryDependencySlice {
        QueryDependencySlice {
            query_ref: "query:q".into(),
            required_axes,
            semantic_refs: BTreeSet::from(["prop:q".into()]),
            proof_refs: BTreeSet::new(),
            source_refs: BTreeSet::from(["source:a".into()]),
            source_revision_refs: BTreeSet::from(["rev:a:1".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn demand(required_axes: BTreeSet<ConsumerAxis>) -> ConsumerQueryDemand {
        ConsumerQueryDemand {
            query_ref: "query:q".into(),
            required_axes,
            required_semantic_refs: BTreeSet::from(["prop:q".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        }
    }

    fn base_axes() -> BTreeSet<ConsumerAxis> {
        BTreeSet::from([
            ConsumerAxis::SemanticIdentity,
            ConsumerAxis::SourceRevision,
            ConsumerAxis::SourceSpan,
            ConsumerAxis::Provenance,
        ])
    }

    #[test]
    fn irrelevant_world_change_preserves_only_operational_closure() {
        let old = world("world:old", "2026-09-21", "AU", "rev:a:1", "rev:b:1");
        let new = world("world:new", "2026-09-21", "AU", "rev:a:1", "rev:b:2");
        let axes = base_axes();
        let coverage = ConsumerCoverage {
            paid_axes: axes.clone(),
            ..ConsumerCoverage::default()
        };

        let decision = decide_query_world_run(
            &old,
            &new,
            &dependencies(),
            &slice(axes.clone()),
            &demand(axes),
            &graph(),
            &coverage,
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        assert_eq!(
            decision.kind,
            QueryWorldRunDecisionKind::PreserveOperationalClosureByInvariance
        );
        assert!(decision.operational_frontier_closed);
        assert!(decision.run_may_stop);
        assert!(!decision.consumer_adequate_formally_proved);
        assert!(!decision.impact.query_projection_digest_changed);
    }

    #[test]
    fn relevant_revision_never_preserves_closed_frontier() {
        let old = world("world:old", "2026-09-21", "AU", "rev:a:1", "rev:b:1");
        let new = world("world:new", "2026-09-21", "AU", "rev:a:2", "rev:b:1");
        let axes = base_axes();
        let coverage = ConsumerCoverage {
            paid_axes: axes.clone(),
            ..ConsumerCoverage::default()
        };

        let decision = decide_query_world_run(
            &old,
            &new,
            &dependencies(),
            &slice(axes.clone()),
            &demand(axes),
            &graph(),
            &coverage,
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        assert_eq!(
            decision.kind,
            QueryWorldRunDecisionKind::ReopenExactResearch
        );
        assert!(!decision.operational_frontier_closed);
        assert!(!decision.run_may_stop);
        assert!(!decision.consumer_adequate_formally_proved);
    }

    #[test]
    fn same_world_without_formal_witness_requires_adequacy_witness() {
        let old = world("world:same", "2026-09-21", "AU", "rev:a:1", "rev:b:1");
        let axes = base_axes();
        let coverage = ConsumerCoverage {
            paid_axes: axes.clone(),
            ..ConsumerCoverage::default()
        };

        let decision = decide_query_world_run(
            &old,
            &old,
            &dependencies(),
            &slice(axes.clone()),
            &demand(axes),
            &graph(),
            &coverage,
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        assert_eq!(
            decision.kind,
            QueryWorldRunDecisionKind::RequireFreshAdequacyWitness
        );
        assert!(decision.operational_frontier_closed);
        assert!(decision.run_may_stop);
        assert!(!decision.consumer_adequate_formally_proved);
    }

    #[test]
    fn only_current_digest_kernel_receipt_can_emit_consumer_adequate() {
        let old = world("world:same", "2026-09-21", "AU", "rev:a:1", "rev:b:1");
        let axes = base_axes();
        let slice = slice(axes.clone());
        let demand = demand(axes.clone());
        let scoped = crate::compile_query_world_impact(
            &old,
            &old,
            &dependencies(),
            &slice,
            &graph(),
        )
        .unwrap();
        let digest = scoped.new_projection.graph.deterministic_digest.clone();

        let checked = kernel_checked_factors_through_witness(
            &AgdaFactorsThroughTypecheckReceipt {
                schema_version: "sl.formal.agda_factors_through_typecheck.v0_1".into(),
                verifier: "agda".into(),
                command_ref: "agda -i . DASHI/Law/QueryWorldControllerFixture.agda".into(),
                exit_code: 0,
                query_ref: "query:q".into(),
                projection_digest: digest,
                theorem_module_ref: "DASHI.Law.QueryWorldControllerFixture".into(),
                theorem_ref: "queryWorldFactorsThrough".into(),
                theorem_artifact_digest:
                    "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                        .into(),
                exact_query_indexed: true,
                factors_through_claim: true,
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            },
        )
        .unwrap();
        let coverage = ConsumerCoverage {
            paid_axes: axes,
            ..ConsumerCoverage::default()
        };

        let decision = decide_query_world_run(
            &old,
            &old,
            &dependencies(),
            &slice,
            &demand,
            &graph(),
            &coverage,
            OperationalResearchState::CurrentFrontierClosed,
            Some(&checked),
            &[],
        )
        .unwrap();

        assert_eq!(decision.kind, QueryWorldRunDecisionKind::ConsumerAdequate);
        assert!(decision.operational_frontier_closed);
        assert!(decision.run_may_stop);
        assert!(decision.consumer_adequate_formally_proved);
        assert!(decision.theorem_adequacy.is_some());
    }
}