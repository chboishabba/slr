//! M11.1/M11.2 typed extension for the stable comparative receipt ABI.
//!
//! ComparativeReceipt v0_1 remains unchanged. This extension binds a receipt
//! to typed change loci and, when available, a typed inclusion-minimal
//! answer-changing explanation.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ChangeLayer, ChangeLocus, ComparativeReceipt, TypedAnswerChangingExplanation,
    TypedChangeSet,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedComparativeReceiptExtension {
    pub schema_version: String,
    pub comparison_ref: String,
    pub base_schema_version: String,
    pub change_layer_by_delta_ref: BTreeMap<String, ChangeLayer>,
    pub changed_layers: BTreeSet<ChangeLayer>,
    pub invariant_layers: BTreeSet<ChangeLayer>,
    pub justification_refs_by_delta_ref: BTreeMap<String, BTreeSet<String>>,
    pub minimal_answer_changing_delta_refs: BTreeSet<String>,
    pub explanation_ref_by_delta_ref: BTreeMap<String, String>,
    pub unresolved_explanation_delta_refs: BTreeSet<String>,
    pub all_answer_changing_deltas_typed: bool,
    pub claims_causation_beyond_receipts: bool,
    pub predicts_outcome: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl TypedComparativeReceiptExtension {
    pub fn validate(&self, base: &ComparativeReceipt) -> Result<(), String> {
        base.validate()?;
        if self.schema_version != "sl.typed_comparative_receipt_extension.v0_1"
            || self.base_schema_version != base.schema_version
            || self.comparison_ref != base.comparison_ref
        {
            return Err("typed comparative extension does not bind the base receipt".into());
        }
        if self.claims_causation_beyond_receipts
            || self.predicts_outcome
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("typed comparative extension crossed non-promotion boundary".into());
        }
        if !self
            .minimal_answer_changing_delta_refs
            .is_subset(&base.answer_changing_delta_refs)
        {
            return Err("typed extension introduces an answer-changing delta absent from base receipt".into());
        }
        let typed_refs = self
            .change_layer_by_delta_ref
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        if self.all_answer_changing_deltas_typed
            && !base.answer_changing_delta_refs.is_subset(&typed_refs)
        {
            return Err("extension marks all answer-changing deltas typed but a locus is missing".into());
        }
        Ok(())
    }
}

pub fn typed_comparative_receipt_extension(
    base: &ComparativeReceipt,
    change_set: &TypedChangeSet,
    explanation: Option<&TypedAnswerChangingExplanation>,
) -> Result<TypedComparativeReceiptExtension, String> {
    base.validate()?;
    if change_set.comparison_ref != base.comparison_ref {
        return Err("typed change set comparison_ref does not match base receipt".into());
    }

    let mut change_layer_by_delta_ref = BTreeMap::new();
    let mut justification_refs_by_delta_ref = BTreeMap::new();
    for locus in &change_set.loci {
        change_layer_by_delta_ref.insert(locus.delta.delta_ref.clone(), locus.layer);
        justification_refs_by_delta_ref.insert(
            locus.delta.delta_ref.clone(),
            locus.justification_refs.clone(),
        );
    }

    let (
        minimal_answer_changing_delta_refs,
        explanation_ref_by_delta_ref,
        unresolved_explanation_delta_refs,
        all_answer_changing_deltas_typed,
        claims_causation_beyond_receipts,
        predicts_outcome,
    ) = if let Some(explanation) = explanation {
        if explanation.query_ref != base.query_ref.clone().unwrap_or_default() {
            return Err("typed explanation query does not match base comparative receipt".into());
        }
        let refs = explanation
            .steps
            .iter()
            .map(|step| (step.delta_ref.clone(), step.explanation_ref.clone()))
            .collect::<BTreeMap<_, _>>();
        (
            explanation.minimal_delta_refs.clone(),
            refs,
            explanation.unresolved_delta_refs.clone(),
            explanation.all_minimal_deltas_typed,
            explanation.claims_causation_beyond_receipts,
            explanation.predicts_outcome,
        )
    } else {
        (
            BTreeSet::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            base.answer_changing_delta_refs.is_empty(),
            false,
            false,
        )
    };

    let extension = TypedComparativeReceiptExtension {
        schema_version: "sl.typed_comparative_receipt_extension.v0_1".into(),
        comparison_ref: base.comparison_ref.clone(),
        base_schema_version: base.schema_version.clone(),
        change_layer_by_delta_ref,
        changed_layers: change_set.changed_layers.clone(),
        invariant_layers: change_set.invariant_layers.clone(),
        justification_refs_by_delta_ref,
        minimal_answer_changing_delta_refs,
        explanation_ref_by_delta_ref,
        unresolved_explanation_delta_refs,
        all_answer_changing_deltas_typed,
        claims_causation_beyond_receipts,
        predicts_outcome,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    extension.validate(base)?;
    Ok(extension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compile_typed_change_set, legal_route_comparative_receipt,
        run_pabai_comparative_regression, typed_locus,
    };

    #[test]
    fn pabai_typed_extension_binds_stable_v0_1_receipt_without_schema_mutation() {
        let pabai = run_pabai_comparative_regression().unwrap();
        let base = legal_route_comparative_receipt(
            "comparison:pabai:w0-w1",
            &pabai.w0_to_w1,
            "query:pabai:duty-route",
            "consumer:pabai-climate-duty",
            Some(&pabai.w0_to_w1_distinction),
        )
        .unwrap();
        let change_set = compile_typed_change_set(
            "comparison:pabai:w0-w1",
            [pabai.w0_to_w1_locus.clone()],
            [],
        )
        .unwrap();
        let typed = typed_comparative_receipt_extension(
            &base,
            &change_set,
            Some(&pabai.w0_to_w1_explanation),
        )
        .unwrap();

        assert_eq!(base.schema_version, "sl.comparative_receipt.v0_1");
        assert_eq!(
            typed.change_layer_by_delta_ref["delta:pabai:w0-w1:defeater"],
            ChangeLayer::Applicability
        );
        assert_eq!(
            typed.minimal_answer_changing_delta_refs,
            base.answer_changing_delta_refs
        );
        assert!(typed.all_answer_changing_deltas_typed);
        assert!(!typed.predicts_outcome);
    }

    #[test]
    fn extension_refuses_change_set_for_different_comparison() {
        let pabai = run_pabai_comparative_regression().unwrap();
        let base = legal_route_comparative_receipt(
            "comparison:pabai:w0-w1",
            &pabai.w0_to_w1,
            "query:pabai:duty-route",
            "consumer:pabai-climate-duty",
            Some(&pabai.w0_to_w1_distinction),
        )
        .unwrap();
        let locus = typed_locus(
            "locus:pabai",
            pabai.w0_to_w1_locus.delta.clone(),
            ChangeLayer::Applicability,
            None,
            BTreeSet::new(),
            BTreeSet::new(),
        )
        .unwrap();
        let wrong = compile_typed_change_set("comparison:other", [locus], []).unwrap();
        assert!(typed_comparative_receipt_extension(&base, &wrong, None).is_err());
    }
}
