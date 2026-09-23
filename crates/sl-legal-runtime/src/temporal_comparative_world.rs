//! M11 / S26.7 temporal and revision comparison adapter.
//!
//! This module does not re-derive revision relevance. It consumes the existing
//! S18 QueryWorldImpact, preserving its query-scoped invariance/reopening
//! result and translating it into the generic comparative delta vocabulary.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{
    ComparativeDelta, ComparativeDeltaKind, ComparativeDeltaRole,
    QueryWorldImpact, QueryWorldImpactKind,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemporalComparativeReceipt {
    pub query_ref: String,
    pub left_world_ref: String,
    pub right_world_ref: String,
    pub deltas: Vec<ComparativeDelta>,
    pub query_relevant_delta_refs: BTreeSet<String>,
    pub query_irrelevant_delta_refs: BTreeSet<String>,
    pub query_projection_changed: bool,
    pub reopens_consumer_research: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn temporal_comparison_from_query_world_impact(
    impact: &QueryWorldImpact,
) -> Result<TemporalComparativeReceipt, String> {
    if !impact.candidate_only
        || impact.creates_semantic_authority
        || impact.creates_claim_truth
    {
        return Err("query-world impact crossed non-promotion boundary".into());
    }

    let mut deltas = Vec::new();
    let mut relevant = BTreeSet::new();
    let mut irrelevant = BTreeSet::new();

    for change in &impact.revision_impact.invalidation.changes {
        let delta_ref = format!("delta:source-revision:{}", change.source_ref);
        let is_relevant = impact
            .revision_impact
            .query_relevant_changed_source_refs
            .contains(&change.source_ref)
            || change.old_revision_ref.as_ref().is_some_and(|revision| {
                impact
                    .revision_impact
                    .query_relevant_changed_revision_refs
                    .contains(revision)
            });

        let delta = ComparativeDelta {
            delta_ref: delta_ref.clone(),
            kind: ComparativeDeltaKind::FactChanged,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some(change.source_ref.clone()),
            route_ref: None,
            residual_ref: None,
            before_ref: change.old_revision_ref.clone(),
            after_ref: change.new_revision_ref.clone(),
            cause_refs: BTreeSet::from(["s18:revision-invalidation".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        delta.validate()?;
        if is_relevant {
            relevant.insert(delta_ref);
        } else {
            irrelevant.insert(delta_ref);
        }
        deltas.push(delta);
    }

    if impact.temporal_changed {
        let delta_ref = "delta:world:as-at".to_owned();
        let delta = ComparativeDelta {
            delta_ref: delta_ref.clone(),
            kind: ComparativeDeltaKind::AsAtChanged,
            role: ComparativeDeltaRole::Context,
            coordinate_ref: Some("coordinate:world:as-at".into()),
            route_ref: None,
            residual_ref: None,
            before_ref: impact.old_projection.as_at.clone(),
            after_ref: impact.new_projection.as_at.clone(),
            cause_refs: BTreeSet::from(["s18:temporal-coordinate".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        delta.validate()?;
        if impact.temporal_relevant {
            relevant.insert(delta_ref);
        } else {
            irrelevant.insert(delta_ref);
        }
        deltas.push(delta);
    }

    if impact.jurisdiction_changed {
        let delta_ref = "delta:world:jurisdiction".to_owned();
        let delta = ComparativeDelta {
            delta_ref: delta_ref.clone(),
            kind: ComparativeDeltaKind::JurisdictionChanged,
            role: ComparativeDeltaRole::Context,
            coordinate_ref: Some("coordinate:world:jurisdiction".into()),
            route_ref: None,
            residual_ref: None,
            before_ref: impact.old_projection.jurisdiction_ref.clone(),
            after_ref: impact.new_projection.jurisdiction_ref.clone(),
            cause_refs: BTreeSet::from(["s18:jurisdiction-coordinate".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        delta.validate()?;
        if impact.jurisdiction_relevant {
            relevant.insert(delta_ref);
        } else {
            irrelevant.insert(delta_ref);
        }
        deltas.push(delta);
    }

    if impact.kind == QueryWorldImpactKind::WorldChangedConsumerInvariant
        && !relevant.is_empty()
    {
        return Err("consumer-invariant impact produced query-relevant comparative deltas".into());
    }

    Ok(TemporalComparativeReceipt {
        query_ref: impact.query_ref.clone(),
        left_world_ref: impact.revision_impact.invalidation.from_world_ref.clone(),
        right_world_ref: impact.revision_impact.invalidation.to_world_ref.clone(),
        deltas,
        query_relevant_delta_refs: relevant,
        query_irrelevant_delta_refs: irrelevant,
        query_projection_changed: impact.query_projection_digest_changed,
        reopens_consumer_research: impact.reopens_consumer_research,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile_query_world_impact, ConsumerAxis, LegalWorldCoordinate,
        ProjectionGraph, ProjectionKind, ProjectionNode, QueryDependencySlice,
        RevisionDependencyIndex,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn world(world_ref: &str, as_at: &str, a_revision: &str, b_revision: &str) -> LegalWorldCoordinate {
        LegalWorldCoordinate {
            world_ref: world_ref.into(),
            matter_ref: "matter:m11-temporal".into(),
            jurisdiction_ref: "AU".into(),
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
                    semantic_ref: "prop:noise".into(),
                    semantic_kind: "Proposition".into(),
                    manifestation_refs: vec!["manifestation:noise".into()],
                    source_revision_refs: vec!["rev:b:1".into()],
                    span_refs: vec!["span:noise".into()],
                    projection_role: "IssueProof".into(),
                },
            ],
            edges: vec![],
            deterministic_digest: "sha256:m11-temporal".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    fn dependencies() -> RevisionDependencyIndex {
        RevisionDependencyIndex {
            source_to_propositions: BTreeMap::from([
                ("source:a".into(), BTreeSet::from(["prop:q".into()])),
                ("source:b".into(), BTreeSet::from(["prop:noise".into()])),
            ]),
            proposition_dependents: BTreeMap::new(),
        }
    }

    fn slice(with_temporal: bool) -> QueryDependencySlice {
        let mut axes = BTreeSet::from([
            ConsumerAxis::SemanticIdentity,
            ConsumerAxis::SourceRevision,
        ]);
        if with_temporal {
            axes.insert(ConsumerAxis::Temporal);
        }
        QueryDependencySlice {
            query_ref: "query:m11-temporal".into(),
            required_axes: axes,
            semantic_refs: BTreeSet::from(["prop:q".into()]),
            proof_refs: BTreeSet::new(),
            source_refs: BTreeSet::from(["source:a".into()]),
            source_revision_refs: BTreeSet::from(["rev:a:1".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn unrelated_revision_is_visible_but_query_irrelevant() {
        let old = world("world:t0", "2026-09-01", "rev:a:1", "rev:b:1");
        let new = world("world:t1", "2026-09-01", "rev:a:1", "rev:b:2");
        let impact =
            compile_query_world_impact(&old, &new, &dependencies(), &slice(false), &graph())
                .unwrap();
        let receipt = temporal_comparison_from_query_world_impact(&impact).unwrap();

        assert!(receipt.query_relevant_delta_refs.is_empty());
        assert!(receipt
            .query_irrelevant_delta_refs
            .contains("delta:source-revision:source:b"));
        assert!(!receipt.query_projection_changed);
        assert!(!receipt.reopens_consumer_research);
    }

    #[test]
    fn required_as_at_change_is_query_relevant() {
        let old = world("world:t0", "2026-09-01", "rev:a:1", "rev:b:1");
        let new = world("world:t1", "2026-09-02", "rev:a:1", "rev:b:1");
        let impact =
            compile_query_world_impact(&old, &new, &dependencies(), &slice(true), &graph())
                .unwrap();
        let receipt = temporal_comparison_from_query_world_impact(&impact).unwrap();

        assert!(receipt
            .query_relevant_delta_refs
            .contains("delta:world:as-at"));
        assert!(receipt.query_projection_changed);
        assert!(receipt.reopens_consumer_research);
        assert!(!receipt.creates_claim_truth);
    }
}
