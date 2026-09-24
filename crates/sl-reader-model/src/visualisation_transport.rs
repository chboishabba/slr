//! Read-only visualization transport owned by the reader ABI.
//!
//! Semantic compilation remains outside this crate.  These DTOs are transport
//! and rendering inputs only; their firewall flags make accidental authority
//! promotion visible at every frontend boundary.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::ReaderIntent;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReaderPickIntent {
    pub target_ref: String,
    pub intent: ReaderIntent,
}

impl ReaderPickIntent {
    pub fn new(target_ref: impl Into<String>, intent: ReaderIntent) -> Result<Self, String> {
        let target_ref = target_ref.into();
        if target_ref.trim().is_empty() {
            return Err("visual pick target_ref must be non-empty".into());
        }
        Ok(Self { target_ref, intent })
    }

    #[must_use]
    pub const fn creates_semantic_authority(&self) -> bool {
        false
    }

    #[must_use]
    pub const fn creates_evidence_payment(&self) -> bool {
        false
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VisualNodeIr {
    pub semantic_ref: String,
    pub semantic_kind: String,
    pub label: String,
    pub source_revision_refs: Vec<String>,
    pub span_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VisualEdgeIr {
    pub from_ref: String,
    pub to_ref: String,
    pub relation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GraphIr {
    pub nodes: Vec<VisualNodeIr>,
    pub edges: Vec<VisualEdgeIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TimelineEntryIr {
    pub semantic_ref: String,
    pub label: String,
    pub source_revision_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct TimelineIr {
    pub entries: Vec<TimelineEntryIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SankeyNodeIr {
    pub node_ref: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SankeyLinkIr {
    pub from_ref: String,
    pub to_ref: String,
    pub weight: u64,
    pub weight_semantics: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SankeyIr {
    pub nodes: Vec<SankeyNodeIr>,
    pub links: Vec<SankeyLinkIr>,
    pub weights_are_legal_importance: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ResearchFlowStage {
    FrontierResidual,
    SelectedDemand,
    SourceAcquired,
    IdentityReview,
    TreatmentReview,
    AcceptedHop,
    PreservedResidual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResearchFlowEvent {
    pub event_ref: String,
    pub from_stage: ResearchFlowStage,
    pub to_stage: ResearchFlowStage,
    pub count: u64,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResearchFrontierIr {
    pub events: Vec<ResearchFlowEvent>,
    pub sankey: SankeyIr,
    pub creates_semantic_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ProofTopologyIr {
    pub nodes: Vec<VisualNodeIr>,
    pub dependency_edges: Vec<VisualEdgeIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ComparativeIr {
    pub nodes: Vec<VisualNodeIr>,
    pub edges: Vec<VisualEdgeIr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VisualisationIr {
    Timeline(TimelineIr),
    Graph(GraphIr),
    Sankey(SankeyIr),
    ResearchFrontier(ResearchFrontierIr),
    ProofTopology(ProofTopologyIr),
    Comparative(ComparativeIr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VisualisationEnvelope {
    pub source_projection_digest: String,
    pub ir: VisualisationIr,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
}

impl VisualisationEnvelope {
    pub fn validate_read_only(&self) -> Result<(), String> {
        if !self.projection_only
            || self.creates_semantic_authority
            || self.creates_legal_authority
        {
            return Err("visualisation envelope crossed read-only authority boundary".into());
        }
        Ok(())
    }
}

impl SankeyIr {
    pub fn validate_count_semantics(&self) -> Result<(), String> {
        if self.weights_are_legal_importance {
            return Err("Sankey weights may not encode legal importance".into());
        }
        if self.links.iter().any(|link| {
            link.weight_semantics != "topology-edge-count"
                && link.weight_semantics != "observed-event-count"
        }) {
            return Err("Sankey link uses unknown/non-count weight semantics".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sankey_transport_rejects_legal_importance_weights() {
        let sankey = SankeyIr {
            nodes: vec![],
            links: vec![],
            weights_are_legal_importance: true,
        };
        assert!(sankey.validate_count_semantics().is_err());
    }

    #[test]
    #[test]
    fn visual_pick_wraps_existing_reader_intent_without_payment() {
        let pick = ReaderPickIntent::new("case:fixture", ReaderIntent::ExpandProofCone).unwrap();
        assert_eq!(pick.target_ref, "case:fixture");
        assert_eq!(pick.intent, ReaderIntent::ExpandProofCone);
        assert!(!pick.creates_semantic_authority());
        assert!(!pick.creates_evidence_payment());
    }

    fn visualisation_transport_is_read_only() {
        let envelope = VisualisationEnvelope {
            source_projection_digest: "sha256:fixture".into(),
            ir: VisualisationIr::Graph(GraphIr {
                nodes: vec![],
                edges: vec![],
            }),
            projection_only: true,
            creates_semantic_authority: false,
            creates_legal_authority: false,
        };
        assert!(envelope.validate_read_only().is_ok());
    }
}