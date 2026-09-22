//! M8.1 live Yindjibarndi finite-cut / recompute experiment.
//!
//! This composes the source-grounded Yindjibarndi treatment packet with the
//! production finite legal cut kernel.  The snapshots mirror the Agda owner:
//!
//! support-only -> reachable + meaningful cut
//! all reviewed defeaters -> unreachable + no meaningful cut
//! Mabo-only distinction -> still unreachable
//! counterfactual full repair -> reachable + recomputed cut with new identity
//!
//! The final snapshot is deliberately candidate-only.  It demonstrates
//! recomputation mechanics; it is not a statement that the remaining
//! Yunupingu scope objections have been paid in the real matter.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{
    finite_reachable, recompute_finite_cut, search_reachable_minimal_cut,
    FiniteCutRecomputeReceipt, FiniteLegalFacts, FiniteLegalGraph,
    FiniteLegalRule, ReachableCutResult,
};

pub const YINDJIBARNDI_FINITE_GOAL: &str =
    "proposition:yindjibarndi:compensable-acquisition-candidate";
pub const YINDJIBARNDI_SUPPORT: &str =
    "proposition:yindjibarndi:yunupingu-supports-acquisition-through-diminution-or-impairment";
pub const YINDJIBARNDI_STATE_YUNUPINGU_SCOPE: &str =
    "proposition:yindjibarndi:yunupingu-did-not-decide-nonextinguishment-act-acquisition";
pub const YINDJIBARNDI_FMG_YUNUPINGU_SCOPE: &str =
    "proposition:yindjibarndi:yunupingu-does-not-establish-every-mining-lease-acquisition";
pub const YINDJIBARNDI_FMG_MABO_DEFEATER: &str =
    "proposition:yindjibarndi:fmg-invokes-mabo-brennan-60-against-acquisition";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YindjibarndiFiniteCutExperiment {
    pub support_reachable: bool,
    pub support_cut: ReachableCutResult,
    pub defeated_reachable: bool,
    pub defeated_cut: ReachableCutResult,
    pub mabo_repair_reachable: bool,
    pub mabo_repair_cut: ReachableCutResult,
    pub fully_repaired_candidate_reachable: bool,
    pub fully_repaired_candidate_cut: ReachableCutResult,
    pub recompute_receipt: FiniteCutRecomputeReceipt,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn rule(rule_ref: &str, defeaters: &[&str]) -> FiniteLegalRule {
    FiniteLegalRule {
        rule_ref: rule_ref.into(),
        premise_refs: vec![YINDJIBARNDI_SUPPORT.into()],
        conclusion_ref: YINDJIBARNDI_FINITE_GOAL.into(),
        exception_refs: vec![],
        defeater_refs: defeaters.iter().map(|value| (*value).into()).collect(),
        source_ref: "fedcourt:WAD37/2022:yindjibarndi-source-packet".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn graph(rule: FiniteLegalRule) -> FiniteLegalGraph {
    FiniteLegalGraph {
        rules: vec![rule],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn facts(values: &[&str]) -> FiniteLegalFacts {
    FiniteLegalFacts {
        proposition_refs: values.iter().map(|value| (*value).into()).collect(),
        candidate_only: true,
        creates_claim_truth: false,
    }
}

pub fn yindjibarndi_support_snapshot() -> (FiniteLegalGraph, FiniteLegalFacts) {
    (
        graph(rule(
            "rule:yindjibarndi:support-route",
            &[
                YINDJIBARNDI_STATE_YUNUPINGU_SCOPE,
                YINDJIBARNDI_FMG_YUNUPINGU_SCOPE,
                YINDJIBARNDI_FMG_MABO_DEFEATER,
            ],
        )),
        facts(&[YINDJIBARNDI_SUPPORT]),
    )
}

pub fn yindjibarndi_defeated_snapshot() -> (FiniteLegalGraph, FiniteLegalFacts) {
    let (graph, _) = yindjibarndi_support_snapshot();
    (
        graph,
        facts(&[
            YINDJIBARNDI_SUPPORT,
            YINDJIBARNDI_STATE_YUNUPINGU_SCOPE,
            YINDJIBARNDI_FMG_YUNUPINGU_SCOPE,
            YINDJIBARNDI_FMG_MABO_DEFEATER,
        ]),
    )
}

pub fn yindjibarndi_mabo_only_repair_snapshot() -> (FiniteLegalGraph, FiniteLegalFacts) {
    (
        graph(rule(
            "rule:yindjibarndi:after-mabo-distinction",
            &[
                YINDJIBARNDI_STATE_YUNUPINGU_SCOPE,
                YINDJIBARNDI_FMG_YUNUPINGU_SCOPE,
            ],
        )),
        facts(&[
            YINDJIBARNDI_SUPPORT,
            YINDJIBARNDI_STATE_YUNUPINGU_SCOPE,
            YINDJIBARNDI_FMG_YUNUPINGU_SCOPE,
        ]),
    )
}

pub fn yindjibarndi_fully_repaired_candidate_snapshot(
) -> (FiniteLegalGraph, FiniteLegalFacts) {
    (
        graph(rule(
            "rule:yindjibarndi:fully-repaired-candidate",
            &[],
        )),
        facts(&[YINDJIBARNDI_SUPPORT]),
    )
}

pub fn run_yindjibarndi_finite_cut_experiment(
) -> Result<YindjibarndiFiniteCutExperiment, String> {
    let (support_graph, support_facts) = yindjibarndi_support_snapshot();
    let (defeated_graph, defeated_facts) = yindjibarndi_defeated_snapshot();
    let (mabo_graph, mabo_facts) = yindjibarndi_mabo_only_repair_snapshot();
    let (full_graph, full_facts) = yindjibarndi_fully_repaired_candidate_snapshot();

    let support_reachable =
        finite_reachable(1, &support_graph, &support_facts, YINDJIBARNDI_FINITE_GOAL);
    let support_cut = search_reachable_minimal_cut(
        1,
        &support_graph,
        &support_facts,
        YINDJIBARNDI_FINITE_GOAL,
    )?;

    let defeated_reachable =
        finite_reachable(1, &defeated_graph, &defeated_facts, YINDJIBARNDI_FINITE_GOAL);
    let defeated_cut = search_reachable_minimal_cut(
        1,
        &defeated_graph,
        &defeated_facts,
        YINDJIBARNDI_FINITE_GOAL,
    )?;

    let mabo_repair_reachable =
        finite_reachable(1, &mabo_graph, &mabo_facts, YINDJIBARNDI_FINITE_GOAL);
    let mabo_repair_cut = search_reachable_minimal_cut(
        1,
        &mabo_graph,
        &mabo_facts,
        YINDJIBARNDI_FINITE_GOAL,
    )?;

    let fully_repaired_candidate_reachable =
        finite_reachable(1, &full_graph, &full_facts, YINDJIBARNDI_FINITE_GOAL);
    let fully_repaired_candidate_cut = search_reachable_minimal_cut(
        1,
        &full_graph,
        &full_facts,
        YINDJIBARNDI_FINITE_GOAL,
    )?;

    let recompute_receipt = recompute_finite_cut(
        1,
        &support_graph,
        &support_facts,
        &full_graph,
        &full_facts,
        YINDJIBARNDI_FINITE_GOAL,
    )?;

    Ok(YindjibarndiFiniteCutExperiment {
        support_reachable,
        support_cut,
        defeated_reachable,
        defeated_cut,
        mabo_repair_reachable,
        mabo_repair_cut,
        fully_repaired_candidate_reachable,
        fully_repaired_candidate_cut,
        recompute_receipt,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_experiment_recomputes_only_after_reachability_returns() {
        let experiment = run_yindjibarndi_finite_cut_experiment().unwrap();

        assert!(experiment.support_reachable);
        assert!(matches!(experiment.support_cut, ReachableCutResult::Found(_)));

        assert!(!experiment.defeated_reachable);
        assert_eq!(
            experiment.defeated_cut,
            ReachableCutResult::NoMeaningfulCut
        );

        assert!(!experiment.mabo_repair_reachable);
        assert_eq!(
            experiment.mabo_repair_cut,
            ReachableCutResult::NoMeaningfulCut
        );

        assert!(experiment.fully_repaired_candidate_reachable);
        assert!(matches!(
            experiment.fully_repaired_candidate_cut,
            ReachableCutResult::Found(_)
        ));

        assert!(experiment.recompute_receipt.cut_changed);
        assert!(!experiment.creates_claim_truth);
    }

    #[test]
    fn recomputed_cut_uses_new_rule_identity() {
        let experiment = run_yindjibarndi_finite_cut_experiment().unwrap();
        let old = match experiment.recompute_receipt.old_cut {
            ReachableCutResult::Found(ref cut) => cut,
            ReachableCutResult::NoMeaningfulCut => panic!("old route should be reachable"),
        };
        let new = match experiment.recompute_receipt.refined_cut {
            ReachableCutResult::Found(ref cut) => cut,
            ReachableCutResult::NoMeaningfulCut => panic!("refined candidate should be reachable"),
        };
        assert_eq!(
            old,
            &BTreeSet::from(["rule:yindjibarndi:support-route".into()])
        );
        assert_eq!(
            new,
            &BTreeSet::from(["rule:yindjibarndi:fully-repaired-candidate".into()])
        );
        assert_ne!(old, new);
    }
}
