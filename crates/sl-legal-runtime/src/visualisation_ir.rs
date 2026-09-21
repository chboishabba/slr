//! Typed visualisation IR over read-only legal projections.
//!
//! Rendering is downstream.  These types may reorganise a ProjectionGraph for
//! timeline/graph/Sankey/spatial consumers, but cannot create semantic
//! identities or legal authority.  Sankey weights are explicit flow counts,
//! never legal importance or authority weight.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{ProjectionGraph, ProjectionKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualNodeIr {
    pub semantic_ref: String,
    pub semantic_kind: String,
    pub label: String,
    pub source_revision_refs: Vec<String>,
    pub span_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualEdgeIr {
    pub from_ref: String,
    pub to_ref: String,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphIr {
    pub nodes: Vec<VisualNodeIr>,
    pub edges: Vec<VisualEdgeIr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEntryIr {
    pub semantic_ref: String,
    pub label: String,
    pub source_revision_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineIr {
    pub entries: Vec<TimelineEntryIr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SankeyNodeIr {
    pub node_ref: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SankeyLinkIr {
    pub from_ref: String,
    pub to_ref: String,
    pub weight: u64,
    pub weight_semantics: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SankeyIr {
    pub nodes: Vec<SankeyNodeIr>,
    pub links: Vec<SankeyLinkIr>,
    pub weights_are_legal_importance: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ResearchFlowStage {
    FrontierResidual,
    SelectedDemand,
    SourceAcquired,
    IdentityReview,
    TreatmentReview,
    AcceptedHop,
    PreservedResidual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchFlowEvent {
    pub event_ref: String,
    pub from_stage: ResearchFlowStage,
    pub to_stage: ResearchFlowStage,
    pub count: u64,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchFrontierIr {
    pub events: Vec<ResearchFlowEvent>,
    pub sankey: SankeyIr,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofTopologyIr {
    pub nodes: Vec<VisualNodeIr>,
    pub dependency_edges: Vec<VisualEdgeIr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeIr {
    pub nodes: Vec<VisualNodeIr>,
    pub edges: Vec<VisualEdgeIr>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisualisationIr {
    Timeline(TimelineIr),
    Graph(GraphIr),
    Sankey(SankeyIr),
    ResearchFrontier(ResearchFrontierIr),
    ProofTopology(ProofTopologyIr),
    Comparative(ComparativeIr),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualisationEnvelope {
    pub source_projection_digest: String,
    pub ir: VisualisationIr,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
}

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
    let nodes = graph
        .nodes
        .iter()
        .map(|node| SankeyNodeIr {
            node_ref: node.semantic_ref.clone(),
            label: node.semantic_ref.clone(),
        })
        .collect();
    let links = graph
        .edges
        .iter()
        .map(|edge| SankeyLinkIr {
            from_ref: edge.from_ref.clone(),
            to_ref: edge.to_ref.clone(),
            weight: 1,
            weight_semantics: "topology-edge-count".into(),
        })
        .collect();
    SankeyIr {
        nodes,
        links,
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
        ProjectionKind::Flow => VisualisationIr::Sankey(projection_sankey(graph)),
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

    Ok(VisualisationEnvelope {
        source_projection_digest: graph.deterministic_digest.clone(),
        ir,
        projection_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
    })
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
