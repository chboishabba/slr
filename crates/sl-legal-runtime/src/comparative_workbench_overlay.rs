//! M11.3 presentation overlay exported from typed comparative explanations.
//!
//! This is a renderer-facing projection only. It deliberately carries semantic
//! refs, layer labels and explanation text, but cannot create authority/truth.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{ChangeLayer, TypedAnswerChangingExplanation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ComparativeWorkbenchOverlay {
    pub change_layer_by_semantic_ref: BTreeMap<String, String>,
    pub explanation_by_semantic_ref: BTreeMap<String, String>,
    pub answer_changing_semantic_refs: BTreeSet<String>,
    pub unresolved_semantic_refs: BTreeSet<String>,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ComparativeWorkbenchOverlay {
    pub fn validate(&self) -> Result<(), String> {
        if self.creates_semantic_authority || self.creates_claim_truth {
            return Err("comparative workbench overlay crossed non-promotion boundary".into());
        }
        Ok(())
    }
}

fn layer_label(layer: ChangeLayer) -> &'static str {
    match layer {
        ChangeLayer::World => "World",
        ChangeLayer::WorldEvidence => "WorldEvidence",
        ChangeLayer::Observation => "Observation",
        ChangeLayer::Representation => "Representation",
        ChangeLayer::Theory => "Theory",
        ChangeLayer::Belief => "Belief",
        ChangeLayer::ConsumerProjection => "ConsumerProjection",
        ChangeLayer::Review => "Review",
        ChangeLayer::Scope => "Scope",
        ChangeLayer::Applicability => "Applicability",
        ChangeLayer::ProofOutcome => "ProofOutcome",
        ChangeLayer::ResidualOutcome => "ResidualOutcome",
    }
}

pub fn workbench_overlay_from_explanation(
    explanation: &TypedAnswerChangingExplanation,
) -> Result<ComparativeWorkbenchOverlay, String> {
    if explanation.predicts_outcome
        || explanation.claims_causation_beyond_receipts
        || explanation.creates_semantic_authority
        || explanation.creates_claim_truth
    {
        return Err("typed explanation crossed renderer projection boundary".into());
    }

    let mut overlay = ComparativeWorkbenchOverlay::default();
    for step in &explanation.steps {
        let semantic_ref = step
            .coordinate_ref
            .clone()
            .or_else(|| step.route_ref.clone())
            .ok_or_else(|| {
                format!(
                    "typed explanation step {} has no semantic coordinate/route identity",
                    step.delta_ref
                )
            })?;
        overlay
            .change_layer_by_semantic_ref
            .insert(semantic_ref.clone(), layer_label(step.layer).into());
        overlay
            .explanation_by_semantic_ref
            .insert(semantic_ref.clone(), step.explanation_ref.clone());
        if explanation.minimal_delta_refs.contains(&step.delta_ref) {
            overlay.answer_changing_semantic_refs.insert(semantic_ref);
        }
    }

    // Unresolved explanation refs are delta identities, not guaranteed semantic
    // object IDs. Keep them explicit rather than inventing a graph identity.
    overlay.unresolved_semantic_refs = BTreeSet::new();
    overlay.validate()?;
    Ok(overlay)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_pabai_comparative_regression;

    #[test]
    fn pabai_D_and_C_export_as_applicability_overlays_without_prediction() {
        let receipt = run_pabai_comparative_regression().unwrap();
        let d = workbench_overlay_from_explanation(&receipt.w0_to_w1_explanation).unwrap();
        let c = workbench_overlay_from_explanation(&receipt.w1_to_w2_explanation).unwrap();

        assert_eq!(
            d.change_layer_by_semantic_ref["coordinate:pabai:comparative:defeater"],
            "Applicability"
        );
        assert!(d
            .answer_changing_semantic_refs
            .contains("coordinate:pabai:comparative:defeater"));
        assert_eq!(
            c.change_layer_by_semantic_ref
                ["coordinate:pabai:comparative:counter-defeater"],
            "Applicability"
        );
        assert!(!d.creates_claim_truth);
        assert!(!c.creates_claim_truth);
    }
}
