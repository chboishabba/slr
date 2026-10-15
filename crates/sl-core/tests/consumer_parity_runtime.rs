use sensiblaw_core::canonical_evidence::{
    ConsumerAdequacyReceipt, ConsumerCounterexampleReceipt, ConsumerInformativenessReceipt,
    DiscriminationHandoffReceipt, ExpressiveAsymmetryReceipt, PerturbationExpectation,
    RepresentationDominanceReceipt, RepresentationParityReceipt, SemanticPerturbationReceipt,
};

fn adequacy(
    representation_ref: &str,
    consumer_ref: &str,
    query_family_ref: &str,
    description_length_rank: u64,
) -> ConsumerAdequacyReceipt {
    ConsumerAdequacyReceipt::new(
        representation_ref,
        consumer_ref,
        query_family_ref,
        format!("admission:{representation_ref}"),
        [format!("factor:{representation_ref}:q1")],
        true,
        true,
        Some(description_length_rank),
    )
    .unwrap()
}

#[test]
fn parity_requires_admissible_and_consumer_adequate_both_sides() {
    let left = adequacy("repr:simple", "consumer:meaning", "queries:meaning-v1", 1);
    let right = adequacy("repr:full", "consumer:meaning", "queries:meaning-v1", 5);

    let parity = RepresentationParityReceipt::new(left, right, "parity:reviewed").unwrap();
    assert!(parity.left.eligible());
    assert!(parity.right.eligible());
}

#[test]
fn expressive_complexity_is_not_semantic_or_evidential_authority() {
    let receipt = ExpressiveAsymmetryReceipt::new(
        "source:short",
        "source:elaborate",
        "semantic:shared",
        "complexity:low",
        "complexity:high",
        "factor:q1",
    )
    .unwrap();

    assert!(!receipt.creates_semantic_authority);
    assert!(!receipt.creates_evidential_authority);
    assert!(!receipt.promotes_claim_truth);
}

#[test]
fn collapsed_consumer_distinction_creates_residual_not_truth() {
    let failure = ConsumerCounterexampleReceipt::new(
        "repr:coarse",
        "consumer:disjointness",
        "query:disjointness",
        "state:left",
        "state:right",
        "outcome:yes",
        "outcome:no",
        "residual:separate-axis",
    )
    .unwrap();

    assert!(!failure.candidate_consumer_adequate);
    assert!(!failure.promotes_claim_truth);
    assert!(!failure.refutes_source_truth);
}

#[test]
fn perturbation_expectation_distinguishes_invariant_from_changed_semantics() {
    let invariant = SemanticPerturbationReceipt::new(
        "perturb:paraphrase",
        PerturbationExpectation::SemanticPreserving,
        "consumer:meaning",
        "before:q1",
        "before:q1",
        "factor:q1",
    )
    .unwrap();
    assert!(invariant.expectation_satisfied());

    let changing = SemanticPerturbationReceipt::new(
        "perturb:fact-flip",
        PerturbationExpectation::SemanticChanging,
        "consumer:meaning",
        "before:q1",
        "after:q1",
        "factor:q1",
    )
    .unwrap();
    assert!(changing.expectation_satisfied());
}

#[test]
fn informativeness_is_query_indexed_and_allows_simple_surface_parity() {
    let simple = adequacy("repr:simple", "consumer:meaning", "queries:meaning-v1", 1);
    let full = adequacy("repr:full", "consumer:meaning", "queries:meaning-v1", 5);

    let receipt = ConsumerInformativenessReceipt::new(
        simple,
        full,
        ["distinction:direct-vs-preservation", "distinction:meaning-vs-application"],
        ["distinction:direct-vs-preservation", "distinction:meaning-vs-application"],
        "review:factorisation:simple-full",
    )
    .unwrap();

    assert!(receipt.left_at_least_as_informative());
    assert!(!receipt.creates_universal_superiority);
    assert!(!receipt.promotes_claim_truth);
}

#[test]
fn informativeness_rejects_mismatched_query_family() {
    let simple = adequacy("repr:simple", "consumer:meaning", "queries:meaning-v1", 1);
    let full = adequacy("repr:full", "consumer:meaning", "queries:quotation-v1", 5);

    let error = ConsumerInformativenessReceipt::new(
        simple,
        full,
        ["distinction:direct-vs-preservation"],
        ["distinction:direct-vs-preservation"],
        "review:factorisation:mismatch",
    )
    .unwrap_err();

    assert_eq!(error.to_string(), "query family mismatch");
}

#[test]
fn simpler_surface_can_strictly_dominate_without_becoming_universally_superior() {
    let simple = adequacy("repr:simple", "consumer:meaning", "queries:meaning-v1", 1);
    let elaborate = adequacy("repr:elaborate", "consumer:meaning", "queries:meaning-v1", 99);

    let informativeness = ConsumerInformativenessReceipt::new(
        simple,
        elaborate,
        ["distinction:purpose"],
        ["distinction:purpose", "distinction:direct-vs-preservation"],
        "review:factorisation:simple-elaborate",
    )
    .unwrap();
    let dominance = RepresentationDominanceReceipt::new(
        informativeness,
        "distinction:direct-vs-preservation",
        "receipt:simple:separator",
        "receipt:elaborate:separator-missing",
    )
    .unwrap();

    assert!(dominance.is_strict_for_declared_consumer());
    assert!(!dominance.creates_universal_superiority);
    assert!(!dominance.complexity_rank_creates_authority);
    assert!(!dominance.promotes_claim_truth);
}

#[test]
fn experiment_and_pareto_handoff_do_not_resolve_or_promote() {
    let handoff = DiscriminationHandoffReceipt::new(
        "consumer:meaning",
        "queries:meaning-v1",
        "residual:wide-vs-direct",
        "experiment:discriminator:wide-vs-direct",
        "investigation:obligation:primary-authority",
        "pareto:frontier:five-axis",
        "legal-follow:intent:smith",
        "source-role:primary-law",
        "jurisdiction:au-qld-hca",
    )
    .unwrap();

    assert!(!handoff.experimental_design_creates_truth);
    assert!(!handoff.pareto_priority_creates_truth);
    assert!(!handoff.creates_access_authority);
    assert!(!handoff.creates_legal_authority);
    assert!(!handoff.resolves_residual);
}
