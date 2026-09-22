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
    ExplanationIndex, KernelCheckedFactorsThroughWitness,
    KernelCheckedNonFactorabilityWitness, LegalWorldCoordinate,
    OperationalResearchState, ProjectionEdge, ProjectionGraph, ProjectionNode,
    RevisionDependencyIndex, RevisionInvalidationReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryDependencySlice {
    pub query_ref: String,
    pub required_axes: BTreeSet<ConsumerAxis>,
    pub semantic_refs: BTreeSet<String>,
    pub proof_refs: BTreeSet<String>,
    pub source_refs: BTreeSet<String>,
    pub source_revision_refs: BTreeSet<String>,
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
            source_revision_refs: BTreeSet::new(),
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
            .chain(self.source_revision_refs.iter())
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

pub fn compile_query_dependency_slice(
    demand: &ConsumerQueryDemand,
    index: &ExplanationIndex,
) -> Result<QueryDependencySlice, String> {
    let mut slice = QueryDependencySlice::from_demand(demand)?;
    index.validate()?;

    let mut seen = BTreeSet::new();
    let mut frontier = demand
        .required_semantic_refs
        .iter()
        .cloned()
        .collect::<Vec<_>>();

    while let Some(reference) = frontier.pop() {
        if !seen.insert(reference.clone()) {
            continue;
        }
        let Some(record) = index.records.get(&reference) else {
            // Missing required semantics remain visible to the adequacy
            // compiler; dependency discovery simply cannot expand through a
            // record that is not present.
            continue;
        };

        for provenance in &record.provenance {
            slice
                .source_revision_refs
                .insert(provenance.source_revision_ref.clone());
        }
        for dependency in &record.dependencies {
            if !seen.contains(dependency) {
                frontier.push(dependency.clone());
            }
        }
    }

    slice.proof_refs = seen
        .difference(&slice.semantic_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    slice.validate()?;
    Ok(slice)
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    parts.extend(
        slice
            .source_revision_refs
            .iter()
            .map(|reference| format!("source-revision:{reference}")),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRevisionImpact {
    pub query_ref: String,
    pub kind: QueryRevisionImpactKind,
    pub invalidation: RevisionInvalidationReceipt,
    pub old_projection: QueryScopedProjection,
    pub new_projection: QueryScopedProjection,
    pub changed_source_refs: BTreeSet<String>,
    pub query_relevant_changed_source_refs: BTreeSet<String>,
    pub query_relevant_changed_revision_refs: BTreeSet<String>,
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
    let query_relevant_changed_revision_refs = invalidation
        .changes
        .iter()
        .filter_map(|change| change.old_revision_ref.as_ref())
        .filter(|revision| slice.source_revision_refs.contains(*revision))
        .cloned()
        .collect::<BTreeSet<_>>();
    let query_refs = slice.all_semantic_or_proof_refs();
    let query_relevant_affected_refs = cone
        .transitively_affected_refs
        .intersection(&query_refs)
        .cloned()
        .collect::<BTreeSet<_>>();

    let consumer_relevant = !query_relevant_changed_source_refs.is_empty()
        || !query_relevant_changed_revision_refs.is_empty()
        || !query_relevant_affected_refs.is_empty();

    // Only stale coordinates inside Q's declared dependency slice are removed.
    // Irrelevant world changes therefore leave the query projection byte-for-
    // byte/digest stable even though the world-level invalidation records them.
    let stale_old_revisions = invalidation
        .changes
        .iter()
        .filter(|change| {
            query_relevant_changed_source_refs.contains(&change.source_ref)
                || change
                    .old_revision_ref
                    .as_ref()
                    .is_some_and(|revision| {
                        query_relevant_changed_revision_refs.contains(revision)
                    })
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
        query_relevant_changed_revision_refs,
        query_relevant_affected_refs,
        query_projection_digest_changed: digest_changed,
        reopens_consumer_research:
            kind == QueryRevisionImpactKind::RevisionChangedConsumerRelevant,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}


#[derive(Debug, Clone, PartialEq, Eq)]
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


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryScopedWorldProjection {
    pub query_ref: String,
    pub graph: ProjectionGraph,
    pub includes_temporal_coordinate: bool,
    pub includes_jurisdiction_coordinate: bool,
    pub as_at: Option<String>,
    pub jurisdiction_ref: Option<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn bind_query_projection_to_world(
    projection: &QueryScopedProjection,
    slice: &QueryDependencySlice,
    world: &LegalWorldCoordinate,
) -> Result<QueryScopedWorldProjection, String> {
    slice.validate()?;
    world.validate()?;
    if projection.query_ref != slice.query_ref {
        return Err("query-scoped projection does not match dependency slice".into());
    }

    let includes_temporal_coordinate =
        slice.required_axes.contains(&ConsumerAxis::Temporal);
    let includes_jurisdiction_coordinate =
        slice.required_axes.contains(&ConsumerAxis::Jurisdiction);

    let mut parts = vec![
        "query-scoped-world-projection:v1".to_owned(),
        projection.graph.deterministic_digest.clone(),
        slice.query_ref.clone(),
    ];
    if includes_temporal_coordinate {
        parts.push(format!("as-at:{}", world.as_at));
    }
    if includes_jurisdiction_coordinate {
        parts.push(format!("jurisdiction:{}", world.jurisdiction_ref));
    }

    let mut graph = projection.graph.clone();
    graph.deterministic_digest = digest_parts(&parts);

    Ok(QueryScopedWorldProjection {
        query_ref: slice.query_ref.clone(),
        graph,
        includes_temporal_coordinate,
        includes_jurisdiction_coordinate,
        as_at: includes_temporal_coordinate.then(|| world.as_at.clone()),
        jurisdiction_ref: includes_jurisdiction_coordinate
            .then(|| world.jurisdiction_ref.clone()),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryWorldImpactKind {
    NoWorldCoordinateChange,
    WorldChangedConsumerInvariant,
    WorldChangedConsumerRelevant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryWorldImpact {
    pub query_ref: String,
    pub kind: QueryWorldImpactKind,
    pub revision_impact: QueryRevisionImpact,
    pub old_projection: QueryScopedWorldProjection,
    pub new_projection: QueryScopedWorldProjection,
    pub temporal_changed: bool,
    pub temporal_relevant: bool,
    pub jurisdiction_changed: bool,
    pub jurisdiction_relevant: bool,
    pub query_projection_digest_changed: bool,
    pub reopens_consumer_research: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn compile_query_world_impact(
    old_world: &LegalWorldCoordinate,
    new_world: &LegalWorldCoordinate,
    dependencies: &RevisionDependencyIndex,
    slice: &QueryDependencySlice,
    graph: &ProjectionGraph,
) -> Result<QueryWorldImpact, String> {
    if old_world.matter_ref != new_world.matter_ref {
        return Err("query world impact requires the same matter_ref".into());
    }
    let revision_impact =
        compile_query_revision_impact(old_world, new_world, dependencies, slice, graph)?;

    let old_projection =
        bind_query_projection_to_world(&revision_impact.old_projection, slice, old_world)?;
    let new_projection =
        bind_query_projection_to_world(&revision_impact.new_projection, slice, new_world)?;

    let temporal_changed = old_world.as_at != new_world.as_at;
    let temporal_relevant =
        temporal_changed && slice.required_axes.contains(&ConsumerAxis::Temporal);
    let jurisdiction_changed =
        old_world.jurisdiction_ref != new_world.jurisdiction_ref;
    let jurisdiction_relevant = jurisdiction_changed
        && slice.required_axes.contains(&ConsumerAxis::Jurisdiction);

    let any_world_change = !revision_impact.invalidation.changes.is_empty()
        || temporal_changed
        || jurisdiction_changed;
    let consumer_relevant = revision_impact.reopens_consumer_research
        || temporal_relevant
        || jurisdiction_relevant;
    let digest_changed =
        old_projection.graph.deterministic_digest != new_projection.graph.deterministic_digest;

    let kind = if !any_world_change {
        QueryWorldImpactKind::NoWorldCoordinateChange
    } else if consumer_relevant {
        QueryWorldImpactKind::WorldChangedConsumerRelevant
    } else {
        QueryWorldImpactKind::WorldChangedConsumerInvariant
    };

    if kind == QueryWorldImpactKind::WorldChangedConsumerInvariant && digest_changed {
        return Err(
            "consumer-invariant world coordinate change unexpectedly changed query digest"
                .into(),
        );
    }
    if (temporal_relevant || jurisdiction_relevant) && !digest_changed {
        return Err(
            "consumer-required world coordinate changed without changing query digest".into(),
        );
    }

    Ok(QueryWorldImpact {
        query_ref: slice.query_ref.clone(),
        kind,
        revision_impact,
        old_projection,
        new_projection,
        temporal_changed,
        temporal_relevant,
        jurisdiction_changed,
        jurisdiction_relevant,
        query_projection_digest_changed: digest_changed,
        reopens_consumer_research: consumer_relevant,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}


pub fn invalidate_consumer_coverage_for_world_impact(
    coverage: &ConsumerCoverage,
    impact: &QueryWorldImpact,
) -> ConsumerCoverage {
    let mut next = coverage.clone();

    if !impact
        .revision_impact
        .query_relevant_changed_source_refs
        .is_empty()
        || !impact
            .revision_impact
            .query_relevant_changed_revision_refs
            .is_empty()
    {
        next.paid_axes.remove(&ConsumerAxis::SourceRevision);
        next.paid_axes.remove(&ConsumerAxis::SourceSpan);
        next.paid_axes.remove(&ConsumerAxis::Provenance);
    }
    if impact.temporal_relevant {
        next.paid_axes.remove(&ConsumerAxis::Temporal);
    }
    if impact.jurisdiction_relevant {
        next.paid_axes.remove(&ConsumerAxis::Jurisdiction);
    }

    next
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryWorldFormalWitnessDisposition {
    pub new_projection_digest: String,
    pub positive_witness_usable: bool,
    pub positive_witness_stale: bool,
    pub usable_nonfactorability_witness_count: usize,
    pub stale_nonfactorability_witness_count: usize,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryWorldResearchOutcome {
    NoWorldCoordinateChange {
        impact: QueryWorldImpact,
        adequacy: ConsumerAdequacyCompilation,
        formal_witnesses: QueryWorldFormalWitnessDisposition,
    },
    WorldChangedConsumerInvariant {
        impact: QueryWorldImpact,
        adequacy: ConsumerAdequacyCompilation,
        formal_witnesses: QueryWorldFormalWitnessDisposition,
    },
    WorldChangedConsumerResidual {
        impact: QueryWorldImpact,
        adequacy: ConsumerAdequacyCompilation,
        formal_witnesses: QueryWorldFormalWitnessDisposition,
    },
}

pub fn compile_query_world_research(
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
) -> Result<QueryWorldResearchOutcome, String> {
    if slice.query_ref != demand.query_ref {
        return Err("query dependency slice does not match consumer demand query_ref".into());
    }
    let impact =
        compile_query_world_impact(old_world, new_world, dependencies, slice, graph)?;
    let current_coverage =
        invalidate_consumer_coverage_for_world_impact(coverage, &impact);

    // Formal receipts are scoped to the exact query projection digest.  A
    // world transition may make a previously valid receipt stale; staleness is
    // an ordinary revision outcome, not a controller error.
    let new_digest = impact.new_projection.graph.deterministic_digest.as_str();
    let usable_positive = formal_adequacy.filter(|witness| {
        let metadata = witness.metadata();
        metadata.query_ref == demand.query_ref
            && metadata.projection_digest == new_digest
    });
    let positive_witness_stale = formal_adequacy.is_some() && usable_positive.is_none();

    let mut usable_negative = Vec::new();
    let mut stale_negative_count = 0usize;
    for witness in nonfactorability_witnesses {
        let metadata = witness.metadata();
        if metadata.query_ref == demand.query_ref
            && metadata.projection_digest == new_digest
        {
            usable_negative.push((*witness).clone());
        } else {
            stale_negative_count += 1;
        }
    }

    let adequacy = compile_consumer_adequacy(
        demand,
        &impact.new_projection.graph,
        &current_coverage,
        operational_state,
        usable_positive,
        &usable_negative,
    )?;
    let formal_witnesses = QueryWorldFormalWitnessDisposition {
        new_projection_digest: new_digest.to_owned(),
        positive_witness_usable: usable_positive.is_some(),
        positive_witness_stale,
        usable_nonfactorability_witness_count: usable_negative.len(),
        stale_nonfactorability_witness_count: stale_negative_count,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };

    match impact.kind {
        QueryWorldImpactKind::NoWorldCoordinateChange => {
            Ok(QueryWorldResearchOutcome::NoWorldCoordinateChange {
                impact,
                adequacy,
                formal_witnesses,
            })
        }
        QueryWorldImpactKind::WorldChangedConsumerInvariant => {
            Ok(QueryWorldResearchOutcome::WorldChangedConsumerInvariant {
                impact,
                adequacy,
                formal_witnesses,
            })
        }
        QueryWorldImpactKind::WorldChangedConsumerRelevant => {
            Ok(QueryWorldResearchOutcome::WorldChangedConsumerResidual {
                impact,
                adequacy,
                formal_witnesses,
            })
        }
    }
}


pub fn query_scoped_world_impact_self_check() -> Result<(), String> {
    use std::collections::BTreeMap;

    let world = |world_ref: &str,
                 jurisdiction_ref: &str,
                 as_at: &str,
                 a_revision: &str,
                 b_revision: &str| {
        LegalWorldCoordinate {
            world_ref: world_ref.to_owned(),
            matter_ref: "matter:query-world-self-check".into(),
            jurisdiction_ref: jurisdiction_ref.to_owned(),
            as_at: as_at.to_owned(),
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
        kind: crate::ProjectionKind::IssueProof,
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
        deterministic_digest: "sha256:query-world-self-check".into(),
        projection_only: true,
        creates_semantic_authority: false,
    };

    let dependencies = RevisionDependencyIndex {
        source_to_propositions: BTreeMap::from([
            ("source:a".into(), BTreeSet::from(["prop:q".into()])),
            ("source:b".into(), BTreeSet::from(["prop:other".into()])),
        ]),
        proposition_dependents: BTreeMap::from([
            ("prop:q".into(), BTreeSet::from(["proof:q".into()])),
            ("prop:other".into(), BTreeSet::from(["proof:other".into()])),
        ]),
    };

    let base_slice = QueryDependencySlice {
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
        source_revision_refs: BTreeSet::from(["rev:a:1".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };

    let old = world("world:old", "AU", "2026-09-21", "rev:a:1", "rev:b:1");

    let irrelevant_revision =
        world("world:irrelevant", "AU", "2026-09-21", "rev:a:1", "rev:b:2");
    let irrelevant = compile_query_world_impact(
        &old,
        &irrelevant_revision,
        &dependencies,
        &base_slice,
        &graph,
    )?;
    if irrelevant.kind != QueryWorldImpactKind::WorldChangedConsumerInvariant
        || irrelevant.query_projection_digest_changed
        || irrelevant.reopens_consumer_research
    {
        return Err("query-world self-check failed irrelevant revision invariance".into());
    }

    let relevant_revision =
        world("world:relevant", "AU", "2026-09-21", "rev:a:2", "rev:b:1");
    let relevant = compile_query_world_impact(
        &old,
        &relevant_revision,
        &dependencies,
        &base_slice,
        &graph,
    )?;
    if relevant.kind != QueryWorldImpactKind::WorldChangedConsumerRelevant
        || !relevant.query_projection_digest_changed
        || !relevant.reopens_consumer_research
    {
        return Err("query-world self-check failed relevant revision reopening".into());
    }

    let later = world("world:later", "AU", "2027-09-21", "rev:a:1", "rev:b:1");
    let time_invariant =
        compile_query_world_impact(&old, &later, &dependencies, &base_slice, &graph)?;
    if time_invariant.kind != QueryWorldImpactKind::WorldChangedConsumerInvariant
        || time_invariant.query_projection_digest_changed
    {
        return Err("query-world self-check failed unrequired temporal invariance".into());
    }

    let mut jurisdiction_slice = base_slice.clone();
    jurisdiction_slice
        .required_axes
        .insert(ConsumerAxis::Jurisdiction);
    let nsw = world("world:nsw", "AU-NSW", "2026-09-21", "rev:a:1", "rev:b:1");
    let jurisdiction =
        compile_query_world_impact(&old, &nsw, &dependencies, &jurisdiction_slice, &graph)?;
    if jurisdiction.kind != QueryWorldImpactKind::WorldChangedConsumerRelevant
        || !jurisdiction.jurisdiction_relevant
        || !jurisdiction.query_projection_digest_changed
    {
        return Err("query-world self-check failed required jurisdiction reopening".into());
    }

    let temporal_demand = ConsumerQueryDemand {
        query_ref: "query:q".into(),
        required_axes: {
            let mut axes = base_slice.required_axes.clone();
            axes.insert(ConsumerAxis::Temporal);
            axes
        },
        required_semantic_refs: BTreeSet::from(["prop:q".into()]),
        candidate_only: true,
        creates_semantic_authority: false,
    };
    let mut temporal_slice = base_slice.clone();
    temporal_slice.required_axes.insert(ConsumerAxis::Temporal);
    let paid_old_world = ConsumerCoverage {
        paid_axes: BTreeSet::from([
            ConsumerAxis::Temporal,
            ConsumerAxis::SourceRevision,
            ConsumerAxis::SourceSpan,
            ConsumerAxis::Provenance,
        ]),
        ..ConsumerCoverage::default()
    };
    let reopened = compile_query_world_research(
        &old,
        &later,
        &dependencies,
        &temporal_slice,
        &temporal_demand,
        &graph,
        &paid_old_world,
        OperationalResearchState::CurrentFrontierClosed,
        None,
        &[],
    )?;
    let QueryWorldResearchOutcome::WorldChangedConsumerResidual {
        adequacy: ConsumerAdequacyCompilation::NeedsResearch(research),
        ..
    } = reopened
    else {
        return Err(
            "query-world self-check failed stale temporal payment reopening".into(),
        );
    };
    if !research.runtime_receipt.missing_axes.contains(&ConsumerAxis::Temporal)
        || research.runtime_receipt.paid_axes.contains(&ConsumerAxis::Temporal)
    {
        return Err(
            "query-world self-check allowed W0 temporal payment to survive W1".into(),
        );
    }

    if irrelevant.creates_semantic_authority
        || irrelevant.creates_claim_truth
        || relevant.creates_semantic_authority
        || relevant.creates_claim_truth
        || jurisdiction.creates_semantic_authority
        || jurisdiction.creates_claim_truth
    {
        return Err("query-world self-check crossed non-promotion boundary".into());
    }

    Ok(())
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
            source_revision_refs: BTreeSet::from(["rev:a:1".into()]),
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


    #[test]
    fn explanation_index_compiles_query_dependency_revisions_transitively() {
        use crate::{
            ExplainableKind, ExplanationClass, ExplainableRef, ProvenanceAddress,
        };
        use std::collections::BTreeMap;

        let index = ExplanationIndex {
            records: BTreeMap::from([
                (
                    "prop:q".into(),
                    ExplainableRef {
                        semantic_ref: "prop:q".into(),
                        kind: ExplainableKind::LegalIssue,
                        class: ExplanationClass::SourceBacked,
                        provenance: vec![ProvenanceAddress {
                            manifestation_ref: Some("manifestation:q".into()),
                            source_revision_ref: "rev:a:1".into(),
                            span_ref: Some("span:q".into()),
                        }],
                        dependencies: vec!["reviewed:q".into()],
                        reverse_dependencies: vec![],
                        evidence_uses: vec![],
                        legal_uses: vec![],
                        residuals: vec![],
                        revision_lineage: vec![],
                    },
                ),
                (
                    "reviewed:q".into(),
                    ExplainableRef {
                        semantic_ref: "reviewed:q".into(),
                        kind: ExplainableKind::ReviewedEvidence,
                        class: ExplanationClass::SourceBacked,
                        provenance: vec![ProvenanceAddress {
                            manifestation_ref: Some("manifestation:q".into()),
                            source_revision_ref: "rev:a:1".into(),
                            span_ref: Some("span:q".into()),
                        }],
                        dependencies: vec![],
                        reverse_dependencies: vec!["prop:q".into()],
                        evidence_uses: vec![],
                        legal_uses: vec![],
                        residuals: vec![],
                        revision_lineage: vec![],
                    },
                ),
            ]),
            residual_explanations: BTreeMap::new(),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let demand = ConsumerQueryDemand {
            query_ref: "query:q".into(),
            required_axes: BTreeSet::from([ConsumerAxis::SourceRevision]),
            required_semantic_refs: BTreeSet::from(["prop:q".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };

        let compiled = compile_query_dependency_slice(&demand, &index).unwrap();
        assert_eq!(
            compiled.semantic_refs,
            BTreeSet::from(["prop:q".into()])
        );
        assert_eq!(
            compiled.proof_refs,
            BTreeSet::from(["reviewed:q".into()])
        );
        assert_eq!(
            compiled.source_revision_refs,
            BTreeSet::from(["rev:a:1".into()])
        );
        assert!(compiled.source_refs.is_empty());
        assert!(!compiled.creates_semantic_authority);
    }


    #[test]
    fn as_at_change_is_invariant_when_query_does_not_require_temporal_axis() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let mut new = old.clone();
        new.world_ref = "world:new".into();
        new.as_at = "2027-09-21".into();

        let impact =
            compile_query_world_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();
        assert_eq!(
            impact.kind,
            QueryWorldImpactKind::WorldChangedConsumerInvariant
        );
        assert!(impact.temporal_changed);
        assert!(!impact.temporal_relevant);
        assert!(!impact.query_projection_digest_changed);
        assert!(!impact.reopens_consumer_research);
    }

    #[test]
    fn as_at_change_changes_digest_when_temporal_axis_is_required() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let mut new = old.clone();
        new.world_ref = "world:new".into();
        new.as_at = "2027-09-21".into();
        let mut temporal_slice = slice();
        temporal_slice.required_axes.insert(ConsumerAxis::Temporal);

        let impact = compile_query_world_impact(
            &old,
            &new,
            &dependencies(),
            &temporal_slice,
            &graph(),
        )
        .unwrap();
        assert_eq!(
            impact.kind,
            QueryWorldImpactKind::WorldChangedConsumerRelevant
        );
        assert!(impact.temporal_changed);
        assert!(impact.temporal_relevant);
        assert!(impact.query_projection_digest_changed);
        assert!(impact.reopens_consumer_research);
    }

    #[test]
    fn jurisdiction_change_is_invariant_unless_query_requires_jurisdiction() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let mut new = old.clone();
        new.world_ref = "world:new".into();
        new.jurisdiction_ref = "AU-NSW".into();

        let invariant =
            compile_query_world_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();
        assert_eq!(
            invariant.kind,
            QueryWorldImpactKind::WorldChangedConsumerInvariant
        );
        assert!(!invariant.query_projection_digest_changed);

        let mut jurisdiction_slice = slice();
        jurisdiction_slice
            .required_axes
            .insert(ConsumerAxis::Jurisdiction);
        let relevant = compile_query_world_impact(
            &old,
            &new,
            &dependencies(),
            &jurisdiction_slice,
            &graph(),
        )
        .unwrap();
        assert_eq!(
            relevant.kind,
            QueryWorldImpactKind::WorldChangedConsumerRelevant
        );
        assert!(relevant.jurisdiction_relevant);
        assert!(relevant.query_projection_digest_changed);
    }


    #[test]
    fn paid_temporal_coordinate_becomes_stale_after_required_as_at_change() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let mut new = old.clone();
        new.world_ref = "world:new".into();
        new.as_at = "2027-09-21".into();

        let mut temporal_slice = slice();
        temporal_slice.required_axes.insert(ConsumerAxis::Temporal);
        let demand = ConsumerQueryDemand {
            query_ref: "query:q".into(),
            required_axes: temporal_slice.required_axes.clone(),
            required_semantic_refs: BTreeSet::from(["prop:q".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };
        let coverage = ConsumerCoverage {
            paid_axes: BTreeSet::from([ConsumerAxis::Temporal]),
            ..ConsumerCoverage::default()
        };

        let outcome = compile_query_world_research(
            &old,
            &new,
            &dependencies(),
            &temporal_slice,
            &demand,
            &graph(),
            &coverage,
            OperationalResearchState::CurrentFrontierClosed,
            None,
            &[],
        )
        .unwrap();
        let QueryWorldResearchOutcome::WorldChangedConsumerResidual {
            impact,
            adequacy,
            ..
        } = outcome
        else {
            panic!("required as-at change must reopen consumer research");
        };
        assert!(impact.temporal_relevant);
        let ConsumerAdequacyCompilation::NeedsResearch(research) = adequacy else {
            panic!("stale temporal payment must no longer satisfy the new world");
        };
        assert!(research
            .runtime_receipt
            .missing_axes
            .contains(&ConsumerAxis::Temporal));
        assert!(!research
            .runtime_receipt
            .paid_axes
            .contains(&ConsumerAxis::Temporal));
    }

    #[test]
    fn paid_source_coordinates_become_stale_after_relevant_revision_change() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:2", "rev:b:1");
        let impact =
            compile_query_world_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();
        let coverage = ConsumerCoverage {
            paid_axes: BTreeSet::from([
                ConsumerAxis::SourceRevision,
                ConsumerAxis::SourceSpan,
                ConsumerAxis::Provenance,
            ]),
            ..ConsumerCoverage::default()
        };
        let invalidated =
            invalidate_consumer_coverage_for_world_impact(&coverage, &impact);
        assert!(!invalidated
            .paid_axes
            .contains(&ConsumerAxis::SourceRevision));
        assert!(!invalidated
            .paid_axes
            .contains(&ConsumerAxis::SourceSpan));
        assert!(!invalidated
            .paid_axes
            .contains(&ConsumerAxis::Provenance));
    }

    #[test]
    fn consumer_invariant_world_change_preserves_paid_coverage() {
        let old = world("world:old", "rev:a:1", "rev:b:1");
        let mut new = old.clone();
        new.world_ref = "world:new".into();
        new.as_at = "2027-09-21".into();
        let impact =
            compile_query_world_impact(&old, &new, &dependencies(), &slice(), &graph())
                .unwrap();
        let coverage = ConsumerCoverage {
            paid_axes: BTreeSet::from([
                ConsumerAxis::SourceRevision,
                ConsumerAxis::Temporal,
            ]),
            ..ConsumerCoverage::default()
        };
        let preserved =
            invalidate_consumer_coverage_for_world_impact(&coverage, &impact);
        assert_eq!(preserved, coverage);
    }


    #[test]
    fn old_positive_adequacy_witness_becomes_stale_not_fatal_after_world_change() {
        use crate::{
            kernel_checked_factors_through_witness,
            AgdaFactorsThroughTypecheckReceipt,
        };

        let old = world("world:old", "rev:a:1", "rev:b:1");
        let new = world("world:new", "rev:a:2", "rev:b:1");
        let demand = ConsumerQueryDemand {
            query_ref: "query:q".into(),
            required_axes: slice().required_axes.clone(),
            required_semantic_refs: BTreeSet::from(["prop:q".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
        };

        let old_projection =
            compile_query_scoped_projection(&graph(), &slice()).unwrap();
        let old_checked = kernel_checked_factors_through_witness(
            &AgdaFactorsThroughTypecheckReceipt {
                schema_version: "sl.formal.agda_factors_through_typecheck.v0_1".into(),
                verifier: "agda".into(),
                command_ref: "agda -i . DASHI/Law/Fixture.agda".into(),
                exit_code: 0,
                query_ref: "query:q".into(),
                projection_digest: old_projection.graph.deterministic_digest.clone(),
                theorem_module_ref: "DASHI.Law.Fixture".into(),
                theorem_ref: "queryAdequate".into(),
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

        let outcome = compile_query_world_research(
            &old,
            &new,
            &dependencies(),
            &slice(),
            &demand,
            &graph(),
            &ConsumerCoverage::default(),
            OperationalResearchState::CurrentFrontierClosed,
            Some(&old_checked),
            &[],
        )
        .unwrap();

        let QueryWorldResearchOutcome::WorldChangedConsumerResidual {
            formal_witnesses,
            ..
        } = outcome
        else {
            panic!("relevant revision change must reopen");
        };
        assert!(formal_witnesses.positive_witness_stale);
        assert!(!formal_witnesses.positive_witness_usable);
    }

}
