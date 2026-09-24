//! M11.1 adapters from existing domain receipts into the generic ChangeLocus IR.
//!
//! These adapters do not discover new facts. They type already-computed
//! consumer-fibre and temporal/revision differences at the shared change-axis
//! boundary.

use std::collections::BTreeSet;

use crate::{
    compile_typed_change_set, typed_locus, ChangeLayer, ComparativeDelta,
    ComparativeDeltaKind, ComparativeDeltaRole, ConsumerFibreComparison,
    TemporalComparativeReceipt, TypedChangeSet,
};

fn projection_delta(
    delta_ref: String,
    coordinate_ref: String,
    kind: ComparativeDeltaKind,
    before_ref: Option<String>,
    after_ref: Option<String>,
) -> ComparativeDelta {
    ComparativeDelta {
        delta_ref,
        kind,
        role: ComparativeDeltaRole::WorldInput,
        coordinate_ref: Some(coordinate_ref),
        route_ref: None,
        residual_ref: None,
        before_ref,
        after_ref,
        cause_refs: BTreeSet::new(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn consumer_fibre_change_set(
    comparison_ref: &str,
    fibre: &ConsumerFibreComparison,
) -> Result<TypedChangeSet, String> {
    let mut loci = Vec::new();

    for coordinate_ref in &fibre.right_scope_blocked_coordinate_refs {
        let delta = projection_delta(
            format!("delta:{}:scope:{coordinate_ref}", fibre.right_consumer_ref),
            coordinate_ref.clone(),
            ComparativeDeltaKind::ScopeChanged,
            Some("visible-to-left-consumer".into()),
            Some("scope-blocked-for-right-consumer".into()),
        );
        loci.push(typed_locus(
            &format!("locus:{}:scope:{coordinate_ref}", fibre.right_consumer_ref),
            delta,
            ChangeLayer::Scope,
            Some("consumer-share-scope".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["m9:professional-handoff:scope-reason".into()]),
        )?);
    }

    for coordinate_ref in &fibre.right_dependency_irrelevant_coordinate_refs {
        let delta = projection_delta(
            format!(
                "delta:{}:dependency:{coordinate_ref}",
                fibre.right_consumer_ref
            ),
            coordinate_ref.clone(),
            ComparativeDeltaKind::ConsumerDependencyChanged,
            Some("visible-to-left-consumer".into()),
            Some("outside-right-consumer-dependency-slice".into()),
        );
        loci.push(typed_locus(
            &format!(
                "locus:{}:dependency:{coordinate_ref}",
                fibre.right_consumer_ref
            ),
            delta,
            ChangeLayer::ConsumerProjection,
            Some("consumer-dependency-slice".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["m9:professional-handoff:dependency-reason".into()]),
        )?);
    }

    for coordinate_ref in &fibre.right_not_ready_coordinate_refs {
        let delta = projection_delta(
            format!("delta:{}:review:{coordinate_ref}", fibre.right_consumer_ref),
            coordinate_ref.clone(),
            ComparativeDeltaKind::ReviewStateChanged,
            Some("visible-in-personal-fibre".into()),
            Some("not-ready-for-professional-fibre".into()),
        );
        loci.push(typed_locus(
            &format!("locus:{}:review:{coordinate_ref}", fibre.right_consumer_ref),
            delta,
            ChangeLayer::Review,
            Some("readiness-gate".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["m9:professional-handoff:not-ready-reason".into()]),
        )?);
    }

    for coordinate_ref in &fibre.right_unreviewed_coordinate_refs {
        let delta = projection_delta(
            format!(
                "delta:{}:unreviewed:{coordinate_ref}",
                fibre.right_consumer_ref
            ),
            coordinate_ref.clone(),
            ComparativeDeltaKind::ReviewStateChanged,
            Some("visible-in-personal-fibre".into()),
            Some("unreviewed-for-professional-fibre".into()),
        );
        loci.push(typed_locus(
            &format!(
                "locus:{}:unreviewed:{coordinate_ref}",
                fibre.right_consumer_ref
            ),
            delta,
            ChangeLayer::Review,
            Some("review-gate".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["m9:professional-handoff:unreviewed-reason".into()]),
        )?);
    }

    // A coordinate can legitimately be right-only due to a consumer projection
    // rule even when no more specific exclusion reason applies.
    for coordinate_ref in &fibre.right_only_visible_coordinate_refs {
        let already_typed = loci.iter().any(|locus| {
            locus.delta.coordinate_ref.as_deref() == Some(coordinate_ref.as_str())
        });
        if already_typed {
            continue;
        }
        let delta = projection_delta(
            format!(
                "delta:{}:projection-visible:{coordinate_ref}",
                fibre.right_consumer_ref
            ),
            coordinate_ref.clone(),
            ComparativeDeltaKind::ConsumerDependencyChanged,
            Some("not-visible-to-left-consumer".into()),
            Some("visible-to-right-consumer".into()),
        );
        loci.push(typed_locus(
            &format!(
                "locus:{}:projection-visible:{coordinate_ref}",
                fibre.right_consumer_ref
            ),
            delta,
            ChangeLayer::ConsumerProjection,
            Some("consumer-fibre".into()),
            BTreeSet::from(["layer:world".into()]),
            BTreeSet::from(["m9:professional-handoff:fibre-projection".into()]),
        )?);
    }

    compile_typed_change_set(comparison_ref, loci, [ChangeLayer::World])
}

pub fn temporal_change_set(
    comparison_ref: &str,
    temporal: &TemporalComparativeReceipt,
) -> Result<TypedChangeSet, String> {
    let mut loci = Vec::new();
    for delta in &temporal.deltas {
        let layer = match delta.kind {
            ComparativeDeltaKind::AsAtChanged
            | ComparativeDeltaKind::JurisdictionChanged => ChangeLayer::World,
            ComparativeDeltaKind::FactAdded
            | ComparativeDeltaKind::FactRemoved
            | ComparativeDeltaKind::FactChanged
            | ComparativeDeltaKind::AuthorityChanged => ChangeLayer::WorldEvidence,
            ComparativeDeltaKind::ReviewStateChanged => ChangeLayer::Review,
            ComparativeDeltaKind::ScopeChanged => ChangeLayer::Scope,
            ComparativeDeltaKind::ConsumerDependencyChanged => ChangeLayer::ConsumerProjection,
            ComparativeDeltaKind::ApplicabilityChanged
            | ComparativeDeltaKind::DefeaterAdded
            | ComparativeDeltaKind::DefeaterRemoved
            | ComparativeDeltaKind::CounterDefeaterAdded
            | ComparativeDeltaKind::CounterDefeaterRemoved => ChangeLayer::Applicability,
            ComparativeDeltaKind::ResidualOpened
            | ComparativeDeltaKind::ResidualClosed => ChangeLayer::ResidualOutcome,
            ComparativeDeltaKind::RouteStatusChanged => ChangeLayer::ProofOutcome,
        };

        // The S18 adapter can emit context deltas. ChangeLocus validation accepts
        // context at input-side layers but still rejects outcome mislabelling.
        loci.push(typed_locus(
            &format!("locus:temporal:{}", delta.delta_ref),
            delta.clone(),
            layer,
            Some("s18-query-world-impact".into()),
            BTreeSet::new(),
            BTreeSet::from(["s18:query-world-impact".into()]),
        )?);
    }

    compile_typed_change_set(comparison_ref, loci, [])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        run_personal_professional_comparison, temporal_comparison_from_query_world_impact,
        compile_query_world_impact, ConsumerAxis, LegalWorldCoordinate, ProjectionGraph,
        ProjectionKind, ProjectionNode, QueryDependencySlice, RevisionDependencyIndex,
    };
    use std::collections::BTreeMap;

    #[test]
    fn professional_fibre_reasons_become_scope_review_and_projection_layers() {
        let run = run_personal_professional_comparison().unwrap();
        let set = consumer_fibre_change_set(
            "comparison:personal-regulator",
            &run.personal_to_regulator,
        )
        .unwrap();

        assert!(set.changed_layers.contains(&ChangeLayer::Scope));
        assert!(set.changed_layers.contains(&ChangeLayer::Review));
        assert!(set.changed_layers.contains(&ChangeLayer::ConsumerProjection));
        assert!(set.invariant_layers.contains(&ChangeLayer::World));
    }

    #[test]
    fn irrelevant_source_revision_is_world_evidence_not_world_identity_change() {
        let old = LegalWorldCoordinate {
            world_ref: "world:old".into(),
            matter_ref: "matter:adapter".into(),
            jurisdiction_ref: "AU".into(),
            as_at: "2026-09-23".into(),
            source_revisions: BTreeMap::from([
                ("source:q".into(), "rev:q:1".into()),
                ("source:noise".into(), "rev:n:1".into()),
            ]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let new = LegalWorldCoordinate {
            world_ref: "world:new".into(),
            matter_ref: "matter:adapter".into(),
            jurisdiction_ref: "AU".into(),
            as_at: "2026-09-23".into(),
            source_revisions: BTreeMap::from([
                ("source:q".into(), "rev:q:1".into()),
                ("source:noise".into(), "rev:n:2".into()),
            ]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let graph = ProjectionGraph {
            kind: ProjectionKind::IssueProof,
            nodes: vec![
                ProjectionNode {
                    semantic_ref: "prop:q".into(),
                    semantic_kind: "Proposition".into(),
                    manifestation_refs: vec!["manifestation:q".into()],
                    source_revision_refs: vec!["rev:q:1".into()],
                    span_refs: vec!["span:q".into()],
                    projection_role: "IssueProof".into(),
                },
                ProjectionNode {
                    semantic_ref: "prop:noise".into(),
                    semantic_kind: "Proposition".into(),
                    manifestation_refs: vec!["manifestation:n".into()],
                    source_revision_refs: vec!["rev:n:1".into()],
                    span_refs: vec!["span:n".into()],
                    projection_role: "IssueProof".into(),
                },
            ],
            edges: vec![],
            deterministic_digest: "sha256:adapter".into(),
            projection_only: true,
            creates_semantic_authority: false,
        };
        let dependencies = RevisionDependencyIndex {
            source_to_propositions: BTreeMap::from([
                ("source:q".into(), BTreeSet::from(["prop:q".into()])),
                ("source:noise".into(), BTreeSet::from(["prop:noise".into()])),
            ]),
            proposition_dependents: BTreeMap::new(),
        };
        let slice = QueryDependencySlice {
            query_ref: "query:q".into(),
            required_axes: BTreeSet::from([
                ConsumerAxis::SemanticIdentity,
                ConsumerAxis::SourceRevision,
            ]),
            semantic_refs: BTreeSet::from(["prop:q".into()]),
            proof_refs: BTreeSet::new(),
            source_refs: BTreeSet::from(["source:q".into()]),
            source_revision_refs: BTreeSet::from(["rev:q:1".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };

        let impact =
            compile_query_world_impact(&old, &new, &dependencies, &slice, &graph).unwrap();
        let temporal = temporal_comparison_from_query_world_impact(&impact).unwrap();
        let set = temporal_change_set("comparison:temporal", &temporal).unwrap();

        assert!(set.changed_layers.contains(&ChangeLayer::WorldEvidence));
        assert!(!set.changed_layers.contains(&ChangeLayer::World));
        assert!(temporal.query_relevant_delta_refs.is_empty());
    }
}
