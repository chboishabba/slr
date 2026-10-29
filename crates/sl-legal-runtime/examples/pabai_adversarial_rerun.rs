//! S20 executable Pabai adversarial rerun.
//!
//! This is a runtime parity specimen for the already-formalised Pabai
//! defeater/refinement loop. It does not determine Pabai's legal outcome.
//!
//! Run:
//!   cargo run -p sensiblaw-legal-runtime --example pabai_adversarial_rerun

use std::collections::BTreeSet;

use sensiblaw_legal_runtime::{
    candidate_route_from_atoms, compile_frontier_adversarial_search,
    AdversarialProofGraph, AdversarialSearchRole, CandidateRouteStatus,
    LegalAtom, LegalAtomKind, ReviewedCounterDefeatEdge, ReviewedDefeatEdge,
};
use sensiblaw_proof_search_loop::{
    frontier::ProofFrontier,
    reasoning::{CitationUse, ReasoningRole},
};

fn atom(
    atom_ref: &str,
    kind: LegalAtomKind,
    proposition_ref: &str,
    source_ref: &str,
) -> LegalAtom {
    LegalAtom {
        atom_ref: atom_ref.into(),
        kind,
        proposition_ref: proposition_ref.into(),
        source_refs: BTreeSet::from([source_ref.into()]),
        reviewed: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn empty_pabai_frontier() -> ProofFrontier {
    ProofFrontier {
        consumer_ref: "consumer:pabai-climate-duty".into(),
        frontier_ref: "frontier:pabai:adversarial-runtime".into(),
        residuals: vec![],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut graph = AdversarialProofGraph::new();

    for atom in [
        atom(
            "atom:pabai:foreseeability",
            LegalAtomKind::Element,
            "proposition:pabai:foreseeability",
            "case:au:fca:2025:796",
        ),
        atom(
            "atom:pabai:knowledge",
            LegalAtomKind::Element,
            "proposition:pabai:knowledge",
            "case:au:fca:2025:796",
        ),
        atom(
            "atom:pabai:control",
            LegalAtomKind::Element,
            "proposition:pabai:control",
            "case:au:fca:2025:796",
        ),
        atom(
            "atom:pabai:core-policy",
            LegalAtomKind::Defeater,
            "proposition:pabai:core-government-policy-blocker",
            "case:au:fca:2025:796",
        ),
        atom(
            "atom:pabai:narrower-policy-distinction",
            LegalAtomKind::CounterDefeater,
            "proposition:pabai:narrower-duty-policy-distinction",
            "reviewed:counterfactual-repair-candidate",
        ),
    ] {
        graph.admit_atom(atom)?;
    }

    let route = candidate_route_from_atoms(
        "route:pabai:candidate-duty",
        "proposition:pabai:candidate-duty",
        BTreeSet::from([
            "atom:pabai:foreseeability".into(),
            "atom:pabai:knowledge".into(),
            "atom:pabai:control".into(),
        ]),
        &graph,
    )?;
    graph.admit_route(route)?;

    let before = graph
        .routes
        .get("route:pabai:candidate-duty")
        .expect("route just admitted")
        .status;
    let before_search =
        compile_frontier_adversarial_search(&empty_pabai_frontier(), &graph);

    graph.apply_reviewed_defeat(ReviewedDefeatEdge {
        edge_ref: "defeat:pabai:core-policy".into(),
        route_ref: "route:pabai:candidate-duty".into(),
        defeater_atom_ref: "atom:pabai:core-policy".into(),
        citation_use: Some(CitationUse::Applied),
        reasoning_role: ReasoningRole::Policy,
        review_ref: "review:pabai:core-policy-defeater".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })?;

    let after_defeat = graph
        .routes
        .get("route:pabai:candidate-duty")
        .expect("route retained")
        .status;

    graph.apply_reviewed_counter_defeat(ReviewedCounterDefeatEdge {
        edge_ref: "counterdefeat:pabai:narrower-policy-distinction".into(),
        route_ref: "route:pabai:candidate-duty".into(),
        defeats_defeater_atom_ref: "atom:pabai:core-policy".into(),
        counter_defeater_atom_ref: "atom:pabai:narrower-policy-distinction".into(),
        review_ref: "review:pabai:narrower-policy-distinction".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })?;

    let after_counter_defeat = graph
        .routes
        .get("route:pabai:candidate-duty")
        .expect("route retained")
        .status;
    let rerun_search =
        compile_frontier_adversarial_search(&empty_pabai_frontier(), &graph);

    println!("consumer_ref=consumer:pabai-climate-duty");
    println!("route_ref=route:pabai:candidate-duty");
    println!("before={before:?}");
    println!(
        "before_defeater_search={}",
        before_search
            .iter()
            .any(|demand| demand.role == AdversarialSearchRole::Defeater)
    );
    println!("after_reviewed_defeater={after_defeat:?}");
    println!("after_reviewed_counter_defeater={after_counter_defeat:?}");
    println!(
        "rerun_defeater_search={}",
        rerun_search
            .iter()
            .any(|demand| demand.role == AdversarialSearchRole::Defeater)
    );
    println!("candidate_only={}", graph.candidate_only);
    println!(
        "creates_semantic_authority={}",
        graph.creates_semantic_authority
    );
    println!("creates_claim_truth={}", graph.creates_claim_truth);

    assert_eq!(before, CandidateRouteStatus::ReachableCandidate);
    assert!(before_search
        .iter()
        .any(|demand| demand.role == AdversarialSearchRole::Defeater));
    assert_eq!(after_defeat, CandidateRouteStatus::Defeated);
    assert_eq!(
        after_counter_defeat,
        CandidateRouteStatus::ReachableCandidate
    );
    assert!(rerun_search
        .iter()
        .any(|demand| demand.role == AdversarialSearchRole::Defeater));
    assert!(!graph.creates_semantic_authority);
    assert!(!graph.creates_claim_truth);

    Ok(())
}
