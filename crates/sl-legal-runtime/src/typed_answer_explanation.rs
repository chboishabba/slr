//! M11.2 typed minimal answer-changing explanation.
//!
//! The existing AnswerChangingDistinction says which input deltas are
//! inclusion-minimal. This layer explains where those deltas occurred and how
//! they connect to the changed route/answer without upgrading correlation into
//! causation beyond the caller-supplied dependency/cause receipt.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{AnswerChangingDistinction, ChangeLayer, ChangeLocus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerChangeStep {
    pub delta_ref: String,
    pub layer: ChangeLayer,
    pub coordinate_ref: Option<String>,
    pub route_ref: Option<String>,
    pub before_ref: Option<String>,
    pub after_ref: Option<String>,
    pub cause_refs: BTreeSet<String>,
    pub explanation_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedAnswerChangingExplanation {
    pub query_ref: String,
    pub baseline_answer_ref: String,
    pub target_answer_ref: String,
    pub minimal_delta_refs: BTreeSet<String>,
    pub steps: Vec<AnswerChangeStep>,
    pub unresolved_delta_refs: BTreeSet<String>,
    pub all_minimal_deltas_typed: bool,
    pub claims_causation_beyond_receipts: bool,
    pub predicts_outcome: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn explain_answer_changing_distinction(
    distinction: &AnswerChangingDistinction,
    loci: &[ChangeLocus],
    explanation_refs: &BTreeMap<String, String>,
) -> Result<TypedAnswerChangingExplanation, String> {
    let loci_by_delta = loci
        .iter()
        .map(|locus| (locus.delta.delta_ref.clone(), locus))
        .collect::<BTreeMap<_, _>>();

    let mut steps = Vec::new();
    let mut unresolved = BTreeSet::new();
    for delta_ref in &distinction.delta_refs {
        let Some(locus) = loci_by_delta.get(delta_ref) else {
            unresolved.insert(delta_ref.clone());
            continue;
        };
        locus.validate()?;
        let Some(explanation_ref) = explanation_refs.get(delta_ref) else {
            unresolved.insert(delta_ref.clone());
            continue;
        };
        if explanation_ref.trim().is_empty() {
            unresolved.insert(delta_ref.clone());
            continue;
        }
        steps.push(AnswerChangeStep {
            delta_ref: delta_ref.clone(),
            layer: locus.layer,
            coordinate_ref: locus.delta.coordinate_ref.clone(),
            route_ref: locus.delta.route_ref.clone(),
            before_ref: locus.delta.before_ref.clone(),
            after_ref: locus.delta.after_ref.clone(),
            cause_refs: locus.delta.cause_refs.clone(),
            explanation_ref: explanation_ref.clone(),
        });
    }

    Ok(TypedAnswerChangingExplanation {
        query_ref: distinction.query_ref.clone(),
        baseline_answer_ref: distinction.baseline_answer_ref.clone(),
        target_answer_ref: distinction.target_answer_ref.clone(),
        minimal_delta_refs: distinction.delta_refs.clone(),
        steps,
        unresolved_delta_refs: unresolved.clone(),
        all_minimal_deltas_typed: unresolved.is_empty(),
        claims_causation_beyond_receipts: false,
        predicts_outcome: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        typed_locus, ComparativeDelta, ComparativeDeltaKind, ComparativeDeltaRole,
    };

    #[test]
    fn minimal_defeater_can_be_explained_at_applicability_layer() {
        let distinction = AnswerChangingDistinction {
            query_ref: "query:pabai".into(),
            baseline_answer_ref: "reachable".into(),
            target_answer_ref: "defeated".into(),
            delta_refs: BTreeSet::from(["delta:D".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let delta = ComparativeDelta {
            delta_ref: "delta:D".into(),
            kind: ComparativeDeltaKind::DefeaterAdded,
            role: ComparativeDeltaRole::WorldInput,
            coordinate_ref: Some("coordinate:D".into()),
            route_ref: Some("route:R".into()),
            residual_ref: None,
            before_ref: None,
            after_ref: Some("present".into()),
            cause_refs: BTreeSet::from(["receipt:reviewed-defeater".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let locus = typed_locus(
            "locus:D",
            delta,
            ChangeLayer::Applicability,
            Some("legal-defeater".into()),
            BTreeSet::new(),
            BTreeSet::from(["receipt:reviewed-defeater".into()]),
        )
        .unwrap();
        let explanation = explain_answer_changing_distinction(
            &distinction,
            &[locus],
            &BTreeMap::from([(
                "delta:D".into(),
                "reviewed defeater enters applicability layer and blocks route R".into(),
            )]),
        )
        .unwrap();

        assert!(explanation.all_minimal_deltas_typed);
        assert_eq!(explanation.steps.len(), 1);
        assert_eq!(explanation.steps[0].layer, ChangeLayer::Applicability);
        assert!(!explanation.claims_causation_beyond_receipts);
        assert!(!explanation.predicts_outcome);
    }

    #[test]
    fn missing_locus_stays_explicitly_unresolved() {
        let distinction = AnswerChangingDistinction {
            query_ref: "query:q".into(),
            baseline_answer_ref: "a".into(),
            target_answer_ref: "b".into(),
            delta_refs: BTreeSet::from(["delta:unknown".into()]),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        };
        let explanation = explain_answer_changing_distinction(
            &distinction,
            &[],
            &BTreeMap::new(),
        )
        .unwrap();
        assert!(!explanation.all_minimal_deltas_typed);
        assert!(explanation
            .unresolved_delta_refs
            .contains("delta:unknown"));
    }
}
