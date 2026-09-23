//! Production typed comparative projection over persisted workbench read models.
//!
//! Database acquisition and semantic comparison happen before Dioxus. The UI
//! receives an already-computed portable ComparativeWorkbenchProjection.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_reader_model::{
    ComparativeWorkbenchProjection, PersistedWorkbenchEdge, PersistedWorkbenchGraph,
    PersistedWorkbenchNode, PersistedWorkbenchProjection, ThreeWayComparativeWorkbenchProjection,
};

use crate::{
    ComparativeWorkbenchOverlay, TypedAnswerChangingExplanation,
    workbench_overlay_from_explanation,
};

fn node_map(graph: &PersistedWorkbenchGraph) -> BTreeMap<String, &PersistedWorkbenchNode> {
    graph
        .nodes
        .iter()
        .map(|node| (node.semantic_ref.clone(), node))
        .collect()
}

fn edge_map(graph: &PersistedWorkbenchGraph) -> BTreeMap<String, &PersistedWorkbenchEdge> {
    graph
        .edges
        .iter()
        .map(|edge| (edge.semantic_ref.clone(), edge))
        .collect()
}

fn semantic_refs(graph: &PersistedWorkbenchGraph) -> BTreeSet<String> {
    graph
        .nodes
        .iter()
        .map(|node| node.semantic_ref.clone())
        .chain(graph.edges.iter().map(|edge| edge.semantic_ref.clone()))
        .collect()
}

fn object_changed(
    semantic_ref: &str,
    left_nodes: &BTreeMap<String, &PersistedWorkbenchNode>,
    right_nodes: &BTreeMap<String, &PersistedWorkbenchNode>,
    left_edges: &BTreeMap<String, &PersistedWorkbenchEdge>,
    right_edges: &BTreeMap<String, &PersistedWorkbenchEdge>,
) -> bool {
    match (
        left_nodes.get(semantic_ref).copied(),
        right_nodes.get(semantic_ref).copied(),
        left_edges.get(semantic_ref).copied(),
        right_edges.get(semantic_ref).copied(),
    ) {
        (Some(left), Some(right), _, _) => left != right,
        (_, _, Some(left), Some(right)) => left != right,
        _ => false,
    }
}

fn semantic_has_source_and_provenance(
    graph: &PersistedWorkbenchGraph,
    semantic_ref: &str,
) -> bool {
    graph
        .nodes
        .iter()
        .find(|node| node.semantic_ref == semantic_ref)
        .is_some_and(|node| !node.source_refs.is_empty() && !node.provenance_refs.is_empty())
        || graph
            .edges
            .iter()
            .find(|edge| edge.semantic_ref == semantic_ref)
            .is_some_and(|edge| !edge.source_refs.is_empty() && !edge.provenance_refs.is_empty())
}

fn validate_overlay(
    overlay: &ComparativeWorkbenchOverlay,
    left: &PersistedWorkbenchGraph,
    right: &PersistedWorkbenchGraph,
) -> Result<(), String> {
    overlay.validate()?;
    let available = semantic_refs(left)
        .union(&semantic_refs(right))
        .cloned()
        .collect::<BTreeSet<_>>();
    let referenced = overlay
        .change_layer_by_semantic_ref
        .keys()
        .chain(overlay.explanation_by_semantic_ref.keys())
        .chain(overlay.answer_changing_semantic_refs.iter())
        .chain(overlay.unresolved_semantic_refs.iter())
        .cloned()
        .collect::<BTreeSet<_>>();

    let missing = referenced
        .difference(&available)
        .cloned()
        .collect::<BTreeSet<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "typed comparative overlay references objects absent from typed workbench graphs: {missing:?}"
        ));
    }
    for semantic_ref in &overlay.answer_changing_semantic_refs {
        if !semantic_has_source_and_provenance(left, semantic_ref)
            && !semantic_has_source_and_provenance(right, semantic_ref)
        {
            return Err(format!(
                "answer-changing semantic object lacks source/provenance closure: {semantic_ref}"
            ));
        }
    }
    Ok(())
}

pub fn project_typed_workbench_comparison(
    comparison_ref: &str,
    left: &PersistedWorkbenchProjection,
    right: &PersistedWorkbenchProjection,
    overlay: Option<&ComparativeWorkbenchOverlay>,
) -> Result<ComparativeWorkbenchProjection, String> {
    left.validate_read_only()?;
    right.validate_read_only()?;
    if comparison_ref.trim().is_empty() {
        return Err("typed comparative projection requires comparison_ref".into());
    }

    let left_refs = semantic_refs(&left.legal_follow_graph);
    let right_refs = semantic_refs(&right.legal_follow_graph);
    let shared = left_refs
        .intersection(&right_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let left_only = left_refs
        .difference(&right_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let right_only = right_refs
        .difference(&left_refs)
        .cloned()
        .collect::<BTreeSet<_>>();

    let left_nodes = node_map(&left.legal_follow_graph);
    let right_nodes = node_map(&right.legal_follow_graph);
    let left_edges = edge_map(&left.legal_follow_graph);
    let right_edges = edge_map(&right.legal_follow_graph);
    let changed = shared
        .iter()
        .filter(|semantic_ref| {
            object_changed(
                semantic_ref,
                &left_nodes,
                &right_nodes,
                &left_edges,
                &right_edges,
            )
        })
        .cloned()
        .collect::<BTreeSet<_>>();

    let overlay = overlay.cloned().unwrap_or_default();
    validate_overlay(&overlay, &left.legal_follow_graph, &right.legal_follow_graph)?;

    let projection = ComparativeWorkbenchProjection {
        comparison_ref: comparison_ref.into(),
        left_world_ref: left.world_ref.clone(),
        right_world_ref: right.world_ref.clone(),
        left_graph: left.legal_follow_graph.clone(),
        right_graph: right.legal_follow_graph.clone(),
        shared_semantic_refs: shared.into_iter().collect(),
        changed_semantic_refs: changed.into_iter().collect(),
        left_only_semantic_refs: left_only.into_iter().collect(),
        right_only_semantic_refs: right_only.into_iter().collect(),
        change_layer_by_semantic_ref: overlay.change_layer_by_semantic_ref,
        explanation_by_semantic_ref: overlay.explanation_by_semantic_ref,
        answer_changing_semantic_refs: overlay.answer_changing_semantic_refs.into_iter().collect(),
        unresolved_semantic_refs: overlay.unresolved_semantic_refs.into_iter().collect(),
        candidate_only: true,
        projection_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        predicts_outcome: false,
    };
    projection.validate_read_only()?;
    Ok(projection)
}

pub fn project_typed_workbench_comparison_from_explanation(
    comparison_ref: &str,
    left: &PersistedWorkbenchProjection,
    right: &PersistedWorkbenchProjection,
    explanation: &TypedAnswerChangingExplanation,
) -> Result<ComparativeWorkbenchProjection, String> {
    let overlay = workbench_overlay_from_explanation(explanation)?;
    project_typed_workbench_comparison(comparison_ref, left, right, Some(&overlay))
}

pub fn project_typed_three_way_workbench_comparison(
    comparison_ref: &str,
    w0: PersistedWorkbenchProjection,
    w1: PersistedWorkbenchProjection,
    w2: PersistedWorkbenchProjection,
    w0_w1_overlay: Option<&ComparativeWorkbenchOverlay>,
    w1_w2_overlay: Option<&ComparativeWorkbenchOverlay>,
) -> Result<ThreeWayComparativeWorkbenchProjection, String> {
    let w0_to_w1 = project_typed_workbench_comparison(
        &format!("{comparison_ref}:w0-w1"),
        &w0,
        &w1,
        w0_w1_overlay,
    )?;
    let w1_to_w2 = project_typed_workbench_comparison(
        &format!("{comparison_ref}:w1-w2"),
        &w1,
        &w2,
        w1_w2_overlay,
    )?;
    let projection = ThreeWayComparativeWorkbenchProjection {
        w0,
        w1,
        w2,
        w0_to_w1,
        w1_to_w2,
        candidate_only: true,
        projection_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
        predicts_outcome: false,
    };
    projection.validate_read_only()?;
    Ok(projection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(reference: &str, receipt: &str) -> PersistedWorkbenchNode {
        PersistedWorkbenchNode {
            semantic_ref: reference.into(),
            semantic_kind: "proposition".into(),
            label: reference.into(),
            source_refs: vec![format!("source:{reference}")],
            provenance_refs: vec![receipt.into()],
            candidate_only: true,
        }
    }

    fn workbench(world: &str, refs: &[(&str, &str)]) -> PersistedWorkbenchProjection {
        PersistedWorkbenchProjection {
            world_ref: world.into(),
            source_refs: vec![],
            event_refs: vec![],
            handoff_refs: vec![],
            research_residual_refs: vec![],
            legal_follow_graph: PersistedWorkbenchGraph {
                projection_ref: format!("projection:{world}"),
                document_ref: format!("document:{world}"),
                nodes: refs.iter().map(|(r, p)| node(r, p)).collect(),
                edges: vec![],
                derived_only: true,
                challengeable: true,
            },
            candidate_only: true,
            projection_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
            pays_residual: false,
        }
    }

    #[test]
    fn typed_comparison_detects_shared_changed_and_one_sided_objects() {
        let left = workbench(
            "w0",
            &[("semantic:shared", "receipt:same"), ("semantic:left", "receipt:left")],
        );
        let right = workbench(
            "w1",
            &[("semantic:shared", "receipt:new"), ("semantic:right", "receipt:right")],
        );
        let projection =
            project_typed_workbench_comparison("comparison:test", &left, &right, None).unwrap();

        assert!(projection
            .shared_semantic_refs
            .contains(&"semantic:shared".to_string()));
        assert!(projection
            .changed_semantic_refs
            .contains(&"semantic:shared".to_string()));
        assert!(projection
            .left_only_semantic_refs
            .contains(&"semantic:left".to_string()));
        assert!(projection
            .right_only_semantic_refs
            .contains(&"semantic:right".to_string()));
        assert!(!projection.creates_claim_truth);
    }

    #[test]
    fn answer_changing_overlay_must_weld_to_typed_graph_and_provenance() {
        let left = workbench("w0", &[("semantic:D", "receipt:D")]);
        let right = workbench("w1", &[("semantic:D", "receipt:D")]);
        let overlay = ComparativeWorkbenchOverlay {
            change_layer_by_semantic_ref: BTreeMap::from([(
                "semantic:missing".into(),
                "Applicability".into(),
            )]),
            explanation_by_semantic_ref: BTreeMap::new(),
            answer_changing_semantic_refs: BTreeSet::from(["semantic:missing".into()]),
            unresolved_semantic_refs: BTreeSet::new(),
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        assert!(project_typed_workbench_comparison(
            "comparison:bad",
            &left,
            &right,
            Some(&overlay),
        )
        .is_err());
    }
}
