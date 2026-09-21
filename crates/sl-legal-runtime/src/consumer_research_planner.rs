//! Admissible/Pareto selection for consumer-directed research residuals.
//!
//! Candidate research moves are eligible only after hard consumer relevance and
//! non-promotion checks.  Cost/priority is used only inside that admissible
//! stratum and is never a legal-truth or authority ranking.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{ConsumerAxis, ExactConsumerResidual};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResearchCostVector {
    pub network_requests: u64,
    pub review_units: u64,
    pub expected_source_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissibleResearchCandidate {
    pub candidate_ref: String,
    pub residual: ExactConsumerResidual,
    pub recovers_axes: BTreeSet<ConsumerAxis>,
    pub expected_query_gain: u64,
    pub cost: ResearchCostVector,
    pub admissible: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
    pub priority_is_legal_truth_rank: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissibleResearchFrontier {
    pub selected: Vec<AdmissibleResearchCandidate>,
    pub rejected_refs: Vec<String>,
    pub subsumed_refs: Vec<String>,
    pub priority_is_legal_truth_rank: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn weakly_cheaper(left: ResearchCostVector, right: ResearchCostVector) -> bool {
    left.network_requests <= right.network_requests
        && left.review_units <= right.review_units
        && left.expected_source_bytes <= right.expected_source_bytes
}

fn strictly_better_or_equal(
    left: &AdmissibleResearchCandidate,
    right: &AdmissibleResearchCandidate,
) -> bool {
    left.recovers_axes.is_superset(&right.recovers_axes)
        && left.expected_query_gain >= right.expected_query_gain
        && weakly_cheaper(left.cost, right.cost)
        && (
            left.recovers_axes != right.recovers_axes
                || left.expected_query_gain > right.expected_query_gain
                || left.cost != right.cost
        )
}

fn equivalence_key(candidate: &AdmissibleResearchCandidate) -> (String, ConsumerAxis) {
    (
        candidate.residual.demand.target_ref.clone(),
        candidate.residual.lost_axis,
    )
}

pub fn admissible_research_frontier(
    candidates: &[AdmissibleResearchCandidate],
) -> Result<AdmissibleResearchFrontier, String> {
    let mut rejected_refs = Vec::new();
    let mut eligible = Vec::new();

    for candidate in candidates {
        if candidate.candidate_ref.trim().is_empty() {
            return Err("research candidate_ref must be non-empty".into());
        }
        if !candidate.candidate_only
            || candidate.creates_semantic_authority
            || candidate.creates_claim_truth
            || candidate.priority_is_legal_truth_rank
            || !candidate.residual.candidate_only
            || candidate.residual.creates_semantic_authority
            || candidate.residual.creates_claim_truth
        {
            return Err(format!(
                "research candidate {} crossed governance boundary",
                candidate.candidate_ref
            ));
        }
        if !candidate.admissible
            || candidate.expected_query_gain == 0
            || candidate.recovers_axes.is_empty()
            || !candidate.recovers_axes.contains(&candidate.residual.lost_axis)
        {
            rejected_refs.push(candidate.candidate_ref.clone());
            continue;
        }
        eligible.push(candidate.clone());
    }

    // First collapse exact consumer-equivalent candidates.  Within one target
    // and lost axis, keep only candidates not Pareto-dominated by another.
    let mut groups: BTreeMap<(String, ConsumerAxis), Vec<AdmissibleResearchCandidate>> =
        BTreeMap::new();
    for candidate in eligible {
        groups
            .entry(equivalence_key(&candidate))
            .or_default()
            .push(candidate);
    }

    let mut selected = Vec::new();
    let mut subsumed_refs = Vec::new();
    for group in groups.into_values() {
        for candidate in &group {
            let dominated = group.iter().any(|other| {
                other.candidate_ref != candidate.candidate_ref
                    && strictly_better_or_equal(other, candidate)
            });
            if dominated {
                subsumed_refs.push(candidate.candidate_ref.clone());
            } else {
                selected.push(candidate.clone());
            }
        }
    }

    // Stable research order only after admissibility/Pareto filtering.
    selected.sort_by(|left, right| {
        right
            .expected_query_gain
            .cmp(&left.expected_query_gain)
            .then_with(|| left.cost.review_units.cmp(&right.cost.review_units))
            .then_with(|| left.cost.network_requests.cmp(&right.cost.network_requests))
            .then_with(|| {
                left.cost
                    .expected_source_bytes
                    .cmp(&right.cost.expected_source_bytes)
            })
            .then_with(|| left.candidate_ref.cmp(&right.candidate_ref))
    });
    rejected_refs.sort();
    rejected_refs.dedup();
    subsumed_refs.sort();
    subsumed_refs.dedup();

    Ok(AdmissibleResearchFrontier {
        selected,
        rejected_refs,
        subsumed_refs,
        priority_is_legal_truth_rank: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsumerResearchStopKind {
    TheoremBackedConsumerAdequate,
    ExplicitlyUnresolved,
    BudgetExhausted,
    CurrentFrontierClosedWithoutAdequacy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerResearchStopReceipt {
    pub kind: ConsumerResearchStopKind,
    pub query_ref: String,
    pub theorem_ref: Option<String>,
    pub unresolved_refs: BTreeSet<String>,
    pub current_frontier_closed: bool,
    pub consumer_adequate_formally_proved: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ConsumerResearchStopReceipt {
    pub fn theorem_adequate(
        query_ref: impl Into<String>,
        theorem_ref: impl Into<String>,
    ) -> Result<Self, String> {
        let query_ref = query_ref.into();
        let theorem_ref = theorem_ref.into();
        if query_ref.trim().is_empty() || theorem_ref.trim().is_empty() {
            return Err("theorem-backed stop requires query and theorem refs".into());
        }
        Ok(Self {
            kind: ConsumerResearchStopKind::TheoremBackedConsumerAdequate,
            query_ref,
            theorem_ref: Some(theorem_ref),
            unresolved_refs: BTreeSet::new(),
            current_frontier_closed: true,
            consumer_adequate_formally_proved: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }

    pub fn explicitly_unresolved(
        query_ref: impl Into<String>,
        unresolved_refs: BTreeSet<String>,
    ) -> Result<Self, String> {
        let query_ref = query_ref.into();
        if query_ref.trim().is_empty() || unresolved_refs.is_empty() {
            return Err("explicitly unresolved stop requires query and residual refs".into());
        }
        Ok(Self {
            kind: ConsumerResearchStopKind::ExplicitlyUnresolved,
            query_ref,
            theorem_ref: None,
            unresolved_refs,
            current_frontier_closed: true,
            consumer_adequate_formally_proved: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }

    pub fn budget_exhausted(query_ref: impl Into<String>) -> Result<Self, String> {
        let query_ref = query_ref.into();
        if query_ref.trim().is_empty() {
            return Err("budget stop requires query_ref".into());
        }
        Ok(Self {
            kind: ConsumerResearchStopKind::BudgetExhausted,
            query_ref,
            theorem_ref: None,
            unresolved_refs: BTreeSet::new(),
            current_frontier_closed: false,
            consumer_adequate_formally_proved: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }

    pub fn frontier_closed_without_adequacy(
        query_ref: impl Into<String>,
    ) -> Result<Self, String> {
        let query_ref = query_ref.into();
        if query_ref.trim().is_empty() {
            return Err("frontier closure requires query_ref".into());
        }
        Ok(Self {
            kind: ConsumerResearchStopKind::CurrentFrontierClosedWithoutAdequacy,
            query_ref,
            theorem_ref: None,
            unresolved_refs: BTreeSet::new(),
            current_frontier_closed: true,
            consumer_adequate_formally_proved: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ConsumerResearchDemand, ConsumerResearchDemandKind, ExactConsumerResidual,
    };

    fn residual(axis: ConsumerAxis) -> ExactConsumerResidual {
        ExactConsumerResidual {
            residual_ref: format!("residual:{axis:?}"),
            query_ref: "query:fixture".into(),
            projection_digest: "sha256:projection".into(),
            lost_axis: axis,
            demand: ConsumerResearchDemand {
                axis,
                kind: match axis {
                    ConsumerAxis::Treatment => ConsumerResearchDemandKind::ReviewTreatment,
                    _ => ConsumerResearchDemandKind::AcquireSource,
                },
                target_ref: "case:fixture".into(),
                reason_ref: "reason:fixture".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            },
            nonfactorability_theorem_ref: "theorem:defect".into(),
            witness_refs: vec!["world:left".into(), "world:right".into()],
            reason_ref: "reason:fixture".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn candidate(
        name: &str,
        gain: u64,
        review: u64,
        network: u64,
    ) -> AdmissibleResearchCandidate {
        AdmissibleResearchCandidate {
            candidate_ref: name.into(),
            residual: residual(ConsumerAxis::Treatment),
            recovers_axes: BTreeSet::from([ConsumerAxis::Treatment]),
            expected_query_gain: gain,
            cost: ResearchCostVector {
                network_requests: network,
                review_units: review,
                expected_source_bytes: 100,
            },
            admissible: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
            priority_is_legal_truth_rank: false,
        }
    }

    #[test]
    fn dominated_equivalent_residual_move_is_subsumed() {
        let weak = candidate("weak", 1, 5, 3);
        let strong = candidate("strong", 2, 2, 1);
        let frontier = admissible_research_frontier(&[weak, strong]).unwrap();
        assert_eq!(frontier.selected.len(), 1);
        assert_eq!(frontier.selected[0].candidate_ref, "strong");
        assert_eq!(frontier.subsumed_refs, vec!["weak"]);
        assert!(!frontier.priority_is_legal_truth_rank);
    }

    #[test]
    fn inadmissible_or_zero_gain_moves_are_rejected_before_ranking() {
        let mut inadmissible = candidate("inadmissible", 10, 0, 0);
        inadmissible.admissible = false;
        let zero = candidate("zero", 0, 0, 0);
        let frontier = admissible_research_frontier(&[inadmissible, zero]).unwrap();
        assert!(frontier.selected.is_empty());
        assert_eq!(frontier.rejected_refs.len(), 2);
    }

    #[test]
    fn frontier_closed_is_not_theorem_adequate_stop() {
        let stop =
            ConsumerResearchStopReceipt::frontier_closed_without_adequacy("query:fixture")
                .unwrap();
        assert_eq!(
            stop.kind,
            ConsumerResearchStopKind::CurrentFrontierClosedWithoutAdequacy
        );
        assert!(stop.current_frontier_closed);
        assert!(!stop.consumer_adequate_formally_proved);

        let adequate =
            ConsumerResearchStopReceipt::theorem_adequate("query:fixture", "theorem:factor")
                .unwrap();
        assert!(adequate.current_frontier_closed);
        assert!(adequate.consumer_adequate_formally_proved);
        assert_eq!(
            adequate.kind,
            ConsumerResearchStopKind::TheoremBackedConsumerAdequate
        );
    }
}
