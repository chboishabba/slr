use sensiblaw_core::canonical_evidence::{
    ConsumerAdequacyReceipt, ConsumerCounterexampleReceipt, ExpressiveAsymmetryReceipt,
    PerturbationExpectation, RepresentationParityReceipt, SemanticPerturbationReceipt,
};

#[test]
fn parity_requires_admissible_and_consumer_adequate_both_sides() {
    let left = ConsumerAdequacyReceipt::new(
        "repr:simple",
        "consumer:meaning",
        "queries:meaning-v1",
        "admission:left",
        ["factor:left:q1"],
        true,
        true,
        Some(1),
    )
    .unwrap();
    let right = ConsumerAdequacyReceipt::new(
        "repr:full",
        "consumer:meaning",
        "queries:meaning-v1",
        "admission:right",
        ["factor:right:q1"],
        true,
        true,
        Some(5),
    )
    .unwrap();

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
