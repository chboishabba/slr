//! Portable typed workbench projection shared by persistence, comparative
//! runtime, and UI consumers.
//!
//! This is the production carrier. Serialization is optional and exists only
//! for diagnostic export, fixture replay, and offline bundles.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PersistedWorkbenchNode {
    pub semantic_ref: String,
    pub semantic_kind: String,
    pub label: String,
    pub source_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PersistedWorkbenchEdge {
    pub semantic_ref: String,
    pub from_ref: String,
    pub to_ref: String,
    pub relation: String,
    pub source_refs: Vec<String>,
    pub provenance_refs: Vec<String>,
    pub challengeable: bool,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PersistedWorkbenchGraph {
    pub projection_ref: String,
    pub document_ref: String,
    pub nodes: Vec<PersistedWorkbenchNode>,
    pub edges: Vec<PersistedWorkbenchEdge>,
    pub derived_only: bool,
    pub challengeable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PersistedWorkbenchProjection {
    pub world_ref: String,
    pub source_refs: Vec<String>,
    pub event_refs: Vec<String>,
    pub handoff_refs: Vec<String>,
    pub research_residual_refs: Vec<String>,
    pub legal_follow_graph: PersistedWorkbenchGraph,
    pub candidate_only: bool,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub pays_residual: bool,
}

impl PersistedWorkbenchProjection {
    pub fn validate_read_only(&self) -> Result<(), String> {
        if self.world_ref.trim().is_empty()
            || self.legal_follow_graph.projection_ref.trim().is_empty()
            || !self.candidate_only
            || !self.projection_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
            || self.pays_residual
            || !self.legal_follow_graph.derived_only
        {
            return Err("persisted workbench projection crossed read-only boundary".into());
        }
        for node in &self.legal_follow_graph.nodes {
            if node.semantic_ref.trim().is_empty()
                || node.semantic_kind.trim().is_empty()
                || !node.candidate_only
            {
                return Err("persisted workbench node is invalid/non-candidate".into());
            }
        }
        let node_refs = self
            .legal_follow_graph
            .nodes
            .iter()
            .map(|node| node.semantic_ref.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        for edge in &self.legal_follow_graph.edges {
            if edge.semantic_ref.trim().is_empty()
                || edge.from_ref == edge.to_ref
                || !node_refs.contains(edge.from_ref.as_str())
                || !node_refs.contains(edge.to_ref.as_str())
                || !edge.candidate_only
            {
                return Err("persisted workbench edge is invalid".into());
            }
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
pub fn export_persisted_workbench_json(
    projection: &PersistedWorkbenchProjection,
) -> Result<String, String> {
    projection.validate_read_only()?;
    serde_json::to_string_pretty(projection)
        .map_err(|error| format!("persisted workbench diagnostic export failed: {error}"))
}

#[cfg(feature = "serde")]
pub fn replay_persisted_workbench_json(
    raw: &str,
) -> Result<PersistedWorkbenchProjection, String> {
    let projection: PersistedWorkbenchProjection = serde_json::from_str(raw)
        .map_err(|error| format!("invalid persisted workbench replay bundle: {error}"))?;
    projection.validate_read_only()?;
    Ok(projection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PersistedWorkbenchProjection {
        PersistedWorkbenchProjection {
            world_ref: "world:fixture".into(),
            source_refs: vec!["source:1".into()],
            event_refs: vec![],
            handoff_refs: vec![],
            research_residual_refs: vec!["residual:1".into()],
            legal_follow_graph: PersistedWorkbenchGraph {
                projection_ref: "projection:1".into(),
                document_ref: "document:1".into(),
                nodes: vec![
                    PersistedWorkbenchNode {
                        semantic_ref: "node:a".into(),
                        semantic_kind: "authority".into(),
                        label: "A".into(),
                        source_refs: vec!["source:1".into()],
                        provenance_refs: vec!["receipt:1".into()],
                        candidate_only: true,
                    },
                    PersistedWorkbenchNode {
                        semantic_ref: "node:b".into(),
                        semantic_kind: "proposition".into(),
                        label: "B".into(),
                        source_refs: vec![],
                        provenance_refs: vec![],
                        candidate_only: true,
                    },
                ],
                edges: vec![PersistedWorkbenchEdge {
                    semantic_ref: "edge:1".into(),
                    from_ref: "node:a".into(),
                    to_ref: "node:b".into(),
                    relation: "supports".into(),
                    source_refs: vec![],
                    provenance_refs: vec!["receipt:edge:1".into()],
                    challengeable: true,
                    candidate_only: true,
                }],
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
    fn typed_workbench_is_read_only() {
        fixture().validate_read_only().unwrap();
    }

    #[cfg(feature = "serde")]
    #[test]
    fn json_is_optional_replay_export_around_typed_projection() {
        let typed = fixture();
        let json = export_persisted_workbench_json(&typed).unwrap();
        let replay = replay_persisted_workbench_json(&json).unwrap();
        assert_eq!(replay, typed);
    }
}
