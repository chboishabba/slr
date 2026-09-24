//! Portable typed comparative workbench projection.
//!
//! The legal/runtime layer computes this structure from typed persisted
//! workbench projections. UI/renderers consume it but do not decide semantic
//! equality, answer-changing identity, or authority.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{PersistedWorkbenchGraph, PersistedWorkbenchProjection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ComparativeObjectClass {
    Shared,
    Changed,
    LeftOnly,
    RightOnly,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ComparativeChangeLayer {
    World,
    WorldEvidence,
    Observation,
    Representation,
    Theory,
    Belief,
    ConsumerProjection,
    Review,
    Scope,
    Applicability,
    ProofOutcome,
    ResidualOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ComparativeChangeAnnotation {
    pub semantic_ref: String,
    pub layer: ComparativeChangeLayer,
    pub justification_refs: Vec<String>,
    pub explanation_ref: Option<String>,
    pub answer_changing: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ComparativeChangeAnnotation {
    pub fn validate_read_only(&self) -> Result<(), String> {
        if self.semantic_ref.trim().is_empty()
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("comparative change annotation crossed read-only boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ComparativeWorkbenchProjection {
    pub comparison_ref: String,
    pub left_world_ref: String,
    pub right_world_ref: String,
    pub left_graph: PersistedWorkbenchGraph,
    pub right_graph: PersistedWorkbenchGraph,
    pub shared_semantic_refs: Vec<String>,
    pub changed_semantic_refs: Vec<String>,
    pub left_only_semantic_refs: Vec<String>,
    pub right_only_semantic_refs: Vec<String>,
    /// Typed production metadata. Frontends consume this directly.
    #[cfg_attr(feature = "serde", serde(default))]
    pub change_annotations: Vec<ComparativeChangeAnnotation>,
    /// Compatibility/export projections retained for older replay fixtures.
    pub change_layer_by_semantic_ref: std::collections::BTreeMap<String, String>,
    pub explanation_by_semantic_ref: std::collections::BTreeMap<String, String>,
    pub answer_changing_semantic_refs: Vec<String>,
    pub unresolved_semantic_refs: Vec<String>,
    pub candidate_only: bool,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub predicts_outcome: bool,
}

impl ComparativeWorkbenchProjection {
    pub fn validate_read_only(&self) -> Result<(), String> {
        if self.comparison_ref.trim().is_empty()
            || self.left_world_ref.trim().is_empty()
            || self.right_world_ref.trim().is_empty()
            || !self.candidate_only
            || !self.projection_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
            || self.predicts_outcome
        {
            return Err("comparative workbench projection crossed read-only boundary".into());
        }

        let available = self
            .left_graph
            .nodes
            .iter()
            .map(|node| node.semantic_ref.as_str())
            .chain(self.left_graph.edges.iter().map(|edge| edge.semantic_ref.as_str()))
            .chain(self.right_graph.nodes.iter().map(|node| node.semantic_ref.as_str()))
            .chain(self.right_graph.edges.iter().map(|edge| edge.semantic_ref.as_str()))
            .collect::<std::collections::BTreeSet<_>>();

        for annotation in &self.change_annotations {
            annotation.validate_read_only()?;
            if !available.contains(annotation.semantic_ref.as_str()) {
                return Err(format!(
                    "typed comparative annotation references unknown semantic object {}",
                    annotation.semantic_ref
                ));
            }
        }

        for reference in self
            .change_layer_by_semantic_ref
            .keys()
            .chain(self.explanation_by_semantic_ref.keys())
            .chain(self.answer_changing_semantic_refs.iter())
            .chain(self.unresolved_semantic_refs.iter())
        {
            if !available.contains(reference.as_str()) {
                return Err(format!(
                    "comparative metadata references unknown semantic object {reference}"
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ThreeWayComparativeWorkbenchProjection {
    pub w0: PersistedWorkbenchProjection,
    pub w1: PersistedWorkbenchProjection,
    pub w2: PersistedWorkbenchProjection,
    pub w0_to_w1: ComparativeWorkbenchProjection,
    pub w1_to_w2: ComparativeWorkbenchProjection,
    pub candidate_only: bool,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub predicts_outcome: bool,
}

impl ThreeWayComparativeWorkbenchProjection {
    pub fn validate_read_only(&self) -> Result<(), String> {
        self.w0.validate_read_only()?;
        self.w1.validate_read_only()?;
        self.w2.validate_read_only()?;
        self.w0_to_w1.validate_read_only()?;
        self.w1_to_w2.validate_read_only()?;
        if !self.candidate_only
            || !self.projection_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
            || self.predicts_outcome
        {
            return Err("three-way comparative projection crossed read-only boundary".into());
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
pub fn export_comparative_workbench_json(
    projection: &ComparativeWorkbenchProjection,
) -> Result<String, String> {
    projection.validate_read_only()?;
    serde_json::to_string_pretty(projection)
        .map_err(|error| format!("comparative diagnostic export failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparative_projection_rejects_unknown_overlay_identity() {
        let graph = PersistedWorkbenchGraph {
            projection_ref: "graph:1".into(),
            document_ref: "doc:1".into(),
            nodes: vec![],
            edges: vec![],
            derived_only: true,
            challengeable: true,
        };
        let projection = ComparativeWorkbenchProjection {
            comparison_ref: "comparison:1".into(),
            left_world_ref: "world:0".into(),
            right_world_ref: "world:1".into(),
            left_graph: graph.clone(),
            right_graph: graph,
            shared_semantic_refs: vec![],
            changed_semantic_refs: vec![],
            left_only_semantic_refs: vec![],
            right_only_semantic_refs: vec![],
            change_annotations: vec![],
            change_layer_by_semantic_ref: std::collections::BTreeMap::from([(
                "missing".into(),
                "Applicability".into(),
            )]),
            explanation_by_semantic_ref: std::collections::BTreeMap::new(),
            answer_changing_semantic_refs: vec![],
            unresolved_semantic_refs: vec![],
            candidate_only: true,
            projection_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
            predicts_outcome: false,
        };
        assert!(projection.validate_read_only().is_err());
    }
}
