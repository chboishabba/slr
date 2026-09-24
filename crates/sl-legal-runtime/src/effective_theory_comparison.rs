//! M11.1 / S26.13 effective-theory lineage comparison.
//!
//! Theory succession is not encoded as boolean replacement. A changed theory
//! coordinate may preserve controlled effective adequacy on a declared regime
//! while remaining non-identical globally and non-identical to the world.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::ChangeLayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TheoryRelationStatus {
    ExactRecovery,
    ControlledEffectiveRecovery,
    AsymptoticRecovery,
    EmpiricalEffectiveAgreement,
    ObservationallyEquivalentHere,
    UnresolvedRelation,
    RefutedOnDeclaredTest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectiveTheoryComparativeReceipt {
    pub comparison_ref: String,
    pub earlier_theory_ref: String,
    pub later_theory_ref: String,
    pub relation: TheoryRelationStatus,
    pub valid_regime_refs: BTreeSet<String>,
    pub outside_regime_refs: BTreeSet<String>,
    pub preserved_adequacy_refs: BTreeSet<String>,
    pub required_residual_refs: BTreeSet<String>,
    pub changed_layers: BTreeSet<ChangeLayer>,
    pub invariant_layers: BTreeSet<ChangeLayer>,
    pub world_identity_inferred: bool,
    pub global_theory_identity: bool,
    pub newer_theory_deletes_older: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl EffectiveTheoryComparativeReceipt {
    pub fn validate(&self) -> Result<(), String> {
        if self.comparison_ref.trim().is_empty()
            || self.earlier_theory_ref.trim().is_empty()
            || self.later_theory_ref.trim().is_empty()
        {
            return Err("effective-theory comparison requires identity refs".into());
        }
        if self.world_identity_inferred
            || self.global_theory_identity
            || self.newer_theory_deletes_older
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("effective-theory comparison crossed lineage/non-promotion boundary".into());
        }
        if self.relation == TheoryRelationStatus::ControlledEffectiveRecovery
            && self.required_residual_refs.is_empty()
        {
            return Err("controlled effective recovery requires a residual/limit witness".into());
        }
        Ok(())
    }
}

pub fn newton_gr_effective_lineage(
) -> Result<EffectiveTheoryComparativeReceipt, String> {
    let receipt = EffectiveTheoryComparativeReceipt {
        comparison_ref: "comparison:gravity:newton-gr-effective-lineage".into(),
        earlier_theory_ref: "theory:newtonian-representation".into(),
        later_theory_ref: "theory:relativistic-representation".into(),
        relation: TheoryRelationStatus::ControlledEffectiveRecovery,
        valid_regime_refs: BTreeSet::from([
            "regime:gravity:weak-field".into(),
            "regime:gravity:low-speed".into(),
        ]),
        outside_regime_refs: BTreeSet::from([
            "regime:gravity:strong-field-relativistic".into(),
        ]),
        preserved_adequacy_refs: BTreeSet::from([
            "consumer:coarse-fall-query".into(),
        ]),
        required_residual_refs: BTreeSet::from([
            "DASHI.Physics.Laws.EffectiveTheoryLineageExact:controlled-residual-required"
                .into(),
        ]),
        changed_layers: BTreeSet::from([ChangeLayer::Theory]),
        invariant_layers: BTreeSet::from([ChangeLayer::World]),
        world_identity_inferred: false,
        global_theory_identity: false,
        newer_theory_deletes_older: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    };
    receipt.validate()?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theory_lineage_preserves_restricted_adequacy_without_global_identity() {
        let receipt = newton_gr_effective_lineage().unwrap();
        assert_eq!(
            receipt.relation,
            TheoryRelationStatus::ControlledEffectiveRecovery
        );
        assert!(receipt
            .valid_regime_refs
            .contains("regime:gravity:weak-field"));
        assert!(receipt
            .outside_regime_refs
            .contains("regime:gravity:strong-field-relativistic"));
        assert!(!receipt.world_identity_inferred);
        assert!(!receipt.global_theory_identity);
        assert!(!receipt.newer_theory_deletes_older);
        assert!(!receipt.creates_claim_truth);
    }
}
