//! Typed visualisation compiler over read-only legal projections.
//!
//! DTO ownership lives in `sensiblaw-reader-model`, the stable frontend ABI.
//! This module owns only semantic-to-read-model compilation and research-flow
//! aggregation.

use std::collections::BTreeMap;

use crate::{ProjectionGraph, ProjectionKind};

pub use sensiblaw_reader_model::{
    ComparativeIr, GraphIr, ProofTopologyIr, ResearchFlowEvent, ResearchFlowStage,
    ResearchFrontierIr, SankeyIr, SankeyLinkIr, SankeyNodeIr, TimelineEntryIr,
    TimelineIr, VisualEdgeIr, VisualNodeIr, VisualisationEnvelope, VisualisationIr,
};

fn nodes(graph: &ProjectionGraph) -> Vec<VisualNodeIr> {
    graph
        .nodes
        .iter()
        .map(|node| VisualNodeIr {
            semantic_ref: node.semantic_ref.clone(),
            semantic_kind: node.semantic_kind.clone(),
            label: node.semantic_ref.clone(),
            source_revision_refs: node.source_revision_refs.clone(),
            span_refs: node.span_refs.clone(),
        })
        .collect()
}

fn edges(graph: &ProjectionGraph) -> Vec<VisualEdgeIr> {
    graph
        .edges
        .iter()
        .map(|edge| VisualEdgeIr {
            from_ref: edge.from_ref.clone(),
            to_ref: edge.to_ref.clone(),
            relation: edge.relation.clone(),
        })
        .collect()
}

fn projection_sankey(graph: &ProjectionGraph) -> SankeyIr {
    SankeyIr {
        nodes: graph
            .nodes
            .iter()
            .map(|node| SankeyNodeIr {
                node_ref: node.semantic_ref.clone(),
                label: node.semantic_ref.clone(),
            })
            .collect(),
        links: graph
            .edges
            .iter()
            .map(|edge| SankeyLinkIr {
                from_ref: edge.from_ref.clone(),
                to_ref: edge.to_ref.clone(),
                weight: 1,
                weight_semantics: "topology-edge-count".into(),
            })
            .collect(),
        weights_are_legal_importance: false,
    }
}

pub fn visualisation_from_projection(
    graph: &ProjectionGraph,
) -> Result<VisualisationEnvelope, String> {
    if !graph.projection_only || graph.creates_semantic_authority {
        return Err("visualisation requires a non-authoritative ProjectionGraph".into());
    }

    let ir = match graph.kind {
        ProjectionKind::Timeline => VisualisationIr::Timeline(TimelineIr {
            entries: graph
                .nodes
                .iter()
                .map(|node| TimelineEntryIr {
                    semantic_ref: node.semantic_ref.clone(),
                    label: node.semantic_ref.clone(),
                    source_revision_refs: node.source_revision_refs.clone(),
                })
                .collect(),
        }),
        ProjectionKind::Flow => {
            let sankey = projection_sankey(graph);
            sankey.validate_count_semantics()?;
            VisualisationIr::Sankey(sankey)
        }
        ProjectionKind::IssueProof => VisualisationIr::ProofTopology(ProofTopologyIr {
            nodes: nodes(graph),
            dependency_edges: edges(graph),
        }),
        ProjectionKind::Comparative => VisualisationIr::Comparative(ComparativeIr {
            nodes: nodes(graph),
            edges: edges(graph),
        }),
        ProjectionKind::SourceView
        | ProjectionKind::EntityRelationship
        | ProjectionKind::CitationAuthority => VisualisationIr::Graph(GraphIr {
            nodes: nodes(graph),
            edges: edges(graph),
        }),
    };

    let envelope = VisualisationEnvelope {
        source_projection_digest: graph.deterministic_digest.clone(),
        ir,
        projection_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
    };
    envelope.validate_read_only()?;
    Ok(envelope)
}

pub fn research_flow_sankey(
    events: &[ResearchFlowEvent],
) -> Result<ResearchFrontierIr, String> {
    if events
        .iter()
        .any(|event| !event.candidate_only || event.event_ref.trim().is_empty())
    {
        return Err("research-flow events must be named and candidate-only".into());
    }

    let mut counts: BTreeMap<(ResearchFlowStage, ResearchFlowStage), u64> = BTreeMap::new();
    for event in events {
        *counts
            .entry((event.from_stage, event.to_stage))
            .or_default() += event.count;
    }

    let stages = [
        ResearchFlowStage::FrontierResidual,
        ResearchFlowStage::SelectedDemand,
        ResearchFlowStage::SourceAcquired,
        ResearchFlowStage::IdentityReview,
        ResearchFlowStage::TreatmentReview,
        ResearchFlowStage::AcceptedHop,
        ResearchFlowStage::PreservedResidual,
    ];
    let sankey = SankeyIr {
        nodes: stages
            .iter()
            .map(|stage| SankeyNodeIr {
                node_ref: format!("research-stage:{stage:?}"),
                label: format!("{stage:?}"),
            })
            .collect(),
        links: counts
            .into_iter()
            .map(|((from, to), weight)| SankeyLinkIr {
                from_ref: format!("research-stage:{from:?}"),
                to_ref: format!("research-stage:{to:?}"),
                weight,
                weight_semantics: "observed-event-count".into(),
            })
            .collect(),
        weights_are_legal_importance: false,
    };
    sankey.validate_count_semantics()?;

    Ok(ResearchFrontierIr {
        events: events.to_vec(),
        sankey,
        creates_semantic_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProjectionEdge, ProjectionNode};

    fn graph(kind: ProjectionKind) -> ProjectionGraph {
        ProjectionGraph {
            kind,
            nodes: vec![
                ProjectionNode {
                    semantic_ref: "case:a".into(),
                    semantic_kind: "CaseAuthority".into(),
                    manifestation_refs: vec![],
                    source_revision_refs: vec!["revision:a".into()],
                    span_refs: vec!["span:a".into()],
                    projection_role: format!("{kind:?}"),
                },
                ProjectionNode {
                    semantic_ref: "case:b".into(),
                    semantic_kind: "CaseAuthority".into(),
                    manifestation_refs: vec![],
                    source_revision_refs: vec!["revision:b".into()],
                    span_refs: vec!["span:b".into()],
                    projection_role: format!("{kind:?}"),
                },
            ],
            edges: vec![ProjectionEdge {
                from_ref: "case:a".into(),
                to_ref: "case:b".into(),
                relation: "supports".into(),
            }],
            deterministic_digest: "sha256:fixture".into(),
            projection_only: true,
            creates_semantic_authority: false,
        }
    }

    #[test]
    fn flow_projection_becomes_unit_count_sankey_not_legal_weight() {
        let visual = visualisation_from_projection(&graph(ProjectionKind::Flow)).unwrap();
        let VisualisationIr::Sankey(sankey) = visual.ir else {
            panic!("expected Sankey IR");
        };
        assert_eq!(sankey.links[0].weight, 1);
        assert_eq!(sankey.links[0].weight_semantics, "topology-edge-count");
        assert!(!sankey.weights_are_legal_importance);
        assert!(!visual.creates_semantic_authority);
        assert!(!visual.creates_legal_authority);
    }

    #[test]
    fn research_flow_aggregates_counts_without_truth_ranking() {
        let flow = research_flow_sankey(&[
            ResearchFlowEvent {
                event_ref: "event:1".into(),
                from_stage: ResearchFlowStage::FrontierResidual,
                to_stage: ResearchFlowStage::SelectedDemand,
                count: 3,
                candidate_only: true,
            },
            ResearchFlowEvent {
                event_ref: "event:2".into(),
                from_stage: ResearchFlowStage::FrontierResidual,
                to_stage: ResearchFlowStage::SelectedDemand,
                count: 2,
                candidate_only: true,
            },
        ])
        .unwrap();

        assert_eq!(flow.sankey.links.len(), 1);
        assert_eq!(flow.sankey.links[0].weight, 5);
        assert_eq!(
            flow.sankey.links[0].weight_semantics,
            "observed-event-count"
        );
        assert!(!flow.sankey.weights_are_legal_importance);
        assert!(!flow.creates_semantic_authority);
    }
}
