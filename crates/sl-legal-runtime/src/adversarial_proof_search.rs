//! S20 adversarial legal proof-search orchestration.
//!
//! This module does not invent a second search algebra.  It composes the
//! existing ProofFrontier, SearchHypothesis family and reviewed reasoning
//! vocabulary into an explicit support -> defeat -> counter-defeat loop.
//!
//! "Reachable", "defeated" and "reopened" below are candidate proof-route
//! states.  They never imply adjudicative truth, liability or final judgment.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_proof_search_loop::{
    frontier::{ProofFrontier, ProofResidual, ResidualStatus},
    hypothesis::{family_for_residual, SearchHypothesis, SearchHypothesisKind},
    reasoning::{CitationUse, ReasoningRole},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LegalAtomKind {
    Fact,
    Element,
    Rule,
    Exception,
    Defeater,
    CounterDefeater,
    Burden,
    Remedy,
    Authority,
    WrongTypeDiscriminator,
    FactualDiscriminator,
    TemporalDiscriminator,
    JurisdictionDiscriminator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalAtom {
    pub atom_ref: String,
    pub kind: LegalAtomKind,
    pub proposition_ref: String,
    pub source_refs: BTreeSet<String>,
    pub reviewed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateRouteStatus {
    MissingAtoms,
    ReachableCandidate,
    Contested,
    Defeated,
    AuthorityBlocked,
    ExplicitlyUnresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateProofRoute {
    pub route_ref: String,
    pub target_proposition_ref: String,
    pub required_atom_refs: BTreeSet<String>,
    pub active_defeater_refs: BTreeSet<String>,
    pub counter_defeater_refs: BTreeSet<String>,
    pub status: CandidateRouteStatus,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdversarialSearchRole {
    Support,
    Defeater,
    CounterDefeater,
    Comparator,
    Contradiction,
    AuthorityTreatment,
    TerminologyExpansion,
    WrongTypeDiscriminator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdversarialSearchDemand {
    pub demand_ref: String,
    pub role: AdversarialSearchRole,
    pub target_route_ref: String,
    pub target_atom_or_residual_ref: String,
    pub hypothesis: SearchHypothesis,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedDefeatEdge {
    pub edge_ref: String,
    pub route_ref: String,
    pub defeater_atom_ref: String,
    pub citation_use: Option<CitationUse>,
    pub reasoning_role: ReasoningRole,
    pub review_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedCounterDefeatEdge {
    pub edge_ref: String,
    pub route_ref: String,
    pub defeats_defeater_atom_ref: String,
    pub counter_defeater_atom_ref: String,
    pub review_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversarialProofGraph {
    pub atoms: BTreeMap<String, LegalAtom>,
    pub routes: BTreeMap<String, CandidateProofRoute>,
    pub reviewed_defeats: BTreeMap<String, ReviewedDefeatEdge>,
    pub reviewed_counter_defeats: BTreeMap<String, ReviewedCounterDefeatEdge>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl AdversarialProofGraph {
    pub fn new() -> Self {
        Self {
            atoms: BTreeMap::new(),
            routes: BTreeMap::new(),
            reviewed_defeats: BTreeMap::new(),
            reviewed_counter_defeats: BTreeMap::new(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    pub fn admit_atom(&mut self, atom: LegalAtom) -> Result<bool, String> {
        if atom.atom_ref.trim().is_empty()
            || atom.proposition_ref.trim().is_empty()
            || !atom.candidate_only
            || atom.creates_semantic_authority
            || atom.creates_claim_truth
        {
            return Err("legal atom crossed proof-graph admission boundary".into());
        }
        if let Some(existing) = self.atoms.get(&atom.atom_ref) {
            if existing != &atom {
                return Err(format!("legal atom {} conflicts with existing atom", atom.atom_ref));
            }
            return Ok(false);
        }
        self.atoms.insert(atom.atom_ref.clone(), atom);
        Ok(true)
    }

    pub fn admit_route(&mut self, route: CandidateProofRoute) -> Result<bool, String> {
        if route.route_ref.trim().is_empty()
            || route.target_proposition_ref.trim().is_empty()
            || !route.candidate_only
            || route.creates_semantic_authority
            || route.creates_claim_truth
        {
            return Err("proof route crossed non-promotion boundary".into());
        }
        if let Some(existing) = self.routes.get(&route.route_ref) {
            if existing != &route {
                return Err(format!("proof route {} conflicts with existing route", route.route_ref));
            }
            return Ok(false);
        }
        self.routes.insert(route.route_ref.clone(), route);
        Ok(true)
    }

    pub fn apply_reviewed_defeat(&mut self, edge: ReviewedDefeatEdge) -> Result<(), String> {
        if edge.review_ref.trim().is_empty()
            || !edge.candidate_only
            || edge.creates_semantic_authority
            || edge.creates_claim_truth
            || edge.reasoning_role != ReasoningRole::Exception
                && edge.reasoning_role != ReasoningRole::Distinction
                && edge.reasoning_role != ReasoningRole::Policy
                && edge.reasoning_role != ReasoningRole::Burden
                && edge.reasoning_role != ReasoningRole::Unresolved
        {
            return Err("reviewed defeat edge crossed admission boundary".into());
        }
        let atom = self
            .atoms
            .get(&edge.defeater_atom_ref)
            .ok_or_else(|| "reviewed defeat references unknown defeater atom".to_string())?;
        if atom.kind != LegalAtomKind::Defeater || !atom.reviewed {
            return Err("reviewed defeat requires a reviewed Defeater atom".into());
        }
        let route = self
            .routes
            .get_mut(&edge.route_ref)
            .ok_or_else(|| "reviewed defeat references unknown proof route".to_string())?;
        route.active_defeater_refs.insert(edge.defeater_atom_ref.clone());
        route.status = CandidateRouteStatus::Defeated;
        self.reviewed_defeats.insert(edge.edge_ref.clone(), edge);
        Ok(())
    }

    pub fn apply_reviewed_counter_defeat(
        &mut self,
        edge: ReviewedCounterDefeatEdge,
    ) -> Result<(), String> {
        if edge.review_ref.trim().is_empty()
            || !edge.candidate_only
            || edge.creates_semantic_authority
            || edge.creates_claim_truth
        {
            return Err("reviewed counter-defeat crossed admission boundary".into());
        }
        let counter = self
            .atoms
            .get(&edge.counter_defeater_atom_ref)
            .ok_or_else(|| "counter-defeat references unknown atom".to_string())?;
        if counter.kind != LegalAtomKind::CounterDefeater || !counter.reviewed {
            return Err("counter-defeat requires reviewed CounterDefeater atom".into());
        }
        let route = self
            .routes
            .get_mut(&edge.route_ref)
            .ok_or_else(|| "counter-defeat references unknown route".to_string())?;
        if !route.active_defeater_refs.contains(&edge.defeats_defeater_atom_ref) {
            return Err("counter-defeat target is not an active defeater on route".into());
        }
        route
            .active_defeater_refs
            .remove(&edge.defeats_defeater_atom_ref);
        route
            .counter_defeater_refs
            .insert(edge.counter_defeater_atom_ref.clone());
        route.status = if route.active_defeater_refs.is_empty() {
            CandidateRouteStatus::ReachableCandidate
        } else {
            CandidateRouteStatus::Defeated
        };
        self.reviewed_counter_defeats
            .insert(edge.edge_ref.clone(), edge);
        Ok(())
    }
}

fn role_for_hypothesis(kind: SearchHypothesisKind) -> AdversarialSearchRole {
    match kind {
        SearchHypothesisKind::Support => AdversarialSearchRole::Support,
        SearchHypothesisKind::Defeater => AdversarialSearchRole::Defeater,
        SearchHypothesisKind::Comparator => AdversarialSearchRole::Comparator,
        SearchHypothesisKind::Contradiction => AdversarialSearchRole::Contradiction,
        SearchHypothesisKind::AuthorityTreatment => AdversarialSearchRole::AuthorityTreatment,
        SearchHypothesisKind::TerminologyExpansion => AdversarialSearchRole::TerminologyExpansion,
    }
}

pub fn compile_frontier_adversarial_search(
    frontier: &ProofFrontier,
    graph: &AdversarialProofGraph,
) -> Vec<AdversarialSearchDemand> {
    let mut demands = Vec::new();

    for residual in &frontier.residuals {
        match residual.status {
            ResidualStatus::Open => {
                demands.extend(family_for_residual(residual).into_iter().map(|hypothesis| {
                    AdversarialSearchDemand {
                        demand_ref: format!("adversarial:{}", hypothesis.hypothesis_ref),
                        role: role_for_hypothesis(hypothesis.kind),
                        target_route_ref: residual.proposition_ref.clone(),
                        target_atom_or_residual_ref: residual.residual_ref.clone(),
                        hypothesis,
                        candidate_only: true,
                        creates_semantic_authority: false,
                        creates_claim_truth: false,
                    }
                }));
            }
            ResidualStatus::Contested => {
                // A contested residual does not merely ask for more support.
                // Reuse a support-shaped provider query, but type the operation
                // as counter-defeater/discriminator search.
                let support = family_for_residual(&ProofResidual {
                    status: ResidualStatus::Open,
                    ..residual.clone()
                })
                .into_iter()
                .find(|hypothesis| hypothesis.kind == SearchHypothesisKind::Support);

                if let Some(hypothesis) = support {
                    demands.push(AdversarialSearchDemand {
                        demand_ref: format!(
                            "adversarial:{}:counter-defeater",
                            residual.residual_ref
                        ),
                        role: AdversarialSearchRole::CounterDefeater,
                        target_route_ref: residual.proposition_ref.clone(),
                        target_atom_or_residual_ref: residual.residual_ref.clone(),
                        hypothesis,
                        candidate_only: true,
                        creates_semantic_authority: false,
                        creates_claim_truth: false,
                    });
                }
            }
            ResidualStatus::AuthorityBlocked => {
                let treatment = family_for_residual(&ProofResidual {
                    status: ResidualStatus::Open,
                    ..residual.clone()
                })
                .into_iter()
                .find(|hypothesis| hypothesis.kind == SearchHypothesisKind::AuthorityTreatment);
                if let Some(hypothesis) = treatment {
                    demands.push(AdversarialSearchDemand {
                        demand_ref: format!(
                            "adversarial:{}:authority-treatment",
                            residual.residual_ref
                        ),
                        role: AdversarialSearchRole::AuthorityTreatment,
                        target_route_ref: residual.proposition_ref.clone(),
                        target_atom_or_residual_ref: residual.residual_ref.clone(),
                        hypothesis,
                        candidate_only: true,
                        creates_semantic_authority: false,
                        creates_claim_truth: false,
                    });
                }
            }
            ResidualStatus::Underidentified => {
                let terminology = family_for_residual(&ProofResidual {
                    status: ResidualStatus::Open,
                    ..residual.clone()
                })
                .into_iter()
                .find(|hypothesis| hypothesis.kind == SearchHypothesisKind::TerminologyExpansion);
                if let Some(hypothesis) = terminology {
                    demands.push(AdversarialSearchDemand {
                        demand_ref: format!(
                            "adversarial:{}:wrongtype-discriminator",
                            residual.residual_ref
                        ),
                        role: AdversarialSearchRole::WrongTypeDiscriminator,
                        target_route_ref: residual.proposition_ref.clone(),
                        target_atom_or_residual_ref: residual.residual_ref.clone(),
                        hypothesis,
                        candidate_only: true,
                        creates_semantic_authority: false,
                        creates_claim_truth: false,
                    });
                }
            }
            ResidualStatus::SatisfiedCandidate => {}
        }
    }

    // Even a currently reachable candidate route still gets defeater search.
    // Support saturation never licenses us to skip the adversarial half.
    for route in graph.routes.values() {
        if route.status == CandidateRouteStatus::ReachableCandidate {
            let synthetic = ProofResidual {
                residual_ref: format!("route-defeater:{}", route.route_ref),
                proposition_ref: route.target_proposition_ref.clone(),
                producer_class_ref: "producer:defeater".into(),
                jurisdiction_ref: None,
                authority_requirement_ref: None,
                salience: 100,
                dependency_refs: route.required_atom_refs.iter().cloned().collect(),
                status: ResidualStatus::Open,
            };
            if let Some(hypothesis) = family_for_residual(&synthetic)
                .into_iter()
                .find(|hypothesis| hypothesis.kind == SearchHypothesisKind::Defeater)
            {
                demands.push(AdversarialSearchDemand {
                    demand_ref: format!("adversarial:{}:defeater", route.route_ref),
                    role: AdversarialSearchRole::Defeater,
                    target_route_ref: route.route_ref.clone(),
                    target_atom_or_residual_ref: route.route_ref.clone(),
                    hypothesis,
                    candidate_only: true,
                    creates_semantic_authority: false,
                    creates_claim_truth: false,
                });
            }
        }
    }

    demands.sort_by(|left, right| left.demand_ref.cmp(&right.demand_ref));
    demands.dedup_by(|left, right| left.demand_ref == right.demand_ref);
    demands
}

pub fn candidate_route_from_atoms(
    route_ref: impl Into<String>,
    target_proposition_ref: impl Into<String>,
    required_atom_refs: BTreeSet<String>,
    graph: &AdversarialProofGraph,
) -> Result<CandidateProofRoute, String> {
    let route_ref = route_ref.into();
    let target_proposition_ref = target_proposition_ref.into();
    if route_ref.trim().is_empty() || target_proposition_ref.trim().is_empty() {
        return Err("candidate proof route requires refs".into());
    }
    let missing = required_atom_refs
        .iter()
        .filter(|atom_ref| {
            graph
                .atoms
                .get(*atom_ref)
                .is_none_or(|atom| !atom.reviewed)
        })
        .count();

    Ok(CandidateProofRoute {
        route_ref,
        target_proposition_ref,
        required_atom_refs,
        active_defeater_refs: BTreeSet::new(),
        counter_defeater_refs: BTreeSet::new(),
        status: if missing == 0 {
            CandidateRouteStatus::ReachableCandidate
        } else {
            CandidateRouteStatus::MissingAtoms
        },
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(reference: &str, kind: LegalAtomKind) -> LegalAtom {
        LegalAtom {
            atom_ref: reference.into(),
            kind,
            proposition_ref: format!("prop:{reference}"),
            source_refs: BTreeSet::from(["source:reviewed".into()]),
            reviewed: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    fn empty_frontier() -> ProofFrontier {
        ProofFrontier {
            consumer_ref: "consumer:pabai".into(),
            frontier_ref: "frontier:pabai".into(),
            residuals: vec![],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec![],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        }
    }

    #[test]
    fn reachable_route_still_generates_defeater_search() {
        let mut graph = AdversarialProofGraph::new();
        graph.admit_atom(atom("foreseeability", LegalAtomKind::Element)).unwrap();
        graph.admit_atom(atom("knowledge", LegalAtomKind::Element)).unwrap();
        let route = candidate_route_from_atoms(
            "route:pabai-duty",
            "proposition:duty",
            BTreeSet::from(["foreseeability".into(), "knowledge".into()]),
            &graph,
        )
        .unwrap();
        assert_eq!(route.status, CandidateRouteStatus::ReachableCandidate);
        graph.admit_route(route).unwrap();

        let demands = compile_frontier_adversarial_search(&empty_frontier(), &graph);
        assert!(demands
            .iter()
            .any(|demand| demand.role == AdversarialSearchRole::Defeater));
    }

    #[test]
    fn reviewed_defeater_then_reviewed_counter_defeater_reopens_candidate_route() {
        let mut graph = AdversarialProofGraph::new();
        graph.admit_atom(atom("premise", LegalAtomKind::Element)).unwrap();
        graph
            .admit_atom(atom("core-policy", LegalAtomKind::Defeater))
            .unwrap();
        graph
            .admit_atom(atom("policy-distinction", LegalAtomKind::CounterDefeater))
            .unwrap();
        graph
            .admit_route(
                candidate_route_from_atoms(
                    "route:pabai-duty",
                    "proposition:duty",
                    BTreeSet::from(["premise".into()]),
                    &graph,
                )
                .unwrap(),
            )
            .unwrap();

        graph
            .apply_reviewed_defeat(ReviewedDefeatEdge {
                edge_ref: "defeat:pabai:policy".into(),
                route_ref: "route:pabai-duty".into(),
                defeater_atom_ref: "core-policy".into(),
                citation_use: Some(CitationUse::Applied),
                reasoning_role: ReasoningRole::Policy,
                review_ref: "review:defeater".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            })
            .unwrap();
        assert_eq!(
            graph.routes["route:pabai-duty"].status,
            CandidateRouteStatus::Defeated
        );

        graph
            .apply_reviewed_counter_defeat(ReviewedCounterDefeatEdge {
                edge_ref: "counter:pabai:policy-distinction".into(),
                route_ref: "route:pabai-duty".into(),
                defeats_defeater_atom_ref: "core-policy".into(),
                counter_defeater_atom_ref: "policy-distinction".into(),
                review_ref: "review:counter-defeater".into(),
                candidate_only: true,
                creates_semantic_authority: false,
                creates_claim_truth: false,
            })
            .unwrap();
        assert_eq!(
            graph.routes["route:pabai-duty"].status,
            CandidateRouteStatus::ReachableCandidate
        );

        // Reopened does not mean safe from a second adversarial pass.
        let demands = compile_frontier_adversarial_search(&empty_frontier(), &graph);
        assert!(demands
            .iter()
            .any(|demand| demand.role == AdversarialSearchRole::Defeater));
    }

    #[test]
    fn contested_frontier_generates_counter_defeater_not_more_support_only() {
        let graph = AdversarialProofGraph::new();
        let frontier = ProofFrontier {
            consumer_ref: "consumer:test".into(),
            frontier_ref: "frontier:test".into(),
            residuals: vec![ProofResidual {
                residual_ref: "residual:contested".into(),
                proposition_ref: "proposition:claim".into(),
                producer_class_ref: "producer:law".into(),
                jurisdiction_ref: Some("AU".into()),
                authority_requirement_ref: None,
                salience: 10,
                dependency_refs: vec![],
                status: ResidualStatus::Contested,
            }],
            satisfied_payment_refs: vec![],
            contested_coordinate_refs: vec!["residual:contested".into()],
            authority_blocked_refs: vec![],
            authority: "experimental_candidate_only",
        };
        let demands = compile_frontier_adversarial_search(&frontier, &graph);
        assert_eq!(demands.len(), 1);
        assert_eq!(demands[0].role, AdversarialSearchRole::CounterDefeater);
    }
}
