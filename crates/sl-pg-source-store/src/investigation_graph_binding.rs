use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const INV_GRAPH_BINDING_SCHEMA: &str = "itir.inv1.graph-binding.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigationGraphBinding {
    pub schema: String,
    pub binding_ref: String,
    pub obligation_ref: String,
    pub projection_ref: String,
    pub matter_ref: String,
    pub binding_evidence_refs: Vec<String>,
    pub derived_only: bool,
    pub challengeable: bool,
    pub creates_graph_edges: bool,
    pub creates_semantic_authority: bool,
    pub creates_access_authority: bool,
    pub creates_acquisition_state: bool,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InvestigationGraphBindingError {
    #[error("graph binding requires stable obligation/projection/matter identity")]
    MissingIdentity,
    #[error("graph binding requires explicit binding evidence")]
    MissingEvidence,
    #[error("graph binding crossed the projection-only authority boundary")]
    PromotionBoundary,
}

fn valid(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn build_investigation_graph_binding(
    binding_ref: &str,
    obligation_ref: &str,
    projection_ref: &str,
    matter_ref: &str,
    binding_evidence_refs: Vec<String>,
) -> Result<InvestigationGraphBinding, InvestigationGraphBindingError> {
    if !valid(binding_ref)
        || !valid(obligation_ref)
        || !valid(projection_ref)
        || !valid(matter_ref)
    {
        return Err(InvestigationGraphBindingError::MissingIdentity);
    }
    if binding_evidence_refs.is_empty()
        || binding_evidence_refs.iter().any(|reference| !valid(reference))
    {
        return Err(InvestigationGraphBindingError::MissingEvidence);
    }
    Ok(InvestigationGraphBinding {
        schema: INV_GRAPH_BINDING_SCHEMA.into(),
        binding_ref: binding_ref.into(),
        obligation_ref: obligation_ref.into(),
        projection_ref: projection_ref.into(),
        matter_ref: matter_ref.into(),
        binding_evidence_refs,
        derived_only: true,
        challengeable: true,
        creates_graph_edges: false,
        creates_semantic_authority: false,
        creates_access_authority: false,
        creates_acquisition_state: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_is_projection_only_and_cannot_manufacture_edges() {
        let binding = build_investigation_graph_binding(
            "graph-binding:1",
            "acquisition:1",
            "projection:legal-follow:1",
            "matter:1",
            vec!["receipt:reviewed-binding".into()],
        )
        .unwrap();
        assert!(binding.derived_only);
        assert!(binding.challengeable);
        assert!(!binding.creates_graph_edges);
        assert!(!binding.creates_semantic_authority);
        assert!(!binding.creates_access_authority);
        assert!(!binding.creates_acquisition_state);
    }

    #[test]
    fn binding_requires_explicit_evidence() {
        assert_eq!(
            build_investigation_graph_binding(
                "graph-binding:1",
                "acquisition:1",
                "projection:1",
                "matter:1",
                vec![],
            ),
            Err(InvestigationGraphBindingError::MissingEvidence)
        );
    }
}
