//! Query-scoped revision impact.
//!
//! The graph-wide revision bridge is deliberately conservative.  This module
//! adds the consumer-relative slice needed for FactorsThrough-style reasoning:
//! only source/proposition/proof coordinates declared as dependencies of Q may
//! change Q's projection digest or reopen research for Q.
//!
//! An unrelated world change remains visible in the world-level invalidation
//! receipt, but is classified as consumer-invariant.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::{
    affected_proof_cone, compile_consumer_adequacy, diff_world_revisions,
    ConsumerAdequacyCompilation, ConsumerAxis, ConsumerCoverage, ConsumerQueryDemand,
    KernelCheckedFactorsThroughWitness, KernelCheckedNonFactorabilityWitness,
    LegalWorldCoordinate, OperationalResearchState, ProjectionEdge, ProjectionGraph,
    ProjectionNode, RevisionDependencyIndex, RevisionInvalidationReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryDependencySlice {
    pub query_ref: String,
    pub required_axes: BTreeSet<ConsumerAxis>,
    pub semantic_refs: BTreeSet<String>,
    pub proof_refs: BTreeSet<String>,
    pub source_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl QueryDependencySlice {
    pub fn from_demand(demand: &ConsumerQueryDemand) -> Result<Self, String> {
        if demand.query_ref.trim().is_empty()
            || !demand.candidate_only
            || demand.creates_semantic_authority
        {
            return Err("query dependency slice requires a candidate-only demand".into());
        }
        Ok(Self {
            query_ref: demand.query_ref.clone(),
            required_axes: demand.required_axes.clone(),
            semantic_refs: demand.required_semantic_refs.clone(),
            proof_refs: BTreeSet::new(),
            source_refs: BTreeSet::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.query_ref.trim().is_empty()
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("query dependency slice crossed non-promotion boundary".into());
        }
        if self
            .semantic_refs
            .iter()
            .chain(self.proof_refs.iter())
            .chain(self.source_refs.iter())
            .any(|reference| reference.trim().is_empty())
        {
            return Err("query dependency slice contains an empty coordinate".into());
        }
        Ok(())
    }

    #[must_use]
    pub fn all_semantic_or_proof_refs(&self) -> BTreeSet<String> {
        self.semantic_refs
            .union(&self.proof_refs)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryScopedProjection {
    pub query_ref: String,
    pub graph: ProjectionGraph,
    pub requested_semantic_refs: BTreeSet<String>,
    pub missing_semantic_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn digest_parts(parts: &[String]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub fn compile_query_scoped_projection(
    graph: &ProjectionGraph,
    slice: &QueryDependencySlice,
) -> Result<QueryScopedProjection, String> {
    slice.validate()?;
    if !graph.projection_only || graph.creates_semantic_authority {
        return Err("query-scoped projection requires projection-only graph".into());
    }

    let requested = slice.all_semantic_or_proof_refs();
    let mut nodes = graph
        .nodes
        .iter()
        .filter(|node| requested.contains(&node.semantic_ref))
        .cloned()
        .collect::<Vec<ProjectionNode>>();
    nodes.sort_by(|left, right| left.semantic_ref.cmp(&right.semantic_ref));

    let visible = nodes
        .iter()
        .map(|node| node.semantic_ref.clone())
        .collect::<BTreeSet<_>>();
    let missing_semantic_refs = requested
        .difference(&visible)
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut edges = graph
        .edges
        .iter()
        .filter(|edge| visible.contains(&edge.from_ref) && visible.contains(&edge.to_ref))
        .cloned()
        .collect::<Vec<ProjectionEdge>>();
    edges.sort_by(|left, right| {
        (&left.from_ref, &left.to_ref, &left.relation)
            .cmp(&(&right.from_ref, &right.to_ref, &right.relation))
    });

    let mut parts = vec![
        "query-scoped-projection:v1".to_owned(),
        slice.query_ref.clone(),
        format!("{:?}", graph.kind),
    ];
    parts.extend(
        slice
            .required_axes
            .iter()
            .map(|axis| format!("axis:{axis:?}")),
    );
    parts.extend(
        slice
            .source_refs
            .iter()
            .map(|reference| format!("source:{reference}")),
    );
    parts.extend(nodes.iter().flat_map(|node| {
        [
            format!("node:{}", node.semantic_ref),
            format!("kind:{}", node.semantic_kind),
            format!("manifestations:{}", node.manifestation_refs.join(",")),
            format!("revisions:{}", node.source_revision_refs.join(",")),
            format!("spans:{}", node.span_refs.join(",")),
        ]
    }));
    parts.extend(edges.iter().map(|edge| {
        format!(
            "edge:{}:{}:{}",
            edge.from_ref, edge.to_ref, edge.relation
        )
    }));
    parts.extend(
        missing_semantic_refs
            .iter()
            .map(|reference| format!("missing:{reference}")),
    );

    Ok(QueryScopedProjection {
        query_ref: slice.query_ref.clone(),
        graph: ProjectionGraph {
            kind: graph.kind,
            nodes,
            edges,
            deterministic_digest: digest_parts(&parts),
            projection_only: true,
            creates_semantic_authority: false,
        },
        requested_semantic_refs: requested,
        missing_semantic_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryRevisionImpactKind {
    NoWorldRevisionChange,
    RevisionChangedConsumerInvariant,
    RevisionChangedConsumerRelevant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRevisionImpact {
    pub query_ref: String,
    pub kind: QueryRevisionImpactKind,
    pub invalidation: RevisionInvalidationReceipt,
    pub old_projection: QueryScopedProjection,
    pub new_projection: QueryScopedProjection,
    pub changed_source_refs: BTreeSet<String>,
    pub query_relevant_changed_source_refs: BTreeSet<String>,
    pub query_relevant_affected_refs: BTreeSet<String>,
    pub query_projection_digest_changed: bool,
    pub reopens_consumer_research: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compile_query_revision_impact(
    old_world: &LegalWorldCoordinate,
    new_world: &LegalWorldCoordinate,
    dependencies: &RevisionDependencyIndex,
    slice: &QueryDependencySlice,
    graph: &ProjectionGraph,
) -> Result<QueryRevisionImpact, String> {
    slice.validate()?;
    let invalidation = diff_world_revisions(old_world, new_world)?;
    let cone = affected_proof_cone(&invalidation, dependencies)?;
    let old_projection = compile_query_scoped_projection(graph, slice)?;

    let changed_source_refs = invalidation
        .changes
        .iter()
        .map(|change| change.source_ref.clone())
        .collect::<BTreeSet<_>>();
    let query_relevant_changed_source_refs = changed_source_refs
        .intersection(&slice.source_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let query_refs = slice.all_semantic_or_proof_refs();
    let query_relevant_affected_refs = cone
        .transitively_affected_refs
        .intersection(&query_refs)
        .cloned()
        .collect::<BTreeSet<_>>();

    let consumer_relevant = !query_relevant_changed_source_refs.is_empty()
        || !query_relevant_affected_refs.is_empty();

    // Only stale coordinates inside Q's declared dependency slice are removed.
    // Irrelevant world changes therefore leave the query projection byte-for-
    // byte/digest stable even though the world-level invalidation records them.
    let stale_old_revisions = invalidation
        .changes
        .iter()
        .filter(|change| {
            query_relevant_changed_source_refs.contains(&change.source_ref)
                || dependencies
                    .source_to_propositions
                    .get(&change.source_ref)
                    .is_some_and(|refs| {
                        refs.iter().any(|reference| query_refs.contains(reference))
                    })
        })
        .filter_map(|change| change.old_revision_ref.clone())
        .collect::<BTreeSet<_>>();

    let mut new_graph = graph.clone();
    if consumer_relevant {
        for node in &mut new_graph.nodes {
            let node_is_query_relevant = query_refs.contains(&node.semantic_ref);
            let stale = node.source_revision_refs.iter().any(|revision| {
                stale_old_revisions.contains(revision)
            });
            if node_is_query_relevant && stale {
                node.source_revision_refs.clear();
                node.span_refs.clear();
            }
        }
    }
    let new_projection = compile_query_scoped_projection(&new_graph, slice)?;
    let digest_changed =
        old_projection.graph.deterministic_digest != new_projection.graph.deterministic_digest;

    let kind = if invalidation.changes.is_empty() {
        QueryRevisionImpactKind::NoWorldRevisionChange
    } else if consumer_relevant {
        QueryRevisionImpactKind::RevisionChangedConsumerRelevant
    } else {
        QueryRevisionImpactKind::RevisionChangedConsumerInvariant
    };

    if kind == QueryRevisionImpactKind::RevisionChangedConsumerInvariant && digest_changed {
        return Err(
            "consumer-invariant revision unexpectedly changed query-scoped projection digest"
                .into(),
        );
    }

    Ok(QueryRevisionImpact {
        query_ref: slice.query_ref.clone(),
        kind,
        invalidation,
        old_projection,
        new_projection,
        changed_source_refs,
        query_relevant_changed_source_refs,
        query_relevant_affected_refs,
        query_projection_digest_changed: digest_changed,
        reopens_consumer_research:
            kind == QueryRevisionImpactKind::RevisionChangedConsumerRelevant,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}


#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryRevisionResearchOutcome {
    NoWorldRevisionChange {
        impact: QueryRevisionImpact,
        adequacy: ConsumerAdequacyCompilation,
    },
    RevisionChangedConsumerInvariant {
        impact: QueryRevisionImpact,
        adequacy: ConsumerAdequacyCompilation,
    },
    RevisionChangedConsumerResidual {
        impact: QueryRevisionImpact,
        adequacy: ConsumerAdequacyCompilation,
    },
}

pub fn compile_query_revision_research(
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
) -> Result<QueryRevisionResearchOutcome, String> {
    if slice.query_ref != demand.query_ref {
        return Err("query dependency slice does not match consumer demand query_ref".into());
    }
    if !demand
        .required_semantic_refs
        .is_subset(&slice.semantic_refs)
    {
        return Err(
            "query dependency slice omitted a semantic coordinate required by the demand".into(),
        );
    }
    if !demand.required_axes.is_subset(&slice.required_axes) {
        return Err("query dependency slice omitted an axis required by the demand".into());
    }

    let impact =
        compile_query_revision_impact(old_world, new_world, dependencies, slice, graph)?;

    let adequacy = compile_consumer_adequacy(
        demand,
        &impact.new_projection.graph,
        coverage,
        operational_state,
        formal_adequacy,
        nonfactorability_witnesses,
    )?;

    match impact.kind {
        QueryRevisionImpactKind::NoWorldRevisionChange => {
            Ok(QueryRevisionResearchOutcome::NoWorldRevisionChange {
                impact,
                adequacy,
            })
        }
        QueryRevisionImpactKind::RevisionChangedConsumerInvariant => {
            if impact.query_projection_digest_changed || impact.reopens_consumer_research {
                return Err(
                    "consumer-invariant revision may not mutate query projection or reopen research"
                        .into(),
                );
            }
            Ok(QueryRevisionResearchOutcome::RevisionChangedConsumerInvariant {
                impact,
                adequacy,
            })
        }
        QueryRevisionImpactKind::RevisionChangedConsumerRelevant => {
            if !impact.reopens_consumer_research {
                return Err(
                    "consumer-relevant revision must reopen consumer research".into(),
                );
            }
            Ok(QueryRevisionResearchOutcome::RevisionChangedConsumerResidual {
                impact,
                adequacy,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProjectionKind, ProjectionNode};
    use std::collections::BTreeMap;

    fn world(name: &str, a: &str, b: &str) -> LegalWorldCoordinate {
        LegalWorldCoordinate {
            world_ref: name.into(),
            matter_ref: "matter:fixture".into(),
            jurisdiction_ref: "AU".into(),
            as_at: "2026-09-21".into(),
            source_revisions: BTreeMap::from([
                ("source:a".into(), a.into()),
                ("source:b".into(), b.into()),
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
                    semantic_ref: "prop:irrelevant".into(),
                    semantic_kind: "Proposition".into(),
                    manifestation_refs: vec!["manifestation:b".into()],
                    source_revision_refs: vec!["rev:b:1".into()],
                    span_refs: vec!["span:b".into()],
                    projection_role: "IssueProof".into(),
                },
            ],
            edges: vec![],
            deterministic_digest: "sha256:whole-graph".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn slice() -> QueryDependencySlice {
        QueryDependencySlice {
            query_ref: "query:q".into(),
            required_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
            ]),
            semantic_refs: BTreeSet::from(["prop:q".into()]),
            proof_refs: BTreeSet::from(["proof:q".into()]),
            source_refs: BTreeSet::from(["source:a".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn dependencies() -> RevisionDependencyIndex {
        RevisionDependencyIndex {
            source_to_propositions: [
                ("source:a".into(), BTreeSet::from(["prop:q".into()])),
                (
                    "source:b".into(),
                    BTreeSet::from(["prop:irrelevant".into()]),
                ),
            ]
            .into_iter()
            .collect(),
            proposition_dependents: [
                ("prop:q".into(), BTreeSet::from(["proof:q".into()])),
                (
                    "prop:irrelevant".into(),
                    BTreeSet::from(["proof:irrelevant".into()]),
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn irrelevant_revision_change_preserves_query_projection_digest() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:1", "rev:b:2");
        let impact =
            compile_query_revision_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();

        assert_eq!(
            impact.kind,
            QueryRevisionImpactKind::RevisionChangedConsumerInvariant
        );
        assert!(!impact.reopens_consumer_research);
        assert!(!impact.query_projection_digest_changed);
        assert_eq!(
            impact.old_projection.graph.deterministic_digest,
            impact.new_projection.graph.deterministic_digest
        );
        assert_eq!(
            impact.changed_source_refs,
            BTreeSet::from(["source:b".into()])
        );
        assert!(impact.query_relevant_changed_source_refs.is_empty());
        assert!(impact.query_relevant_affected_refs.is_empty());
    }

    #[test]
    fn relevant_revision_change_invalidates_only_query_slice_and_reopens() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:2", "rev:b:1");
        let impact =
            compile_query_revision_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();

        assert_eq!(
            impact.kind,
            QueryRevisionImpactKind::RevisionChangedConsumerRelevant
        );
        assert!(impact.reopens_consumer_research);
        assert!(impact.query_projection_digest_changed);
        assert_eq!(
            impact.query_relevant_changed_source_refs,
            BTreeSet::from(["source:a".into()])
        );
        assert!(impact
            .query_relevant_affected_refs
            .contains("prop:q"));
        assert!(impact
            .query_relevant_affected_refs
            .contains("proof:q"));
        assert!(impact.new_projection.graph.nodes[0]
            .source_revision_refs
            .is_empty());
        // The unrelated graph node is outside the query-scoped projection.
        assert_eq!(impact.new_projection.graph.nodes.len(), 1);
    }

    #[test]
    fn no_world_revision_change_is_not_a_reopen() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:1", "rev:b:1");
        let impact =
            compile_query_revision_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();
        assert_eq!(impact.kind, QueryRevisionImpactKind::NoWorldRevisionChange);
        assert!(!impact.reopens_consumer_research);
        assert!(!impact.query_projection_digest_changed);
    }

    #[test]
    fn invariant_world_change_preserves_query_digest_and_does_not_reopen() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:1", "rev:b:2");
        let demand = ConsumerQueryDemand {
            query_ref: "query:q".into(),
            required_axes: slice().required_axes.clone(),
            required_semantic_refs: BTreeSet::from(["prop:q".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let outcome = compile_query_revision_research(
            &old,
            &new,
            &dependencies(),
            &slice(),
            &demand,
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        let QueryRevisionResearchOutcome::RevisionChangedConsumerInvariant {
            impact,
            adequacy: _,
        } = outcome
        else {
            panic!("unrelated source revision must be consumer invariant");
        };
        assert!(!impact.query_projection_digest_changed);
        assert!(!impact.reopens_consumer_research);
    }

    #[test]
    fn relevant_world_change_reopens_query_scoped_research() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:2", "rev:b:1");
        let demand = ConsumerQueryDemand {
            query_ref: "query:q".into(),
            required_axes: slice().required_axes.clone(),
            required_semantic_refs: BTreeSet::from(["prop:q".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let outcome = compile_query_revision_research(
            &old,
            &new,
            &dependencies(),
            &slice(),
            &demand,
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();

        let QueryRevisionResearchOutcome::RevisionChangedConsumerResidual {
            impact,
            adequacy,
        } = outcome
        else {
            panic!("query-relevant source revision must reopen research");
        };
        assert!(impact.query_projection_digest_changed);
        assert!(impact.reopens_consumer_research);
        let ConsumerAdequacyCompilation::NeedsResearch(research) = adequacy else {
            panic!("stale query source coordinate must require research");
        };
        assert!(research
            .runtime_receipt
            .missing_axes
            .contains(&ConsumerAxis::SourceRevision));
    }

}