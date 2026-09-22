//! Generic S20.10 reviewed-treatment -> adversarial-proof-graph compiler.
//!
//! The compiler is intentionally matter-agnostic.  A reviewed party treatment
//! remains a candidate-only argument coordinate; it is not upgraded from
//! submission/citation into a holding or claim truth.
//!
//! Counter-defeaters are matched to active defeaters on the same exact
//! treatment coordinate.  This is the key non-collapse rule used by live case
//! adapters: a distinction aimed at one authority treatment cannot silently
//! discharge a different defeater merely because both concern the same case.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_proof_search_loop::{
    frontier::{ProofFrontier, ProofResidual, ResidualStatus},
    reasoning::{CitationUse, ReasoningRole},
};

use crate::{
    candidate_route_from_atoms, compile_frontier_adversarial_search,
    AdversarialProofGraph, AdversarialSearchDemand, CandidateRouteStatus,
    LegalAtom, LegalAtomKind, ReviewedCounterDefeatEdge, ReviewedDefeatEdge,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReviewedTreatmentRole {
    Support,
    Defeater,
    CounterDefeater,
    AuthorityScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewedTreatmentCoordinate {
    pub treatment_ref: String,
    pub coordinate_ref: String,
    pub source_ref: String,
    pub proposition_ref: String,
    pub cited_authority_ref: String,
    pub role: ReviewedTreatmentRole,
    pub reviewed: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl ReviewedTreatmentCoordinate {
    pub fn validate(&self) -> Result<(), String> {
        if self.treatment_ref.trim().is_empty()
            || self.coordinate_ref.trim().is_empty()
            || self.source_ref.trim().is_empty()
            || self.proposition_ref.trim().is_empty()
            || self.cited_authority_ref.trim().is_empty()
        {
            return Err("reviewed treatment coordinate is incomplete".into());
        }
        if !self.reviewed
            || !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("reviewed treatment crossed review/non-promotion boundary".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TypedRerunGapKind {
    MissingSupport,
    ActiveDefeater,
    AuthorityScope,
    WrongType,
    Fact,
    Applicability,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedRerunGap {
    pub gap_ref: String,
    pub kind: TypedRerunGapKind,
    pub coordinate_ref: String,
    pub proposition_ref: String,
    pub source_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdversarialRerunReceipt {
    pub consumer_ref: String,
    pub route_ref: String,
    pub graph: AdversarialProofGraph,
    pub paid_atom_refs: BTreeSet<String>,
    pub active_defeater_refs: BTreeSet<String>,
    pub counter_defeater_refs: BTreeSet<String>,
    pub typed_gaps: Vec<TypedRerunGap>,
    pub next_demands: Vec<AdversarialSearchDemand>,
    pub route_status: CandidateRouteStatus,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn atom_ref(treatment: &ReviewedTreatmentCoordinate) -> String {
    format!("atom:treatment:{}", treatment.treatment_ref)
}

fn legal_atom_kind(role: ReviewedTreatmentRole) -> LegalAtomKind {
    match role {
        ReviewedTreatmentRole::Support => LegalAtomKind::Authority,
        ReviewedTreatmentRole::Defeater | ReviewedTreatmentRole::AuthorityScope => {
            LegalAtomKind::Defeater
        }
        ReviewedTreatmentRole::CounterDefeater => LegalAtomKind::CounterDefeater,
    }
}

fn legal_atom(treatment: &ReviewedTreatmentCoordinate) -> LegalAtom {
    LegalAtom {
        atom_ref: atom_ref(treatment),
        kind: legal_atom_kind(treatment.role),
        proposition_ref: treatment.proposition_ref.clone(),
        source_refs: BTreeSet::from([
            treatment.source_ref.clone(),
            treatment.cited_authority_ref.clone(),
            treatment.coordinate_ref.clone(),
        ]),
        reviewed: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn role_reasoning(role: ReviewedTreatmentRole) -> ReasoningRole {
    match role {
        ReviewedTreatmentRole::AuthorityScope => ReasoningRole::Unresolved,
        ReviewedTreatmentRole::Defeater => ReasoningRole::Unresolved,
        ReviewedTreatmentRole::CounterDefeater => ReasoningRole::Distinction,
        ReviewedTreatmentRole::Support => ReasoningRole::Premise,
    }
}

fn gap_frontier(
    consumer_ref: &str,
    route_ref: &str,
    active: &[(&ReviewedTreatmentCoordinate, String)],
) -> ProofFrontier {
    let residuals = active
        .iter()
        .map(|(treatment, atom_ref)| ProofResidual {
            residual_ref: format!("residual:active-defeater:{atom_ref}"),
            proposition_ref: route_ref.to_owned(),
            producer_class_ref: match treatment.role {
                ReviewedTreatmentRole::AuthorityScope => "producer:authority-treatment",
                _ => "producer:counter-defeater",
            }
            .into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some(treatment.cited_authority_ref.clone()),
            salience: 100,
            dependency_refs: vec![treatment.coordinate_ref.clone()],
            status: ResidualStatus::Contested,
        })
        .collect();

    ProofFrontier {
        consumer_ref: consumer_ref.into(),
        frontier_ref: format!("frontier:{route_ref}:adversarial-rerun"),
        residuals,
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: active
            .iter()
            .map(|(treatment, _)| treatment.coordinate_ref.clone())
            .collect(),
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

pub fn compile_reviewed_treatment_rerun(
    consumer_ref: &str,
    route_ref: &str,
    target_proposition_ref: &str,
    treatments: &[ReviewedTreatmentCoordinate],
) -> Result<AdversarialRerunReceipt, String> {
    if consumer_ref.trim().is_empty()
        || route_ref.trim().is_empty()
        || target_proposition_ref.trim().is_empty()
    {
        return Err("adversarial rerun requires consumer/route/target refs".into());
    }

    for treatment in treatments {
        treatment.validate()?;
    }

    let mut graph = AdversarialProofGraph::new();
    for treatment in treatments {
        graph.admit_atom(legal_atom(treatment))?;
    }

    let support_atom_refs = treatments
        .iter()
        .filter(|t| t.role == ReviewedTreatmentRole::Support)
        .map(atom_ref)
        .collect::<BTreeSet<_>>();
    if support_atom_refs.is_empty() {
        return Err("candidate route has no reviewed support treatment".into());
    }

    let route = candidate_route_from_atoms(
        route_ref,
        target_proposition_ref,
        support_atom_refs.clone(),
        &graph,
    )?;
    graph.admit_route(route)?;

    let mut defeaters_by_coordinate: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for treatment in treatments.iter().filter(|t| {
        matches!(
            t.role,
            ReviewedTreatmentRole::Defeater | ReviewedTreatmentRole::AuthorityScope
        )
    }) {
        let defeat_atom_ref = atom_ref(treatment);
        graph.apply_reviewed_defeat(ReviewedDefeatEdge {
            edge_ref: format!("edge:defeat:{}", treatment.treatment_ref),
            route_ref: route_ref.into(),
            defeater_atom_ref: defeat_atom_ref.clone(),
            citation_use: Some(CitationUse::PartySubmission),
            reasoning_role: role_reasoning(treatment.role),
            review_ref: format!("review:treatment:{}", treatment.treatment_ref),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })?;
        defeaters_by_coordinate
            .entry(treatment.coordinate_ref.clone())
            .or_default()
            .push(defeat_atom_ref);
    }

    // Exact-coordinate counter-defeat only.  A counter-defeater does not erase
    // unrelated live objections elsewhere in the graph.
    for treatment in treatments
        .iter()
        .filter(|t| t.role == ReviewedTreatmentRole::CounterDefeater)
    {
        let Some(targets) = defeaters_by_coordinate.get(&treatment.coordinate_ref) else {
            continue;
        };
        // Deterministic selection is safe only when the coordinate has one
        // active defeater.  Multiple live defeaters remain explicit rather
        // than being guessed away.
        let active_targets = targets
            .iter()
            .filter(|target| {
                graph
                    .routes
                    .get(route_ref)
                    .map(|route| route.active_defeater_refs.contains(*target))
                    .unwrap_or(false)
            })
            .cloned()
            .collect::<Vec<_>>();
        if active_targets.len() != 1 {
            continue;
        }
        graph.apply_reviewed_counter_defeat(ReviewedCounterDefeatEdge {
            edge_ref: format!("edge:counter-defeat:{}", treatment.treatment_ref),
            route_ref: route_ref.into(),
            defeats_defeater_atom_ref: active_targets[0].clone(),
            counter_defeater_atom_ref: atom_ref(treatment),
            review_ref: format!("review:treatment:{}", treatment.treatment_ref),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        })?;
    }

    let route = graph
        .routes
        .get(route_ref)
        .ok_or_else(|| "compiled route disappeared".to_string())?;

    let treatment_by_atom = treatments
        .iter()
        .map(|t| (atom_ref(t), t))
        .collect::<BTreeMap<_, _>>();

    let active = route
        .active_defeater_refs
        .iter()
        .filter_map(|atom| treatment_by_atom.get(atom).map(|t| (*t, atom.clone())))
        .collect::<Vec<_>>();

    let typed_gaps = active
        .iter()
        .map(|(treatment, atom)| TypedRerunGap {
            gap_ref: format!("gap:{atom}"),
            kind: if treatment.role == ReviewedTreatmentRole::AuthorityScope {
                TypedRerunGapKind::Applicability
            } else {
                TypedRerunGapKind::ActiveDefeater
            },
            coordinate_ref: treatment.coordinate_ref.clone(),
            proposition_ref: treatment.proposition_ref.clone(),
            source_refs: BTreeSet::from([
                treatment.source_ref.clone(),
                treatment.cited_authority_ref.clone(),
            ]),
            candidate_only: true,
            creates_claim_truth: false,
        })
        .collect::<Vec<_>>();

    let frontier = gap_frontier(consumer_ref, route_ref, &active);
    let next_demands = compile_frontier_adversarial_search(&frontier, &graph);

    Ok(AdversarialRerunReceipt {
        consumer_ref: consumer_ref.into(),
        route_ref: route_ref.into(),
        graph,
        paid_atom_refs: support_atom_refs,
        active_defeater_refs: route.active_defeater_refs.clone(),
        counter_defeater_refs: route.counter_defeater_refs.clone(),
        typed_gaps,
        next_demands,
        route_status: route.status,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn treatment(
        name: &str,
        coordinate: &str,
        role: ReviewedTreatmentRole,
    ) -> ReviewedTreatmentCoordinate {
        ReviewedTreatmentCoordinate {
            treatment_ref: name.into(),
            coordinate_ref: coordinate.into(),
            source_ref: format!("source:{name}"),
            proposition_ref: format!("proposition:{name}"),
            cited_authority_ref: "case:authority".into(),
            role,
            reviewed: true,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn counter_defeater_only_pays_exact_single_coordinate_target() {
        let receipt = compile_reviewed_treatment_rerun(
            "consumer:test",
            "route:test",
            "proposition:target",
            &[
                treatment("support", "coordinate:support", ReviewedTreatmentRole::Support),
                treatment("d1", "coordinate:a", ReviewedTreatmentRole::Defeater),
                treatment("d2", "coordinate:b", ReviewedTreatmentRole::Defeater),
                treatment(
                    "counter-a",
                    "coordinate:a",
                    ReviewedTreatmentRole::CounterDefeater,
                ),
            ],
        )
        .unwrap();

        assert_eq!(receipt.route_status, CandidateRouteStatus::Defeated);
        assert_eq!(receipt.active_defeater_refs.len(), 1);
        assert_eq!(receipt.counter_defeater_refs.len(), 1);
        assert_eq!(receipt.typed_gaps.len(), 1);
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn ambiguous_same_coordinate_defeaters_are_not_guessed_away() {
        let receipt = compile_reviewed_treatment_rerun(
            "consumer:test",
            "route:test",
            "proposition:target",
            &[
                treatment("support", "coordinate:support", ReviewedTreatmentRole::Support),
                treatment("d1", "coordinate:a", ReviewedTreatmentRole::Defeater),
                treatment("d2", "coordinate:a", ReviewedTreatmentRole::AuthorityScope),
                treatment(
                    "counter-a",
                    "coordinate:a",
                    ReviewedTreatmentRole::CounterDefeater,
                ),
            ],
        )
        .unwrap();

        assert_eq!(receipt.route_status, CandidateRouteStatus::Defeated);
        assert_eq!(receipt.active_defeater_refs.len(), 2);
        assert!(receipt.counter_defeater_refs.is_empty());
    }
}
