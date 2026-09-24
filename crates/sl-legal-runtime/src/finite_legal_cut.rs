//! Production mirror of DASHI finite executable legal reachability/cut search.
//!
//! This mirrors the existing Agda kernel: bounded reachability over finite
//! rules/facts, rule disabling, and inclusion-minimal cut search guarded by
//! current reachability.
//!
//! A cut is meaningful only for a route that is currently reachable. If a
//! reviewed exception/defeater already makes the goal unreachable, the runtime
//! returns NoMeaningfulCut and routes to repair/transformation search instead
//! of reporting the vacuous empty cut.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiniteLegalRule {
    pub rule_ref: String,
    pub premise_refs: Vec<String>,
    pub conclusion_ref: String,
    pub exception_refs: Vec<String>,
    pub defeater_refs: Vec<String>,
    pub source_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiniteLegalGraph {
    pub rules: Vec<FiniteLegalRule>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiniteLegalFacts {
    pub proposition_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReachableCutResult {
    Found(BTreeSet<String>),
    NoMeaningfulCut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiniteCutRecomputeReceipt {
    pub old_goal_reachable: bool,
    pub old_cut: ReachableCutResult,
    pub refined_goal_reachable: bool,
    pub refined_cut: ReachableCutResult,
    pub cut_changed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl FiniteLegalRule {
    pub fn validate(&self) -> Result<(), String> {
        if self.rule_ref.trim().is_empty()
            || self.conclusion_ref.trim().is_empty()
            || self.source_ref.trim().is_empty()
        {
            return Err("finite legal rule requires rule/conclusion/source refs".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("finite legal rule crossed non-promotion boundary".into());
        }
        Ok(())
    }
}

impl FiniteLegalGraph {
    pub fn validate(&self) -> Result<(), String> {
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("finite legal graph crossed non-promotion boundary".into());
        }
        let mut refs = BTreeSet::new();
        for rule in &self.rules {
            rule.validate()?;
            if !refs.insert(rule.rule_ref.clone()) {
                return Err(format!("duplicate finite rule ref {}", rule.rule_ref));
            }
        }
        Ok(())
    }
}

pub fn finite_reachable_with_disabled(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    disabled_rule_refs: &BTreeSet<String>,
    goal_ref: &str,
) -> bool {
    if facts.proposition_refs.contains(goal_ref) {
        return true;
    }
    if depth == 0 {
        return false;
    }

    graph.rules.iter().any(|rule| {
        !disabled_rule_refs.contains(&rule.rule_ref)
            && rule.conclusion_ref == goal_ref
            && rule.premise_refs.iter().all(|premise| {
                finite_reachable_with_disabled(
                    depth - 1,
                    graph,
                    facts,
                    disabled_rule_refs,
                    premise,
                )
            })
            && rule.exception_refs.iter().all(|exception| {
                !finite_reachable_with_disabled(
                    depth - 1,
                    graph,
                    facts,
                    disabled_rule_refs,
                    exception,
                )
            })
            && rule.defeater_refs.iter().all(|defeater| {
                !finite_reachable_with_disabled(
                    depth - 1,
                    graph,
                    facts,
                    disabled_rule_refs,
                    defeater,
                )
            })
    })
}

pub fn finite_reachable(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    goal_ref: &str,
) -> bool {
    finite_reachable_with_disabled(depth, graph, facts, &BTreeSet::new(), goal_ref)
}

fn cut_blocks(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    goal_ref: &str,
    cut: &BTreeSet<String>,
) -> bool {
    !finite_reachable_with_disabled(depth, graph, facts, cut, goal_ref)
}

fn every_cut_rule_essential(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    goal_ref: &str,
    cut: &BTreeSet<String>,
) -> bool {
    cut.iter().all(|rule_ref| {
        let mut reduced = cut.clone();
        reduced.remove(rule_ref);
        finite_reachable_with_disabled(depth, graph, facts, &reduced, goal_ref)
    })
}

pub fn is_inclusion_minimal_cut(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    goal_ref: &str,
    cut: &BTreeSet<String>,
) -> bool {
    cut_blocks(depth, graph, facts, goal_ref, cut)
        && every_cut_rule_essential(depth, graph, facts, goal_ref, cut)
}

fn enumerate_subsets(
    values: &[String],
    index: usize,
    current: &mut BTreeSet<String>,
    out: &mut Vec<BTreeSet<String>>,
) {
    if index == values.len() {
        out.push(current.clone());
        return;
    }
    enumerate_subsets(values, index + 1, current, out);
    current.insert(values[index].clone());
    enumerate_subsets(values, index + 1, current, out);
    current.remove(&values[index]);
}

pub fn inclusion_minimal_cuts(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    goal_ref: &str,
) -> Result<Vec<BTreeSet<String>>, String> {
    graph.validate()?;
    if graph.rules.len() > 20 {
        return Err("finite cut search refuses more than 20 rules".into());
    }
    let rule_refs = graph
        .rules
        .iter()
        .map(|rule| rule.rule_ref.clone())
        .collect::<Vec<_>>();
    let mut subsets = Vec::new();
    enumerate_subsets(&rule_refs, 0, &mut BTreeSet::new(), &mut subsets);
    let mut cuts = subsets
        .into_iter()
        .filter(|cut| is_inclusion_minimal_cut(depth, graph, facts, goal_ref, cut))
        .collect::<Vec<_>>();
    cuts.sort_by(|left, right| {
        left.len()
            .cmp(&right.len())
            .then_with(|| left.iter().cmp(right.iter()))
    });
    Ok(cuts)
}

pub fn search_reachable_minimal_cut(
    depth: usize,
    graph: &FiniteLegalGraph,
    facts: &FiniteLegalFacts,
    goal_ref: &str,
) -> Result<ReachableCutResult, String> {
    graph.validate()?;
    if !finite_reachable(depth, graph, facts, goal_ref) {
        return Ok(ReachableCutResult::NoMeaningfulCut);
    }
    let cut = inclusion_minimal_cuts(depth, graph, facts, goal_ref)?
        .into_iter()
        .next()
        .ok_or_else(|| "reachable route had no inclusion-minimal finite cut".to_string())?;
    Ok(ReachableCutResult::Found(cut))
}

pub fn recompute_finite_cut(
    depth: usize,
    old_graph: &FiniteLegalGraph,
    old_facts: &FiniteLegalFacts,
    refined_graph: &FiniteLegalGraph,
    refined_facts: &FiniteLegalFacts,
    goal_ref: &str,
) -> Result<FiniteCutRecomputeReceipt, String> {
    let old_goal_reachable = finite_reachable(depth, old_graph, old_facts, goal_ref);
    let old_cut = search_reachable_minimal_cut(depth, old_graph, old_facts, goal_ref)?;
    let refined_goal_reachable =
        finite_reachable(depth, refined_graph, refined_facts, goal_ref);
    let refined_cut =
        search_reachable_minimal_cut(depth, refined_graph, refined_facts, goal_ref)?;
    let cut_changed = old_cut != refined_cut;

    Ok(FiniteCutRecomputeReceipt {
        old_goal_reachable,
        old_cut,
        refined_goal_reachable,
        refined_cut,
        cut_changed,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(rule_ref: &str, defeaters: &[&str]) -> FiniteLegalGraph {
        FiniteLegalGraph {
            rules: vec![FiniteLegalRule {
                rule_ref: rule_ref.into(),
                premise_refs: vec!["support".into()],
                conclusion_ref: "goal".into(),
                exception_refs: vec![],
                defeater_refs: defeaters.iter().map(|x| (*x).into()).collect(),
                source_ref: "source:test".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            }],
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn facts(values: &[&str]) -> FiniteLegalFacts {
        FiniteLegalFacts {
            proposition_refs: values.iter().map(|x| (*x).into()).collect(),
            candidate_only: true,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn reachable_route_has_singleton_inclusion_minimal_cut() {
        let result =
            search_reachable_minimal_cut(1, &graph("rule:a", &["d"]), &facts(&["support"]), "goal")
                .unwrap();
        assert_eq!(
            result,
            ReachableCutResult::Found(BTreeSet::from(["rule:a".into()]))
        );
    }

    #[test]
    fn already_defeated_route_has_no_meaningful_cut() {
        let result = search_reachable_minimal_cut(
            1,
            &graph("rule:a", &["d"]),
            &facts(&["support", "d"]),
            "goal",
        )
        .unwrap();
        assert_eq!(result, ReachableCutResult::NoMeaningfulCut);
    }

    #[test]
    fn recompute_does_not_freeze_old_cut_identity() {
        let receipt = recompute_finite_cut(
            1,
            &graph("rule:old", &[]),
            &facts(&["support"]),
            &graph("rule:new", &[]),
            &facts(&["support"]),
            "goal",
        )
        .unwrap();
        assert!(receipt.old_goal_reachable);
        assert!(receipt.refined_goal_reachable);
        assert!(receipt.cut_changed);
        assert!(!receipt.creates_claim_truth);
    }
}
