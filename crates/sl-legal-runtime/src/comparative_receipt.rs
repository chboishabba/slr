//! M11 normalized comparative receipt ABI.
//!
//! Domain adapters retain their richer receipts, but every comparison can also
//! emit this stable cross-mode surface:
//!   what changed / did not change;
//!   what the declared consumer/query can see;
//!   what is query relevant;
//!   what changed the answer/proof;
//!   what residuals opened/closed;
//!   why each important delta is present.
//!
//! The ABI never declares a winning party/world and never promotes a
//! comparison into semantic authority or claim truth.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AdversarialPartyComparativeReceipt, AnswerChangingDistinction,
    ComparativeWorldIr, ConsumerFibreComparison, PabaiComparativeReceipt,
    PersonalProfessionalComparativeReceipt, TemporalComparativeReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComparativeReceiptMode {
    LegalRouteChange,
    ConsumerProjectionChange,
    TemporalWorldChange,
    AdversarialPartyChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparativeReceipt {
    pub schema_version: String,
    pub comparison_ref: String,
    pub mode: ComparativeReceiptMode,
    pub left_ref: String,
    pub right_ref: String,
    pub query_ref: Option<String>,
    pub consumer_ref: Option<String>,

    pub shared_coordinate_refs: BTreeSet<String>,
    pub changed_coordinate_refs: BTreeSet<String>,
    pub unchanged_coordinate_refs: BTreeSet<String>,

    pub visible_coordinate_refs: BTreeSet<String>,
    pub blocked_coordinate_refs: BTreeSet<String>,
    pub query_relevant_delta_refs: BTreeSet<String>,
    pub query_irrelevant_delta_refs: BTreeSet<String>,
    pub answer_changing_delta_refs: BTreeSet<String>,

    pub shared_route_refs: BTreeSet<String>,
    pub changed_route_refs: BTreeSet<String>,
    pub changed_residual_refs: BTreeSet<String>,

    pub reason_by_ref: BTreeMap<String, String>,
    pub predicts_outcome: bool,
    pub selects_winner: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ComparativeReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != "sl.comparative_receipt.v0_1"
            || self.comparison_ref.trim().is_empty()
            || self.left_ref.trim().is_empty()
            || self.right_ref.trim().is_empty()
        {
            return Err("comparative receipt has invalid identity/schema".into());
        }
        if self.predicts_outcome
            || self.selects_winner
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("comparative receipt crossed neutrality/non-promotion boundary".into());
        }
        Ok(())
    }
}

fn answer_changing_refs(
    distinction: Option<&AnswerChangingDistinction>,
) -> BTreeSet<String> {
    distinction
        .map(|distinction| distinction.delta_refs.clone())
        .unwrap_or_default()
}

pub fn legal_route_comparative_receipt(
    comparison_ref: &str,
    comparison: &ComparativeWorldIr,
    query_ref: &str,
    consumer_ref: &str,
    distinction: Option<&AnswerChangingDistinction>,
) -> Result<ComparativeReceipt, String> {
    let mut reason_by_ref = BTreeMap::new();
    for delta in &comparison.deltas {
        reason_by_ref.insert(
            delta.delta_ref.clone(),
            format!(
                "kind={:?};role={:?};before={};after={}",
                delta.kind,
                delta.role,
                delta.before_ref.as_deref().unwrap_or("none"),
                delta.after_ref.as_deref().unwrap_or("none")
            ),
        );
    }

    let receipt = ComparativeReceipt {
        schema_version: "sl.comparative_receipt.v0_1".into(),
        comparison_ref: comparison_ref.into(),
        mode: ComparativeReceiptMode::LegalRouteChange,
        left_ref: comparison.left_world_ref.clone(),
        right_ref: comparison.right_world_ref.clone(),
        query_ref: Some(query_ref.into()),
        consumer_ref: Some(consumer_ref.into()),
        shared_coordinate_refs: comparison.shared_coordinate_refs.clone(),
        changed_coordinate_refs: comparison.changed_coordinate_refs.clone(),
        unchanged_coordinate_refs: comparison
            .shared_coordinate_refs
            .difference(&comparison.changed_coordinate_refs)
            .cloned()
            .collect(),
        visible_coordinate_refs: comparison
            .shared_coordinate_refs
            .union(&comparison.changed_coordinate_refs)
            .cloned()
            .collect(),
        blocked_coordinate_refs: BTreeSet::new(),
        query_relevant_delta_refs: comparison.query_relevant_delta_refs.clone(),
        query_irrelevant_delta_refs: comparison.query_irrelevant_delta_refs.clone(),
        answer_changing_delta_refs: answer_changing_refs(distinction),
        shared_route_refs: comparison.shared_route_refs.clone(),
        changed_route_refs: comparison.changed_route_refs.clone(),
        changed_residual_refs: comparison.changed_residual_refs.clone(),
        reason_by_ref,
        predicts_outcome: false,
        selects_winner: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    receipt.validate()?;
    Ok(receipt)
}

fn fibre_receipt(
    comparison_ref: &str,
    fibre: &ConsumerFibreComparison,
) -> Result<ComparativeReceipt, String> {
    let mut blocked = fibre.right_scope_blocked_coordinate_refs.clone();
    blocked.extend(fibre.right_not_ready_coordinate_refs.iter().cloned());
    blocked.extend(
        fibre
            .right_dependency_irrelevant_coordinate_refs
            .iter()
            .cloned(),
    );
    blocked.extend(fibre.right_unreviewed_coordinate_refs.iter().cloned());

    let mut reasons = BTreeMap::new();
    for reference in &fibre.right_scope_blocked_coordinate_refs {
        reasons.insert(reference.clone(), "scope-blocked".into());
    }
    for reference in &fibre.right_not_ready_coordinate_refs {
        reasons.insert(reference.clone(), "not-ready".into());
    }
    for reference in &fibre.right_dependency_irrelevant_coordinate_refs {
        reasons.insert(reference.clone(), "consumer-dependency-irrelevant".into());
    }
    for reference in &fibre.right_unreviewed_coordinate_refs {
        reasons.insert(reference.clone(), "unreviewed".into());
    }

    let mut changed = fibre.left_only_visible_coordinate_refs.clone();
    changed.extend(fibre.right_only_visible_coordinate_refs.iter().cloned());

    let receipt = ComparativeReceipt {
        schema_version: "sl.comparative_receipt.v0_1".into(),
        comparison_ref: comparison_ref.into(),
        mode: ComparativeReceiptMode::ConsumerProjectionChange,
        left_ref: fibre.left_consumer_ref.clone(),
        right_ref: fibre.right_consumer_ref.clone(),
        query_ref: None,
        consumer_ref: Some(fibre.right_consumer_ref.clone()),
        shared_coordinate_refs: fibre.shared_visible_coordinate_refs.clone(),
        changed_coordinate_refs: changed,
        unchanged_coordinate_refs: fibre.shared_visible_coordinate_refs.clone(),
        visible_coordinate_refs: fibre.shared_visible_coordinate_refs.clone(),
        blocked_coordinate_refs: blocked,
        query_relevant_delta_refs: BTreeSet::new(),
        query_irrelevant_delta_refs: BTreeSet::new(),
        answer_changing_delta_refs: BTreeSet::new(),
        shared_route_refs: BTreeSet::new(),
        changed_route_refs: BTreeSet::new(),
        changed_residual_refs: BTreeSet::new(),
        reason_by_ref: reasons,
        predicts_outcome: false,
        selects_winner: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    receipt.validate()?;
    Ok(receipt)
}

pub fn personal_professional_receipts(
    receipt: &PersonalProfessionalComparativeReceipt,
) -> Result<Vec<ComparativeReceipt>, String> {
    [
        ("comparison:personal-lawyer", &receipt.personal_to_lawyer),
        ("comparison:personal-doctor", &receipt.personal_to_doctor),
        ("comparison:personal-advocate", &receipt.personal_to_advocate),
        ("comparison:personal-regulator", &receipt.personal_to_regulator),
    ]
    .into_iter()
    .map(|(reference, fibre)| fibre_receipt(reference, fibre))
    .collect()
}

pub fn temporal_comparative_receipt(
    comparison_ref: &str,
    temporal: &TemporalComparativeReceipt,
) -> Result<ComparativeReceipt, String> {
    let changed_coordinate_refs = temporal
        .deltas
        .iter()
        .filter_map(|delta| delta.coordinate_ref.clone())
        .collect::<BTreeSet<_>>();
    let reason_by_ref = temporal
        .deltas
        .iter()
        .map(|delta| {
            (
                delta.delta_ref.clone(),
                format!(
                    "kind={:?};relevant={}",
                    delta.kind,
                    temporal
                        .query_relevant_delta_refs
                        .contains(&delta.delta_ref)
                ),
            )
        })
        .collect();

    let receipt = ComparativeReceipt {
        schema_version: "sl.comparative_receipt.v0_1".into(),
        comparison_ref: comparison_ref.into(),
        mode: ComparativeReceiptMode::TemporalWorldChange,
        left_ref: temporal.left_world_ref.clone(),
        right_ref: temporal.right_world_ref.clone(),
        query_ref: Some(temporal.query_ref.clone()),
        consumer_ref: None,
        shared_coordinate_refs: BTreeSet::new(),
        changed_coordinate_refs,
        unchanged_coordinate_refs: BTreeSet::new(),
        visible_coordinate_refs: BTreeSet::new(),
        blocked_coordinate_refs: BTreeSet::new(),
        query_relevant_delta_refs: temporal.query_relevant_delta_refs.clone(),
        query_irrelevant_delta_refs: temporal.query_irrelevant_delta_refs.clone(),
        answer_changing_delta_refs: BTreeSet::new(),
        shared_route_refs: BTreeSet::new(),
        changed_route_refs: BTreeSet::new(),
        changed_residual_refs: BTreeSet::new(),
        reason_by_ref,
        predicts_outcome: false,
        selects_winner: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    receipt.validate()?;
    Ok(receipt)
}

pub fn adversarial_party_receipts(
    receipt: &AdversarialPartyComparativeReceipt,
) -> Result<Vec<ComparativeReceipt>, String> {
    let comparisons = [
        ("comparison:yindjibarndi:applicant-state", &receipt.applicant_vs_state),
        ("comparison:yindjibarndi:applicant-fmg", &receipt.applicant_vs_fmg),
        ("comparison:yindjibarndi:state-fmg", &receipt.state_vs_fmg),
    ];

    comparisons
        .into_iter()
        .map(|(comparison_ref, pair)| {
            let mut changed = pair.left_only_coordinate_refs.clone();
            changed.extend(pair.right_only_coordinate_refs.iter().cloned());
            changed.extend(
                pair.coordinates_with_different_treatment
                    .iter()
                    .cloned(),
            );
            let mut reasons = BTreeMap::new();
            for reference in &pair.coordinates_with_different_treatment {
                reasons.insert(reference.clone(), "same-coordinate-different-treatment".into());
            }
            for reference in &pair.left_only_coordinate_refs {
                reasons.insert(reference.clone(), "left-party-only-coordinate".into());
            }
            for reference in &pair.right_only_coordinate_refs {
                reasons.insert(reference.clone(), "right-party-only-coordinate".into());
            }

            let normalized = ComparativeReceipt {
                schema_version: "sl.comparative_receipt.v0_1".into(),
                comparison_ref: comparison_ref.into(),
                mode: ComparativeReceiptMode::AdversarialPartyChange,
                left_ref: format!("{:?}", pair.left_party),
                right_ref: format!("{:?}", pair.right_party),
                query_ref: None,
                consumer_ref: None,
                shared_coordinate_refs: pair.shared_coordinate_refs.clone(),
                changed_coordinate_refs: changed,
                unchanged_coordinate_refs: pair
                    .shared_coordinate_refs
                    .difference(&pair.coordinates_with_different_treatment)
                    .cloned()
                    .collect(),
                visible_coordinate_refs: pair
                    .shared_coordinate_refs
                    .union(&pair.left_only_coordinate_refs)
                    .cloned()
                    .chain(pair.right_only_coordinate_refs.iter().cloned())
                    .collect(),
                blocked_coordinate_refs: BTreeSet::new(),
                query_relevant_delta_refs: BTreeSet::new(),
                query_irrelevant_delta_refs: BTreeSet::new(),
                answer_changing_delta_refs: BTreeSet::new(),
                shared_route_refs: BTreeSet::new(),
                changed_route_refs: BTreeSet::new(),
                changed_residual_refs: BTreeSet::new(),
                reason_by_ref: reasons,
                predicts_outcome: false,
                selects_winner: false,
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            };
            normalized.validate()?;
            Ok(normalized)
        })
        .collect()
}

pub fn pabai_normalized_receipts(
    receipt: &PabaiComparativeReceipt,
) -> Result<Vec<ComparativeReceipt>, String> {
    Ok(vec![
        legal_route_comparative_receipt(
            "comparison:pabai:w0-w1",
            &receipt.w0_to_w1,
            "query:pabai:duty-route",
            "consumer:pabai-climate-duty",
            Some(&receipt.w0_to_w1_distinction),
        )?,
        legal_route_comparative_receipt(
            "comparison:pabai:w1-w2",
            &receipt.w1_to_w2,
            "query:pabai:duty-route",
            "consumer:pabai-climate-duty",
            Some(&receipt.w1_to_w2_distinction),
        )?,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        run_pabai_comparative_regression, run_personal_professional_comparison,
        run_yindjibarndi_party_comparison,
    };

    #[test]
    fn pabai_normalized_receipt_keeps_exact_answer_changing_atom() {
        let pabai = run_pabai_comparative_regression().unwrap();
        let receipts = pabai_normalized_receipts(&pabai).unwrap();
        assert_eq!(receipts.len(), 2);
        assert_eq!(receipts[0].answer_changing_delta_refs.len(), 1);
        assert!(receipts[0]
            .answer_changing_delta_refs
            .contains("delta:pabai:w0-w1:defeater"));
        assert!(!receipts[0].predicts_outcome);
    }

    #[test]
    fn consumer_receipt_explains_hidden_material_instead_of_erasing_it() {
        let fibres = run_personal_professional_comparison().unwrap();
        let receipts = personal_professional_receipts(&fibres).unwrap();
        assert_eq!(receipts.len(), 4);
        assert!(receipts.iter().all(|receipt| {
            !receipt.blocked_coordinate_refs.is_empty()
                && !receipt.creates_claim_truth
        }));
    }

    #[test]
    fn adversarial_receipt_neither_ranks_nor_predicts() {
        let parties = run_yindjibarndi_party_comparison().unwrap();
        let receipts = adversarial_party_receipts(&parties).unwrap();
        assert_eq!(receipts.len(), 3);
        assert!(receipts
            .iter()
            .all(|receipt| !receipt.selects_winner && !receipt.predicts_outcome));
    }
}
